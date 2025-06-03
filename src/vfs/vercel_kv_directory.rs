use crate::log_debug;
use crate::utils::string::{
    build_dir_entry_path, build_file_entry_path, normalize_dir_path, normalize_file_path,
};
use crate::vfs::vercel_kv_helpers::*;
use crate::vfs::vercel_kv_store::{create_directory_attributes, create_file_attributes};
use crate::vfs::vercel_kv_types::*;
use crate::vfs::vercel_kv_vfs::VercelKvVfs;
use crate::vfs::VfsError;
use crate::vfs::VirtualFileSystem;
use std::collections::{HashMap, HashSet};

impl VercelKvVfs {
    /// Read file or directory attributes
    pub(crate) async fn read_file_attributes_impl(
        &self,
        path: &str,
    ) -> Result<FileAttributes, VfsError> {
        let normalized_path = normalize_path(path);
        let is_dir = is_directory_path(&normalized_path); // Check if it looks like a directory path

        // --- Cache Check ---
        // Check memory metadata cache first
        if let Some(attrs) = self.store.read_from_metadata_cache(&normalized_path).await {
            // If it's a placeholder file with zero size, try to get actual size from GitHub
            if !attrs.is_directory && attrs.source_type == "placeholder" && attrs.size == 0 {
                if let Ok(github_result) = self.load_github_file_info_impl(&normalized_path).await {
                    let mut updated_attrs = attrs.clone();
                    updated_attrs.size = github_result.size;
                    updated_attrs.source_type = "cloud".to_string(); // Update status

                    // Update metadata cache
                    self.store
                        .write_to_metadata_cache(&normalized_path, updated_attrs.clone())
                        .await;
                    // Update KV store in background (write to parent dir metadata)
                    self.store.write_file_attributes_to_dir_kv_background(
                        normalized_path.clone(),
                        updated_attrs.clone(),
                    );
                    return Ok(updated_attrs);
                } else {
                    // If GitHub load fails, return the cached placeholder attributes
                    log::debug!(
                        "GitHub info lookup failed for placeholder '{}', returning cached attrs.",
                        normalized_path
                    );
                }
            }
            // Return cached attributes (could be file or directory)
            return Ok(attrs);
        }

        // --- KV Check (Directory Metadata) ---
        if !is_dir {
            // It's potentially a file, try reading its attributes from the parent directory's metadata
            match self
                .store
                .read_file_attributes_from_dir_kv(&normalized_path)
                .await
            {
                Ok(Some(attributes)) => {
                    // If it's a placeholder file with zero size, try to get actual size from GitHub
                    if attributes.source_type == "placeholder" && attributes.size == 0 {
                        if let Ok(github_result) =
                            self.load_github_file_info_impl(&normalized_path).await
                        {
                            let mut updated_attrs = attributes.clone();
                            updated_attrs.size = github_result.size;
                            updated_attrs.source_type = "cloud".to_string(); // Update status

                            // Update metadata cache
                            self.store
                                .write_to_metadata_cache(&normalized_path, updated_attrs.clone())
                                .await;
                            // Update KV store in background (write to parent dir metadata)
                            self.store.write_file_attributes_to_dir_kv_background(
                                normalized_path.clone(),
                                updated_attrs.clone(),
                            );
                            return Ok(updated_attrs);
                        } else {
                            log::debug!("GitHub info lookup failed for placeholder '{}', returning KV attrs.", normalized_path);
                        }
                    }
                    // Cache the attributes found in directory metadata
                    self.store
                        .write_to_metadata_cache(&normalized_path, attributes.clone())
                        .await;
                    return Ok(attributes);
                }
                Ok(None) => {
                    log::debug!(
                        "No attributes found for file '{}' in parent directory metadata.",
                        normalized_path
                    );
                    // No attributes found in parent dir metadata, continue checks...
                }
                Err(e) => {
                    log::error!(
                        "Failed to read attributes from directory KV for '{}': {:?}",
                        normalized_path,
                        e
                    );
                    // Continue checks, maybe it's a directory or exists implicitly
                }
            }
        }

        // --- Existence Check (KV Content/Marker) ---
        // Check if the path exists (either as content or directory marker)
        let exists = self.exists_impl(&normalized_path).await?;

        if exists {
            if is_dir {
                // It's a directory that exists (via marker). Return default directory attributes.
                log::debug!(
                    "Directory marker exists for '{}', returning default directory attributes.",
                    normalized_path
                );
                let attributes = create_directory_attributes(&normalized_path, "system"); // Pass path
                                                                                          // Cache these default directory attributes
                self.store
                    .write_to_metadata_cache(&normalized_path, attributes.clone())
                    .await;
                return Ok(attributes);
            } else {
                // It's a file that exists (has @@content key), but had no attributes in parent dir metadata.
                // This might happen if created outside the standard VFS write process or metadata failed.
                log::warn!("File content exists for '{}' but no attributes found in parent directory metadata. Creating default attributes.", normalized_path);

                // Try to get size information from GitHub first as it might be a cloud file without metadata entry
                if let Ok(github_result) = self.load_github_file_info_impl(&normalized_path).await {
                    let attributes =
                        create_file_attributes(&normalized_path, github_result.size, "cloud");
                    // Cache the attributes and write back to parent dir metadata
                    self.store
                        .write_to_metadata_cache(&normalized_path, attributes.clone())
                        .await;
                    self.store.write_file_attributes_to_dir_kv_background(
                        normalized_path.clone(),
                        attributes.clone(),
                    );
                    return Ok(attributes);
                }

                // Fallback: read content to get size (assume it's 'user' or 'unknown' source)
                return match self.store.read_from_kv(&normalized_path).await {
                    Ok(Some(content)) => {
                        let attributes =
                            create_file_attributes(&normalized_path, content.len(), "user");
                        // Cache the attributes and write back to parent dir metadata
                        self.store
                            .write_to_metadata_cache(&normalized_path, attributes.clone())
                            .await;
                        self.store.write_file_attributes_to_dir_kv_background(
                            normalized_path.clone(),
                            attributes.clone(),
                        );
                        Ok(attributes)
                    }
                    Ok(None) => {
                        // Content key exists but reading returns None? Should be rare.
                        log::error!(
                            "File content key exists for '{}' but read returned None.",
                            normalized_path
                        );
                        Err(VfsError::NotFound(format!(
                            "File content inconsistent for: {}",
                            normalized_path
                        )))
                    }
                    Err(e) => {
                        log::error!(
                            "Failed to read content for existing file '{}' to determine size: {:?}",
                            normalized_path,
                            e
                        );
                        Err(e) // Propagate the read error
                    }
                };
            }
        }

        // --- Not Found ---
        log::debug!(
            "Attributes not found and path does not exist: '{}'",
            normalized_path
        );
        Err(VfsError::NotFound(format!(
            "File or directory not found: {}",
            normalized_path
        )))
    }

