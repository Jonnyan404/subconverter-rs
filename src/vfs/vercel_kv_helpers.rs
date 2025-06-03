use crate::vfs::vercel_kv_types::*;

//------------------------------------------------------------------------------
// LOGGING
//------------------------------------------------------------------------------

/// Provides consistent debug logging across the VFS module
#[macro_export]
macro_rules! log_debug {
    ($($arg:tt)*) => {
        log::debug!($($arg)*)
    };
}

//------------------------------------------------------------------------------
// PATH MANIPULATION
//------------------------------------------------------------------------------

/// Normalizes a path by removing leading slashes for consistent key generation
///
/// # Arguments
/// * `path` - The path to normalize
///
/// # Returns
/// A normalized path string without leading slashes
pub fn normalize_path(path: &str) -> String {
    let normalized = path.trim_start_matches('/').to_string();
    // log_debug!("Normalized path: '{}' → '{}'", path, normalized);
    normalized
}

/// Checks if a path represents a directory
///
/// A path can be a directory in several ways:
/// 1. It ends with '/' (explicitly a directory)
/// 2. It is empty (the root directory)
/// 3. It could be a directory without trailing slash (will need additional verification)
///
/// # Arguments
/// * `path` - The path to check
///
/// # Returns
/// `true` if the path is definitely a directory path, `false` if it might be a file
pub fn is_directory_path(path: &str) -> bool {
    // Always consider explicit directory paths (ending with /) or root as directories
    let is_explicit_dir = path.ends_with('/') || path.is_empty();

    if is_explicit_dir {
        log_debug!("Path '{}' is explicitly a directory", path);
        return true;
    }

    // For paths without trailing slash, they could be directories
    // The caller should verify with directory_exists_in_kv
    log_debug!(
        "Path '{}' might be a file or directory without trailing slash",
        path
    );
    false
}

/// Extracts the parent directory path from a given path
///
/// # Arguments
/// * `path` - The path to get the parent directory from
///
/// # Returns
/// The parent directory path, ending with '/'. Returns empty string for top-level items.
pub fn get_parent_directory(path: &str) -> String {
    let path = path.trim_end_matches('/');
    let parent = match path.rfind('/') {
        Some(idx) => path[..=idx].to_string(),
        None => "".to_string(),
    };
    // log_debug!("Parent directory of '{}': '{}'", path, parent);
    parent
}

/// Extracts the filename from a path
/// This should return only the filename part, without any directory prefix.
///
/// # Arguments
/// * `path` - The path to extract the filename from
///
/// # Returns
/// The filename (last path component)
pub fn get_filename(path: &str) -> String {
    let path = path.trim_end_matches('/');
    let filename = match path.rfind('/') {
        Some(idx) => path[idx + 1..].to_string(),
        None => path.to_string(), // If no slash, the whole path is the filename
    };
    // log_debug!("Filename from path '{}': '{}'", path, filename);
    filename
}

//------------------------------------------------------------------------------
// KV KEY GENERATION
//------------------------------------------------------------------------------

/// Generates a complete key for KV storage by appending a suffix to a path
///
/// # Arguments
/// * `path` - The normalized path
/// * `suffix` - The suffix to append
///
/// # Returns
/// A key string for KV storage
pub fn get_key_with_suffix(path: &str, suffix: &str) -> String {
    let key = format!("{}{}", path, suffix);
    // log_debug!(
    //     "Generated key with suffix '{}' for path '{}': '{}'",
    //     suffix,
    //     path,
    //     key
    // );
    key
}

/// Generates the content key for a file path
///
/// # Arguments
/// * `path` - The normalized path
///
/// # Returns
/// A content key string for KV storage
pub fn get_content_key(path: &str) -> String {
    
    get_key_with_suffix(path, FILE_CONTENT_SUFFIX)
}

/// Generates the metadata key for a file path
///
/// # Arguments
/// * `path` - The normalized path
///
/// # Returns
/// A metadata key string for KV storage
// pub fn get_metadata_key(path: &str) -> String {
//     get_key_with_suffix(path, FILE_METADATA_SUFFIX)
// }

/// Generates the status key for a file path
///
/// # Arguments
/// * `path` - The normalized path
///
/// # Returns
/// A status key string for KV storage
// pub fn get_status_key(path: &str) -> String {
//     get_key_with_suffix(path, FILE_STATUS_SUFFIX)
// }

/// Generates the directory marker key for a directory path
///
/// # Arguments
/// * `path` - The normalized path
///
/// # Returns
/// A directory marker key string for KV storage
pub fn get_directory_marker_key(path: &str) -> String {
    // Ensure the path ends with a slash if it's not empty, then append @@dir
    let normalized = normalize_path(path);
    if normalized.is_empty() {
        // Root directory marker
        DIRECTORY_MARKER_SUFFIX.trim_start_matches('/').to_string()
    } else {
        // Append @@dir to the normalized path, ensuring it ends with /
        format!(
            "{}{}",
            normalized.trim_end_matches('/'),
            DIRECTORY_MARKER_SUFFIX
        )
    }
}

/// Generates the GitHub tree cache key for a repository+branch
///
/// # Arguments
/// * `owner` - Repository owner
/// * `repo` - Repository name
/// * `branch` - Branch name
/// * `recursive` - Whether this is for a recursive tree
///
/// # Returns
/// A GitHub tree cache key string for KV storage
pub fn get_github_tree_cache_key(owner: &str, repo: &str, branch: &str, recursive: bool) -> String {
    let recursive_flag = if recursive { "1" } else { "0" };
    format!(
        "{}/{}@{}@{}{}",
        owner, repo, branch, recursive_flag, GITHUB_TREE_CACHE_SUFFIX
    )
}

//------------------------------------------------------------------------------
// FILE TYPE HANDLING
//------------------------------------------------------------------------------

/// Guesses the MIME type of a file based on its extension
///
/// # Arguments
/// * `path` - The file path
///
/// # Returns
/// A MIME type string
pub fn guess_file_type(path: &str) -> String {
    // Extract the file extension and convert to lowercase
    let extension = path
        .rsplit('.')
        .next()
        .filter(|&ext| !ext.contains('/'))
        .map(|ext| ext.to_lowercase());

    // Map the extension to a MIME type
    let file_type = match extension {
        Some(ext) => match ext.as_str() {
            "txt" => "text/plain",
            "html" | "htm" => "text/html",
            "css" => "text/css",
            "js" => "application/javascript",
            "json" => "application/json",
            "xml" => "application/xml",
            "png" => "image/png",
            "jpg" | "jpeg" => "image/jpeg",
            "gif" => "image/gif",
            "svg" => "image/svg+xml",
            "pdf" => "application/pdf",
            "md" => "text/markdown",
            "ini" => "text/plain",
            "yaml" | "yml" => "application/yaml",
            "conf" => "text/plain",
            "rs" => "text/rust",          // Added Rust files
            "toml" => "application/toml", // Added TOML files
            "wasm" => "application/wasm", // Added WebAssembly files
            _ => "application/octet-stream",
        }
        .to_string(),
        None => "application/octet-stream".to_string(),
    };

    // log_debug!("Guessed file type for '{}': '{}'", path, file_type);
    file_type
}
