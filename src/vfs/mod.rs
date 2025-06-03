// Add the new modules
pub mod vercel_kv_github;
pub mod vercel_kv_helpers;
pub mod vercel_kv_js_bindings;
pub mod vercel_kv_store;
pub mod vercel_kv_types;

// VFS operations modules
pub mod vercel_kv_directory;
pub mod vercel_kv_github_loader;
pub mod vercel_kv_operations;

// Main VFS implementation
pub mod vercel_kv_vfs;

// Re-export core types and the main VFS implementation
pub use vercel_kv_store::{
    create_directory_attributes, create_file_attributes, get_real_path_from_key, is_internal_key,
    VercelKvStore,
};
pub use vercel_kv_types::{DirectoryEntry, FileAttributes, LoadDirectoryResult, LoadedFile};
pub use vercel_kv_vfs::VercelKvVfs;

// Re-export the helper macro
pub use vercel_kv_types::*;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum VfsError {
    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("File not found: {0}")]
    NotFound(String),

    #[error("Storage error: {0}")]
    StorageError(String),

    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Is a directory: {0}")]
    IsDirectory(String),

    #[error("Is not a directory: {0}")]
    NotDirectory(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Path already exists: {0}")]
    AlreadyExists(String),

    #[error("Operation not supported: {0}")]
    NotSupported(String),

    #[error("Other error: {0}")]
    Other(String),

    #[error("Invalid path: {0}")]
    InvalidPath(String),
}

// --- WASM Specific Helpers ---

#[cfg(target_arch = "wasm32")]
pub mod wasm_helpers {
    use super::{VercelKvVfs, VfsError};
    use crate::utils::file_wasm;
    use serde_json::{json, Value};
    use wasm_bindgen::prelude::*;

    /// Helper to get the VFS instance (WASM only).
    pub async fn get_vfs() -> Result<VercelKvVfs, VfsError> {
        file_wasm::get_vfs()
            .await
            .map_err(|e| VfsError::Other(format!("Failed to get VFS: {}", e)))
    }

    /// Helper to convert VfsError to JsValue for the FFI boundary (WASM only).
    pub fn vfs_error_to_js(err: VfsError) -> JsValue {
        let error_type = match &err {
            VfsError::NotFound(_) => "NotFound",
            VfsError::ConfigError(_) => "ConfigError",
            VfsError::StorageError(_) => "StorageError",
            VfsError::NetworkError(_) => "NetworkError",
            VfsError::IoError(_) => "IoError",
            VfsError::IsDirectory(_) => "IsDirectory",
            VfsError::NotDirectory(_) => "NotDirectory",
            VfsError::InvalidPath(_) => "InvalidPath",
            VfsError::PermissionDenied(_) => "PermissionDenied",
            VfsError::AlreadyExists(_) => "AlreadyExists",
            VfsError::NotSupported(_) => "NotSupported",
            VfsError::Other(_) => "Other",
        };

        let error_obj = json!({
            "type": error_type,
            "message": format!("{}", err)
        });

        // Serialize the JSON object to a string and then into JsValue
        let error_json = error_obj.to_string();
        JsValue::from_str(&error_json)
    }
}