    /// List directory contents by reading the directory's metadata object.
    /// Assumes the metadata object contains entries for both files and subdirectories.
    /// For the root directory, if metadata is initially missing/empty and GitHub load is allowed,
    /// it will trigger a GitHub load to populate the metadata before reading it.
    pub(crate) async fn list_directory_impl(
        &self,
        path: &str,
        _skip_github_load: bool, // Parameter kept for signature consistency
    ) -> Result<Vec<DirectoryEntry>, VfsError> {
        log_debug!("Listing directory using metadata: '{}'", path);
        let normalized_dir_path = normalize_path(path);
        let dir_path_for_lookup = normalize_dir_path(&normalized_dir_path);
        log_debug!(
            "Normalized directory path for lookup: '{}'",
            dir_path_for_lookup
        );

        // --- Trigger GitHub Load for Root if Allowed ---
        // If listing root and GitHub load is enabled, trigger it first.
        // We expect this load to populate the KV metadata for the root.
        if dir_path_for_lookup.is_empty() && !_skip_github_load {
            // Check if root metadata *already* exists and isn't empty before triggering load?
            // Let's check cache first. If cache exists and has items, maybe skip load.
            let cached_metadata_exists = self.store.exists_in_metadata_cache("").await; // Check cache for root
            let mut skip_github_due_to_cache = false;
            if cached_metadata_exists {
                if let Some(cached_attrs) = self.store.read_from_metadata_cache("").await {
                    // If it's cached, assume it's accurate for now, skip GitHub load.
                    // This check might need refinement depending on cache invalidation strategy.
                    log::debug!(
                        "Root directory metadata found in cache, skipping GitHub load trigger."
                    );
                    skip_github_due_to_cache = true;
                }
            }
            // Also check KV store directly? This might be redundant if cache reflects KV.
            // Let's stick with the cache check for now. If cache is empty/missing, proceed to check KV before load.
            if !skip_github_due_to_cache {
                match self.store.read_directory_metadata_from_kv("").await {
                    Ok(metadata) if !metadata.files.is_empty() => {
                        log::debug!("Root directory metadata found in KV and is not empty, skipping GitHub load trigger.");
                        skip_github_due_to_cache = true; // Treat existing KV data as reason to skip load
                    }
                    Ok(_) => {
                        log::debug!("Root directory metadata in KV is empty or missing.");
                        // Proceed to load from GitHub
                    }
                    Err(e) => {
                        log::warn!("Error checking root metadata in KV before GitHub load trigger: {:?}. Proceeding with load attempt.", e);
                        // Proceed to load from GitHub even if check failed
                    }
                }
            }

            if !skip_github_due_to_cache {
                log::debug!(
                    "Root directory listing and metadata appears empty/missing, triggering GitHub load..."
                );
                // Use shallow load (true) to only populate metadata placeholders
                match self.load_github_directory(&dir_path_for_lookup, true).await {
                    Ok(_) => {
                        log::info!("GitHub load triggered successfully for root directory.");
                        // Proceed to read metadata below
                    }
                    Err(load_err) => {
                        log::error!(
                            "GitHub load trigger failed for root directory: {:?}. Listing might be empty.",
                            load_err
                        );
                        // Proceed to read metadata below, which might still fail or return empty
                    }
                }
            }
        }

        // --- Read Final Directory Metadata ---
        // This read happens *after* any potential GitHub load for the root.
        log::debug!(
            "Attempting to read final directory metadata for '{}'",
            dir_path_for_lookup
        );
        let final_read_result = self
            .store
            .read_directory_metadata_from_kv(&dir_path_for_lookup)
            .await;

        match final_read_result {
            Ok(final_metadata) => {
                log::debug!(
                    "Successfully read final metadata for '{}', found {} entries.",
                    dir_path_for_lookup,
                    final_metadata.files.len()
                );
                // Process the metadata into DirectoryEntry items
                let mut final_entries = Vec::with_capacity(final_metadata.files.len());
                for (name, attrs) in final_metadata.files.iter() {
                    let entry_path = if attrs.is_directory {
                        build_dir_entry_path(&dir_path_for_lookup, name)
                    } else {
                        build_file_entry_path(&dir_path_for_lookup, name)
                    };
                    final_entries.push(DirectoryEntry {
                        name: name.clone(),
                        path: entry_path,
                        is_directory: attrs.is_directory,
                        attributes: Some(attrs.clone()),
                    });
                }
                log::debug!(
                    "Final listing for '{}' returning {} entries.",
                    dir_path_for_lookup,
                    final_entries.len()
                );
                Ok(final_entries)
            }
            Err(e) => {
                log::error!(
                    "Failed to read final directory metadata for '{}': {:?}. Returning error or empty list.",
                    dir_path_for_lookup,
                    e
                );
                // Handle final read errors
                match e {
                    VfsError::NotFound(_) => {
                        log::debug!("Final metadata read for '{}' resulted in NotFound, returning empty list.", dir_path_for_lookup);
                        Ok(Vec::new()) // Treat NotFound as empty directory
                    }
                    VfsError::Other(ref msg)
                        if msg.contains("Failed to parse DirectoryMetadata") =>
                    {
                        log::warn!(
                            "Parsing final metadata failed for '{}', treating as empty.",
                            dir_path_for_lookup
                        );
                        Ok(Vec::new())
                    }
                    _ => {
                        log::error!(
                            "Propagating error during final metadata read for '{}': {:?}",
                            dir_path_for_lookup,
                            e
                        );
                        Err(e) // Propagate other errors
                    }
                }
            }
        }
    }

