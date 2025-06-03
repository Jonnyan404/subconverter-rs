use crate::utils::system::safe_system_time;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::UNIX_EPOCH;
use wasm_bindgen::prelude::*;

use super::VfsError;

// File metadata structure
#[derive(Clone, Debug, Serialize, Deserialize)]
#[wasm_bindgen(getter_with_clone, inspectable)]
pub struct FileAttributes {
    /// Full path of the file or directory
    pub path: String,
    /// Size of the file in bytes
    pub size: usize,
    /// Creation timestamp (seconds since UNIX epoch)
    pub created_at: u64,
    /// Last modified timestamp (seconds since UNIX epoch)
    pub modified_at: u64,
    /// File type (mime type or extension)
    pub file_type: String,
    /// Is this a directory marker
    pub is_directory: bool,
    /// Source type of the file: user-modified, cloud-synced, or placeholder
    /// - "user" = modified by user and saved locally
    /// - "cloud" = pulled from cloud (GitHub) but not modified
    /// - "placeholder" = not loaded yet, but known to exist in cloud
    /// - "" = unknown or default
    /// This field now also implicitly represents the status.
    pub source_type: String,
}

#[wasm_bindgen]
impl FileAttributes {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        let now = safe_system_time()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Self {
            path: "".to_string(),
            size: 0,
            created_at: now,
            modified_at: now,
            file_type: "text/plain".to_string(),
            is_directory: false,
            source_type: "".to_string(),
        }
    }
}

impl Default for FileAttributes {
    fn default() -> Self {
        let now = safe_system_time()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Self {
            path: "".to_string(),
            size: 0,
            created_at: now,
            modified_at: now,
            file_type: "text/plain".to_string(),
            is_directory: false,
            source_type: "".to_string(),
        }
    }
}

// Directory entry for listing
#[derive(Clone, Debug, Serialize, Deserialize)]
#[wasm_bindgen(getter_with_clone, inspectable)]
pub struct DirectoryEntry {
    /// Name of the file or directory (not the full path)
    pub name: String,
    /// Full path to the file or directory
    pub path: String,
    /// Is this entry a directory
    pub is_directory: bool,
    /// File attributes
    /// For directories, this might hold the directory's own attributes
    /// For files, this holds the file's attributes
    #[wasm_bindgen(getter_with_clone)]
    pub attributes: Option<FileAttributes>,
}

#[wasm_bindgen]
impl DirectoryEntry {
    #[wasm_bindgen(constructor)]
    pub fn new(
        name: String,
        path: String,
        is_directory: bool,
        attributes: Option<FileAttributes>,
    ) -> Self {
        Self {
            name,
            path,
            is_directory,
            attributes,
        }
    }
}

/// Represents a file that was loaded from GitHub
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LoadedFile {
    /// Path to the file that was loaded
    pub path: String,
    /// Size of the file in bytes
    pub size: usize,
    /// Whether this is a placeholder entry (content not loaded)
    pub is_placeholder: bool,
    /// Whether this is a directory
    pub is_directory: bool,
}

/// Result of loading a directory from GitHub
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LoadDirectoryResult {
    /// Total number of files attempted to load
    pub total_files: usize,
    /// Number of files successfully loaded
    pub successful_files: usize,
    /// Number of files that failed to load
    pub failed_files: usize,
    /// Information about each successfully loaded file
    pub loaded_files: Vec<LoadedFile>,
}

// Constants
pub const FILE_CONTENT_SUFFIX: &str = "@@content";
pub const DIRECTORY_MARKER_SUFFIX: &str = "/@@dir";

//------------------------------------------------------------------------------
// NEW DIRECTORY METADATA TYPE
//------------------------------------------------------------------------------

/// Structure to store metadata for all files within a directory.
/// This will be serialized to JSON and stored under the directory's `@@dir` key.
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct DirectoryMetadata {
    /// Maps relative filename (within the directory) to its FileAttributes.
    pub files: HashMap<String, FileAttributes>,
}

//------------------------------------------------------------------------------
// GITHUB CACHE TYPES
//------------------------------------------------------------------------------

/// The suffix for GitHub tree cache entries in KV store
pub const GITHUB_TREE_CACHE_SUFFIX: &str = "@@github_tree_cache";

/// Structure to store GitHub tree cache data
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GitHubTreeCache {
    /// Tree response data
    pub data: String,
    /// When the cache was created
    pub created_at: u64,
    /// How long the cache is valid for in seconds
    pub ttl: u64,
}

impl GitHubTreeCache {
    /// Check if the cache entry has expired
    pub fn is_expired(&self, current_time: u64) -> bool {
        current_time > self.created_at + self.ttl
    }
}

// VFS trait definition
pub trait VirtualFileSystem {
    fn read_file(&self, path: &str)
        -> impl std::future::Future<Output = Result<Vec<u8>, VfsError>>;
    fn write_file(
        &self,
        path: &str,
        content: Vec<u8>,
    ) -> impl std::future::Future<Output = Result<(), VfsError>>;
    fn exists(&self, path: &str) -> impl std::future::Future<Output = Result<bool, VfsError>>;
    fn delete_file(&self, path: &str) -> impl std::future::Future<Output = Result<(), VfsError>>;
    fn read_file_attributes(
        &self,
        path: &str,
    ) -> impl std::future::Future<Output = Result<FileAttributes, VfsError>>;
    fn list_directory(
        &self,
        path: &str,
    ) -> impl std::future::Future<Output = Result<Vec<DirectoryEntry>, VfsError>>;
    fn list_directory_skip_github(
        &self,
        path: &str,
    ) -> impl std::future::Future<Output = Result<Vec<DirectoryEntry>, VfsError>>;
    fn create_directory(
        &self,
        path: &str,
    ) -> impl std::future::Future<Output = Result<(), VfsError>>;

    /// Load files from a GitHub repository directory (recursive)
    fn load_github_directory(
        &self,
        directory_path: &str,
        shallow: bool,
    ) -> impl std::future::Future<Output = Result<LoadDirectoryResult, VfsError>>;

    /// Load only direct children of a GitHub repository directory (non-recursive)
    fn load_github_directory_flat(
        &self,
        directory_path: &str,
        shallow: bool,
    ) -> impl std::future::Future<Output = Result<LoadDirectoryResult, VfsError>>;

    /// Initializes the VFS by attempting to load the root directory from GitHub
    /// if it hasn't been loaded yet.
    /// Returns `true` if the GitHub load was actually triggered, `false` otherwise.
    fn initialize_github_load(&self) -> impl std::future::Future<Output = Result<bool, VfsError>>;
}
