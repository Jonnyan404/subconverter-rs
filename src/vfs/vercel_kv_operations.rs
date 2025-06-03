use crate::utils::http_wasm::{web_get_async, ProxyConfig};
use crate::vfs::vercel_kv_helpers::*;
use crate::vfs::vercel_kv_store::create_file_attributes;
use crate::vfs::vercel_kv_vfs::VercelKvVfs;
use case_insensitive_string::CaseInsensitiveString;
use std::collections::HashMap;

use super::VfsError;

// Define helper methods for VercelKvVfs to keep the implementation
// manageable and well-organized
impl VercelKvVfs {
    // File operations

    /// Read a file from the VFS
    pub(crate) async fn read_file_impl(&self, path: &str) -> Result<Vec<u8>, VfsError> {
        let normalized_path = normalize_path(path);
        log::debug!("Reading file: {}", normalized_path);

        // Check memory cache first
        if let Some(content) = self.store.read_from_memory_cache(&normalized_path).await {
            log::debug!("Cache hit for: {}", normalized_path);
            return Ok(content);
        }

        log::debug!("Cache miss for: {}", normalized_path);

        // Check KV store for content
        match self.store.read_from_kv(&normalized_path).await {
            Ok(Some(content)) => {
                log::debug!(
                    "Found content in KV for: {} ({} bytes)",
                    normalized_path,
                    content.len()
                );
                // Cache the content
                self.store
                    .write_to_memory_cache(&normalized_path, content.clone())
                    .await;
                Ok(content)
            }
            Ok(None) => {
                // No content in KV, try GitHub
                log::debug!("No content in KV for: {}, checking GitHub", normalized_path);
                self.read_from_github_and_cache(&normalized_path).await
            }
            Err(e) => {
                // Error reading from KV, try GitHub as fallback?
                log::warn!(
                    "Error reading from KV for {}: {:?}. Trying GitHub.",
                    normalized_path,
                    e
                );
                self.read_from_github_and_cache(&normalized_path).await
            }
        }
    }

    /// Helper to read from GitHub, cache results, and update metadata
    async fn read_from_github_and_cache(&self, normalized_path: &str) -> Result<Vec<u8>, VfsError> {
        let raw_url = self.github_config.get_raw_url(normalized_path);
        log::debug!("Fetching from GitHub: {}", raw_url);

        // Prepare headers with authorization if token is available
        let mut headers = HashMap::new();
        if let Some(token) = &self.github_config.auth_token {
            headers.insert(
                CaseInsensitiveString::new("Authorization"),
                format!("token {}", token),
            );
        }
        headers.insert(
            CaseInsensitiveString::new("User-Agent"),
            "subconverter-rs".to_string(),
        );

        let proxy_config = ProxyConfig::default();

        match web_get_async(&raw_url, &proxy_config, Some(&headers)).await {
            Ok(response) => {
                if (200..300).contains(&response.status) {
                    let content = response.body.into_bytes();
                    log::debug!(
                        "Successfully fetched from GitHub: {} ({} bytes)",
                        normalized_path,
                        content.len()
                    );

                    // Create file attributes for the cloud file
                    let attributes = create_file_attributes(
                        normalized_path,
                        content.len(),
                        "cloud", // Source is cloud
                    );

                    // Update memory cache
                    self.store
                        .write_to_memory_cache(normalized_path, content.clone())
                        .await;

                    // Update metadata cache
                    self.store
                        .write_to_metadata_cache(normalized_path, attributes.clone())
                        .await;

                    // Write content and metadata to KV in background
                    self.store
                        .write_to_kv_background(normalized_path.to_string(), content.clone());
                    self.store.write_file_attributes_to_dir_kv_background(
                        normalized_path.to_string(),
                        attributes,
                    );

                    Ok(content)
                } else {
                    log::warn!(
                        "GitHub fetch failed for {}: status {}, body: {}",
                        normalized_path,
                        response.status,
                        response.body
                    );
                    Err(VfsError::NotFound(format!(
                        "File not found on GitHub: {}",
                        normalized_path
                    )))
                }
            }
            Err(e) => {
                log::error!(
                    "Error fetching from GitHub {}: {}",
                    normalized_path,
                    e.message
                );
                Err(VfsError::NetworkError(format!(
                    "Network error fetching {}: {}",
                    normalized_path, e.message
                )))
            }
        }
    }

