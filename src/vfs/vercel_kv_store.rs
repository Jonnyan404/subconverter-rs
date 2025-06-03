use crate::utils::system::safe_system_time;
use crate::vfs::vercel_kv_helpers::*;
use crate::vfs::vercel_kv_js_bindings::*;
use crate::vfs::vercel_kv_types::*;
use crate::vfs::VfsError;
use serde_json;
use serde_wasm_bindgen;
use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::Arc;
use std::time::UNIX_EPOCH;
use tokio::sync::RwLock;
use wasm_bindgen_futures;

/// Represents the storage layer for Vercel KV VFS
/// Handles all interactions with KV store and memory caches
#[derive(Clone)]
pub struct VercelKvStore {
    memory_cache: Arc<RwLock<HashMap<String, Vec<u8>>>>,
    metadata_cache: Arc<RwLock<HashMap<String, FileAttributes>>>,
}

impl VercelKvStore {
    /// Create a new KV store instance
    pub fn new() -> Self {
        Self {
            memory_cache: Arc::new(RwLock::new(HashMap::new())),
            metadata_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Get memory cache reference
    pub fn get_memory_cache(&self) -> Arc<RwLock<HashMap<String, Vec<u8>>>> {
        self.memory_cache.clone()
    }

    /// Get metadata cache reference
    pub fn get_metadata_cache(&self) -> Arc<RwLock<HashMap<String, FileAttributes>>> {
        self.metadata_cache.clone()
    }

    //------------------------------------------------------------------------------
    // Memory Cache Operations
    //------------------------------------------------------------------------------

    /// Read file content from memory cache
    pub async fn read_from_memory_cache(&self, path: &str) -> Option<Vec<u8>> {
        self.memory_cache.read().await.get(path).cloned()
    }

    /// Write file content to memory cache
    pub async fn write_to_memory_cache(&self, path: &str, content: Vec<u8>) {
        self.memory_cache
            .write()
            .await
            .insert(path.to_string(), content);
    }

    /// Check if file exists in memory cache
    pub async fn exists_in_memory_cache(&self, path: &str) -> bool {
        self.memory_cache.read().await.contains_key(path)
    }

    /// Remove file from memory cache
    pub async fn remove_from_memory_cache(&self, path: &str) {
        self.memory_cache.write().await.remove(path);
    }

    //------------------------------------------------------------------------------
    // Metadata Cache Operations
    //------------------------------------------------------------------------------

    /// Read file attributes from metadata cache
    pub async fn read_from_metadata_cache(&self, path: &str) -> Option<FileAttributes> {
        self.metadata_cache.read().await.get(path).cloned()
    }

    /// Write file attributes to metadata cache
    pub async fn write_to_metadata_cache(&self, path: &str, attributes: FileAttributes) {
        self.metadata_cache
            .write()
            .await
            .insert(path.to_string(), attributes);
    }

    /// Check if metadata exists in cache
    pub async fn exists_in_metadata_cache(&self, path: &str) -> bool {
        self.metadata_cache.read().await.contains_key(path)
    }

    /// Remove metadata from cache
    pub async fn remove_from_metadata_cache(&self, path: &str) {
        self.metadata_cache.write().await.remove(path);
    }

    //------------------------------------------------------------------------------
    // KV Store Content Operations
    //------------------------------------------------------------------------------

    /// Read file content from KV store
    pub async fn read_from_kv(&self, path: &str) -> Result<Option<Vec<u8>>, VfsError> {
        let content_key = get_content_key(path);
        match kv_get(&content_key).await {
            Ok(js_value) => {
                if js_value.is_null() || js_value.is_undefined() {
                    Ok(None)
                } else {
                    // Convert JsValue to Vec<u8>
                    let content: Vec<u8> =
                        serde_wasm_bindgen::from_value(js_value).map_err(|e| {
                            VfsError::Other(format!("Failed to deserialize file content: {}", e))
                        })?;
                    Ok(Some(content))
                }
            }
            Err(e) => Err(js_error_to_vfs(e, "Failed to read file from KV")),
        }
    }

    /// Write file content to KV store
    pub async fn write_to_kv(&self, path: &str, content: &[u8]) -> Result<(), VfsError> {
        let content_key = get_content_key(path);
        match kv_set(&content_key, content).await {
            Ok(_) => Ok(()),
            Err(e) => Err(js_error_to_vfs(e, "Failed to write file to KV")),
        }
    }

    /// Write file content to KV store in background (non-blocking)
    pub fn write_to_kv_background(&self, path: String, content: Vec<u8>) {
        let content_key = get_content_key(&path);
        let content_clone = content.clone();

        wasm_bindgen_futures::spawn_local(async move {
            match kv_set(&content_key, &content_clone).await {
                Ok(_) => {
                    log::debug!("Successfully stored {} in KV background.", path);
                }
                Err(e) => {
                    log::error!("Background KV write error for {}: {:?}", path, e);
                }
            }
        });
    }

    /// Check if file exists in KV store
    pub async fn exists_in_kv(&self, path: &str) -> Result<bool, VfsError> {
        let content_key = get_content_key(path);
        match kv_exists(&content_key).await {
            Ok(js_value) => {
                let exists = js_value.as_bool().unwrap_or(false);
                Ok(exists)
            }
            Err(e) => Err(js_error_to_vfs(e, "Failed to check if file exists in KV")),
        }
    }

    /// Delete file from KV store
    pub async fn delete_from_kv(&self, path: &str) -> Result<(), VfsError> {
        let content_key = get_content_key(path);
        match kv_del(&content_key).await {
            Ok(_) => Ok(()),
            Err(e) => Err(js_error_to_vfs(e, "Failed to delete file from KV")),
        }
    }

    //------------------------------------------------------------------------------
    // KV Store Directory Metadata Operations
    //------------------------------------------------------------------------------

    /// Reads the entire DirectoryMetadata JSON object for a given directory path.
    pub async fn read_directory_metadata_from_kv(
        &self,
        dir_path: &str,
    ) -> Result<DirectoryMetadata, VfsError> {
        let dir_key = get_directory_marker_key(dir_path);
        log::debug!(
            "Inside read_directory_metadata_from_kv for key: '{}'",
            dir_key
        );
        match kv_get_text(&dir_key).await {
            Ok(js_value) => {
                if js_value.is_null() || js_value.is_undefined() {
                    // If the directory marker doesn't exist or has no JSON content, return default (empty) metadata
                    log::debug!(
                        "Metadata text for key '{}' is null or undefined, returning default.",
                        dir_key
                    );
                    Ok(DirectoryMetadata::default())
                } else {
                    // Expecting a JsString, convert to Rust String and parse
                    js_value.as_string().map_or_else(|| {
                        log::error!("kv_get_text returned non-string value for key '{}': {:?}", dir_key, js_value);
                        Err(VfsError::Other(format!(
                            "KV store returned non-string type for directory metadata key '{}'", dir_key
                        )))
                    }, |json_string| {
                        log::debug!("Attempting to parse JSON string for key '{}', length: {}", dir_key, json_string.len());
                        serde_json::from_str::<DirectoryMetadata>(&json_string).map_err(|e| {
                            log::error!(
                                "Failed to parse DirectoryMetadata JSON string for '{}': {}, JSON: {}",
                                dir_path,
                                e,
                                json_string // Log the problematic JSON string
                            );
                            VfsError::Other(format!(
                                "Failed to parse DirectoryMetadata for '{}': {}",
                                dir_path, e
                            ))
                        })
                    })
                }
            }
            Err(e) => Err(js_error_to_vfs(
                e,
                &format!(
                    "Failed to read directory metadata from KV for '{}'",
                    dir_path
                ),
            )),
        }
    }

    /// Writes the entire DirectoryMetadata JSON object for a given directory path.
    pub async fn write_directory_metadata_to_kv(
        &self,
        dir_path: &str,
        dir_metadata: &DirectoryMetadata,
    ) -> Result<(), VfsError> {
        let dir_key = get_directory_marker_key(dir_path);
        // Serialize DirectoryMetadata to a JSON string
        let metadata_json_string = serde_json::to_string(dir_metadata).map_err(|e| {
            VfsError::Other(format!(
                "Failed to serialize DirectoryMetadata for '{}': {}",
                dir_path, e
            ))
        })?;

        log::debug!(
            "Writing directory metadata text to KV for '{}', length: {}",
            dir_path,
            metadata_json_string.len()
        );

        match kv_set_text(&dir_key, &metadata_json_string).await {
            Ok(_) => Ok(()),
            Err(e) => Err(js_error_to_vfs(
                e,
                &format!(
                    "Failed to write directory metadata to KV for '{}'",
                    dir_path
                ),
            )),
        }
    }

    /// Writes DirectoryMetadata to KV in background (non-blocking).
    pub fn write_directory_metadata_to_kv_background(
        &self,
        dir_path: String,
        dir_metadata: DirectoryMetadata,
    ) {
        let dir_key = get_directory_marker_key(&dir_path);
        // Serialize DirectoryMetadata to JSON string
        let metadata_json_string = match serde_json::to_string(&dir_metadata) {
            Ok(s) => s,
            Err(e) => {
                log::error!(
                    "Failed to serialize DirectoryMetadata for background write '{}': {}",
                    dir_path,
                    e
                );
                return;
            }
        };

        log::debug!(
            "Writing directory metadata text to KV for '{}' (background), length: {}",
            dir_path,
            metadata_json_string.len()
        );

        wasm_bindgen_futures::spawn_local(async move {
            match kv_set_text(&dir_key, &metadata_json_string).await {
                Ok(_) => {
                    log::debug!(
                        "Successfully stored directory metadata for '{}' in KV background.",
                        dir_path
                    );
                }
                Err(e) => {
                    log::error!(
                        "Background KV directory metadata write error for '{}': {:?}",
                        dir_path,
                        e
                    );
                }
            }
        });
    }

    /// Reads attributes for a *specific file* within a directory's metadata JSON.
    pub async fn read_file_attributes_from_dir_kv(
        &self,
        path: &str,
    ) -> Result<Option<FileAttributes>, VfsError> {
        let parent_dir = get_parent_directory(path);
        let filename = get_filename(path);
        if filename.is_empty() {
            return Err(VfsError::InvalidPath("Path cannot be empty.".to_string()));
        }

        match self.read_directory_metadata_from_kv(&parent_dir).await {
            Ok(dir_metadata) => Ok(dir_metadata.files.get(&filename).cloned()),
            Err(e) => Err(e), // Propagate read error
        }
    }

    /// Writes attributes for a *specific file* into its directory's metadata JSON.
    pub async fn write_file_attributes_to_dir_kv(
        &self,
        path: &str,
        attributes: &FileAttributes,
    ) -> Result<(), VfsError> {
        let parent_dir = get_parent_directory(path);
        let filename = get_filename(path);
        if filename.is_empty() {
            return Err(VfsError::InvalidPath(
                "Path cannot be empty for attribute write.".to_string(),
            ));
        }

        // Read-modify-write the directory metadata
        let mut dir_metadata = self.read_directory_metadata_from_kv(&parent_dir).await?;
        dir_metadata.files.insert(filename, attributes.clone());
        self.write_directory_metadata_to_kv(&parent_dir, &dir_metadata)
            .await
    }

    /// Writes attributes for a *specific file* into its directory's metadata JSON in the background.
    pub fn write_file_attributes_to_dir_kv_background(
        &self,
        path: String,
        attributes: FileAttributes,
    ) {
        let parent_dir = get_parent_directory(&path);
        let filename = get_filename(&path);
        if filename.is_empty() {
            log::error!("Cannot write attributes for empty filename path: {}", path);
            return;
        }

        // Clone the Arc store itself, which is cheap and gives ownership to the async block
        let store_clone = self.clone();

        wasm_bindgen_futures::spawn_local(async move {
            match store_clone
                .read_directory_metadata_from_kv(&parent_dir)
                .await
            {
                Ok(mut dir_metadata) => {
                    dir_metadata.files.insert(filename, attributes);
                    // Now write the modified directory metadata back
                    store_clone.write_directory_metadata_to_kv_background(parent_dir, dir_metadata);
                }
                Err(e) => {
                    log::error!("Failed to read directory metadata for background write of file attributes for path '{}': {:?}", path, e);
                }
            }
        });
    }

    /// Deletes attributes for a *specific file* from its directory's metadata JSON.
    pub async fn delete_file_attributes_from_dir_kv(&self, path: &str) -> Result<(), VfsError> {
        let parent_dir = get_parent_directory(path);
        let filename = get_filename(path);
        if filename.is_empty() {
            return Err(VfsError::InvalidPath(
                "Path cannot be empty for attribute delete.".to_string(),
            ));
        }

        // Read-modify-write the directory metadata
        let mut dir_metadata = self.read_directory_metadata_from_kv(&parent_dir).await?;
        let existed = dir_metadata.files.remove(&filename).is_some();
        if existed {
            // Only write back if the file was actually removed
            self.write_directory_metadata_to_kv(&parent_dir, &dir_metadata)
                .await?;
        }
        // If it didn't exist, no error, just do nothing.
        Ok(())
    }

    //------------------------------------------------------------------------------
    // Directory Operations
    //------------------------------------------------------------------------------

    /// Check if directory exists in KV store
    pub async fn directory_exists_in_kv(&self, path: &str) -> Result<bool, VfsError> {
        let dir_key = get_directory_marker_key(path);
        match kv_exists(&dir_key).await {
            Ok(js_value) => {
                let exists = js_value.as_bool().unwrap_or(false);
                Ok(exists)
            }
            Err(e) => Err(js_error_to_vfs(
                e,
                &format!("Failed to check if directory exists in KV for '{}'", path),
            )),
        }
    }

    /// Creates a directory marker key in KV.
    /// Note: This no longer writes initial metadata, as that will be handled
    /// by the first process that needs to add file attributes to the directory.
    pub async fn create_directory_in_kv(&self, path: &str) -> Result<(), VfsError> {
        if path.is_empty() {
            log::warn!("Attempted to explicitly create root directory marker, skipping.");
            return Ok(());
        }
        // let dir_key = get_directory_marker_key(path); // dir_key not strictly needed now
        // Check if it already exists
        if self.directory_exists_in_kv(path).await? {
            log::debug!(
                "Directory marker already exists for '{}', skipping creation.",
                path
            );
            return Ok(()); // Idempotent: already exists is success
        }

        // Since we are not writing anything if it doesn't exist, we just return Ok.
        // The existence check ensures we don't signal an error if it's already there.
        // The *actual* creation of the metadata blob happens when the first file is added.
        // For now, simply ensuring the conceptual directory can be created (or already exists)
        // is sufficient for the caller (load_github_directory_impl).
        Ok(())
    }

    /// Delete directory marker from KV store
    pub async fn delete_directory_marker_from_kv(&self, path: &str) -> Result<(), VfsError> {
        let dir_key = get_directory_marker_key(path);
        match kv_del(&dir_key).await {
            Ok(_) => Ok(()),
            Err(e) => Err(js_error_to_vfs(
                e,
                &format!("Failed to delete directory marker from KV for '{}'", path),
            )),
        }
    }

    /// List keys with prefix from KV store (for directory listing)
    pub async fn list_keys_with_prefix(&self, prefix: &str) -> Result<Vec<String>, VfsError> {
        match kv_list(prefix).await {
            Ok(js_value) => {
                // Convert JsValue to Vec<String>
                let keys: Vec<String> = serde_wasm_bindgen::from_value(js_value)
                    .map_err(|e| VfsError::Other(format!("Failed to deserialize keys: {}", e)))?;
                Ok(keys)
            }
            Err(e) => Err(js_error_to_vfs(e, "Failed to list keys from KV")),
        }
    }

    //------------------------------------------------------------------------------
    // GitHub Cache Operations
    //------------------------------------------------------------------------------

    /// Read GitHub tree cache from KV store
    pub async fn read_github_tree_cache(
        &self,
        cache_key: &str,
    ) -> Result<Option<GitHubTreeCache>, VfsError> {
        match kv_get(cache_key).await {
            Ok(js_value) => {
                if js_value.is_null() || js_value.is_undefined() {
                    Ok(None)
                } else {
                    // Convert JsValue to Vec<u8>
                    let cache_bytes: Vec<u8> =
                        serde_wasm_bindgen::from_value(js_value).map_err(|e| {
                            VfsError::Other(format!("Failed to deserialize cache bytes: {}", e))
                        })?;

                    let cache: GitHubTreeCache =
                        serde_json::from_slice(&cache_bytes).map_err(|e| {
                            VfsError::Other(format!("Failed to parse GitHub tree cache: {}", e))
                        })?;

                    Ok(Some(cache))
                }
            }
            Err(e) => Err(js_error_to_vfs(
                e,
                "Failed to read GitHub tree cache from KV",
            )),
        }
    }

    /// Write GitHub tree cache to KV store
    pub async fn write_github_tree_cache(
        &self,
        cache_key: &str,
        cache: &GitHubTreeCache,
    ) -> Result<(), VfsError> {
        let cache_json = serde_json::to_vec(cache).map_err(|e| {
            VfsError::Other(format!("Failed to serialize GitHub tree cache: {}", e))
        })?;

        match kv_set(cache_key, &cache_json).await {
            Ok(_) => Ok(()),
            Err(e) => Err(js_error_to_vfs(
                e,
                "Failed to write GitHub tree cache to KV",
            )),
        }
    }

    /// Write GitHub tree cache to KV store in background
    pub fn write_github_tree_cache_background(&self, cache_key: String, cache: GitHubTreeCache) {
        let cache_json = match serde_json::to_vec(&cache) {
            Ok(json) => json,
            Err(e) => {
                log::error!("Failed to serialize GitHub tree cache: {}", e);
                return;
            }
        };

        wasm_bindgen_futures::spawn_local(async move {
            match kv_set(&cache_key, &cache_json).await {
                Ok(_) => {
                    log::debug!("Successfully stored GitHub tree cache for {}", cache_key);
                }
                Err(e) => {
                    log::error!(
                        "Background KV write error for GitHub tree cache {}: {:?}",
                        cache_key,
                        e
                    );
                }
            }
        });
    }
}

// Helper function to create FileAttributes for files (moved from Vfs?)
pub fn create_file_attributes(
    path: &str,
    content_size: usize,
    source_type: &str,
) -> FileAttributes {
    let now = safe_system_time()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    FileAttributes {
        path: path.to_string(), // Set the path
        size: content_size,
        created_at: now, // Consider if created_at should be persisted/updated differently
        modified_at: now,
        file_type: guess_file_type(path),
        is_directory: false,
        source_type: source_type.to_string(),
    }
}

// Helper function to create FileAttributes for directories (moved from Vfs?)
pub fn create_directory_attributes(path: &str, source_type: &str) -> FileAttributes {
    let now = safe_system_time()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    FileAttributes {
        path: path.to_string(), // Set the path
        size: 0,
        created_at: now,
        modified_at: now,
        file_type: "inode/directory".to_string(), // Set fixed directory type
        is_directory: true,
        source_type: source_type.to_string(),
    }
}

/// Helper function to check if a key is an internal VFS key (suffix based)
pub fn is_internal_key(key: &str) -> bool {
    key.ends_with(FILE_CONTENT_SUFFIX)
        || key.ends_with(DIRECTORY_MARKER_SUFFIX)
        || key.ends_with(GITHUB_TREE_CACHE_SUFFIX)
}

/// Helper function to extract the real VFS path from a KV key
pub fn get_real_path_from_key(key: &str) -> Option<String> {
    if key.ends_with(FILE_CONTENT_SUFFIX) {
        key.strip_suffix(FILE_CONTENT_SUFFIX).map(|s| s.to_string())
    } else if key.ends_with(DIRECTORY_MARKER_SUFFIX) {
        // For directory markers, return path ending with / for consistency
        key.strip_suffix(DIRECTORY_MARKER_SUFFIX)
            .map(|s| format!("{}/", s.trim_end_matches('/')))
    } else {
        None // GitHub cache keys don't represent direct VFS paths
    }
}