    /// Create directory
    pub(crate) async fn create_directory_impl(&self, path: &str) -> Result<(), VfsError> {
        let normalized_path = normalize_path(path);
        // Use normalize_dir_path to ensure it represents a directory consistently
        let dir_path = normalize_dir_path(&normalized_path);

        if dir_path == "/" || dir_path.is_empty() {
            log::debug!("Attempted to create root directory, which implicitly exists.");
            return Ok(()); // Root implicitly exists
        }

        log::debug!("Attempting to create directory: {}", dir_path);

        // --- Ensure Parent Exists ---
        // Get parent path (must end with /)
        let parent = get_parent_directory(&dir_path);
        log::trace!("Parent directory for '{}' is '{}'", dir_path, parent);

        // Check parent existence recursively only if parent is not root
        if !parent.is_empty() && parent != "/" {
            // Check cache first for parent attributes
            let parent_attrs = self.store.read_from_metadata_cache(&parent).await;
            if parent_attrs.is_none() || !parent_attrs.unwrap().is_directory {
                // If not cached or not a directory, check KV marker
                if !self.store.directory_exists_in_kv(&parent).await? {
                    log::debug!(
                        "Parent directory '{}' does not exist. Creating recursively.",
                        parent
                    );
                    // Box the recursive future to avoid infinitely sized types
                    Box::pin(self.create_directory_impl(&parent)).await?;
                } else {
                    log::trace!("Parent directory '{}' exists (KV marker found).", parent);
                }
            } else {
                log::trace!(
                    "Parent directory '{}' exists (found in metadata cache).",
                    parent
                );
            }
        } else {
            log::trace!(
                "Parent directory is root ('{}'), skipping existence check.",
                parent
            );
        }

        // --- Create Directory Marker and Initial Metadata in KV ---
        // This function now handles creating the @@dir key with empty DirectoryMetadata JSON
        match self.store.create_directory_in_kv(&dir_path).await {
            Ok(_) => {
                log::debug!(
                    "Successfully ensured directory marker exists for: {}",
                    dir_path
                );
                // Optionally, cache the directory attributes immediately
                let dir_attributes = create_directory_attributes(&dir_path, "user"); // Pass path
                self.store
                    .write_to_metadata_cache(&dir_path, dir_attributes)
                    .await;
                Ok(())
            }
            Err(e) => {
                log::error!(
                    "Failed to create directory marker/metadata in KV for '{}': {:?}",
                    dir_path,
                    e
                );
                Err(e)
            }
        }
    }
}