    /// Write a file to the VFS
    pub(crate) async fn write_file_impl(
        &self,
        path: &str,
        content: Vec<u8>,
    ) -> Result<(), VfsError> {
        let normalized_path = normalize_path(path);
        log::debug!("Writing file: {}", normalized_path);

        // Create file attributes for the user file
        let attributes = create_file_attributes(
            &normalized_path,
            content.len(),
            "user", // Source is user
        );

        // Ensure parent directory exists (creates marker if needed)
        let parent = get_parent_directory(&normalized_path);
        if !parent.is_empty() {
            self.create_directory_impl(&parent).await?;
        }

        // Write content to KV
        self.store.write_to_kv(&normalized_path, &content).await?;

        // Write attributes to parent directory's metadata in KV
        self.store
            .write_file_attributes_to_dir_kv(&normalized_path, &attributes)
            .await?;

        // Update caches
        self.store
            .write_to_memory_cache(&normalized_path, content)
            .await;
        self.store
            .write_to_metadata_cache(&normalized_path, attributes)
            .await;

        Ok(())
    }

    /// Check if a file or directory exists
    pub(crate) async fn exists_impl(&self, path: &str) -> Result<bool, VfsError> {
        let normalized_path = normalize_path(path);
        log::debug!("Checking existence for: {}", normalized_path);

        // 1. Check memory caches (content or metadata)
        if self.store.exists_in_memory_cache(&normalized_path).await
            || self.store.exists_in_metadata_cache(&normalized_path).await
        {
            log::debug!("Found in cache: {}", normalized_path);
            return Ok(true);
        }

        // 2. Check for file content key in KV
        if self.store.exists_in_kv(&normalized_path).await? {
            log::debug!("Found content key in KV: {}", normalized_path);
            return Ok(true);
        }

        // 3. Check for directory marker key in KV
        if self.store.directory_exists_in_kv(&normalized_path).await? {
            log::debug!("Found directory marker in KV: {}", normalized_path);
            return Ok(true);
        }

        // 4. Check if attributes exist in parent directory's metadata KV
        if let Ok(Some(_)) = self
            .store
            .read_file_attributes_from_dir_kv(&normalized_path)
            .await
        {
            log::debug!("Found attributes in parent dir KV for: {}", normalized_path);
            return Ok(true);
        }

        // 5. Check GitHub (optional - might be slow)
        // Consider adding a flag to control this check if performance is critical
        log::debug!("Checking GitHub for: {}", normalized_path);
        if self
            .load_github_file_info_impl(&normalized_path)
            .await
            .is_ok()
        {
            log::debug!("Found on GitHub: {}", normalized_path);
            return Ok(true);
        }

        log::debug!("Path not found: {}", normalized_path);
        Ok(false)
    }

    /// Delete a file from the VFS
    pub(crate) async fn delete_file_impl(&self, path: &str) -> Result<(), VfsError> {
        let normalized_path = normalize_path(path);
        log::debug!("Deleting file: {}", normalized_path);

        // Delete content from KV
        let content_delete_result = self.store.delete_from_kv(&normalized_path).await;

        // Delete attributes from parent directory metadata in KV
        let attributes_delete_result = self
            .store
            .delete_file_attributes_from_dir_kv(&normalized_path)
            .await;

        // Remove from caches
        self.store.remove_from_memory_cache(&normalized_path).await;
        self.store
            .remove_from_metadata_cache(&normalized_path)
            .await;

        // Check results (report first error encountered)
        content_delete_result?; // Propagate KV content delete error first
        attributes_delete_result?; // Propagate KV attributes delete error second

        Ok(())
    }

    /// Delete directory (recursive)
    pub(crate) async fn delete_directory_impl(&self, path: &str) -> Result<(), VfsError> {
        let normalized_path = normalize_path(path);
        log::debug!("Deleting directory recursively: {}", normalized_path);

        // List directory contents (without GitHub supplement for deletion)
        let entries = self.list_directory_impl(&normalized_path, true).await?;

        // Recursively delete children
        for entry in entries {
            if entry.is_directory {
                // Box the recursive future
                Box::pin(self.delete_directory_impl(&entry.path)).await?;
            } else {
                self.delete_file_impl(&entry.path).await?;
            }
        }

        // Delete the directory marker itself
        self.store
            .delete_directory_marker_from_kv(&normalized_path)
            .await?;

        // Remove directory from metadata cache
        self.store
            .remove_from_metadata_cache(&normalized_path)
            .await;

        // Optional: Clean up directory entry from parent's metadata?
        // let parent = get_parent_directory(&normalized_path);
        // if !parent.is_empty() {
        //     // This would involve reading parent, removing entry, writing parent back
        //     // Might be complex/racy, consider if necessary.
        // }

        Ok(())
    }
}
