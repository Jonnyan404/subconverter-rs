use crate::vfs::{vercel_kv_vfs::VercelKvVfs, VfsError};
use log::{debug, error, info, warn}; // Import log macros
use once_cell::sync::Lazy;
use std::io;
use std::path::Path;
use tokio::sync::Mutex; // Use Mutex if VFS needs mutable access, or if init is async
                        // Import the trait
use crate::vfs::VirtualFileSystem;

// Global VFS instance - initialized lazily and asynchronously
// We need an async initialization pattern, Lazy cannot directly await.
// Let's use a Mutex around an Option<VercelKvVfs> and initialize on first use.
static VFS: Lazy<Mutex<Option<VercelKvVfs>>> = Lazy::new(|| Mutex::new(None));

/// Get the VFS instance, initializing it if needed
pub async fn get_vfs() -> Result<VercelKvVfs, io::Error> {
    let mut vfs_guard = VFS.lock().await;
    if vfs_guard.is_none() {
        info!("Initializing VercelKvVfs..."); // Keep info level for init
        let vfs = VercelKvVfs::new().map_err(|e| {
            error!("VFS initialization failed: {}", e);
            io::Error::new(io::ErrorKind::Other, format!("VFS init failed: {}", e))
        })?;
        *vfs_guard = Some(vfs);
        info!("VercelKvVfs initialized successfully."); // Keep info level
    }
    // Clone the VFS instance for use (it's Clone)
    vfs_guard.as_ref().cloned().ok_or_else(|| {
        error!("Attempted to use VFS before initialization");
        io::Error::new(io::ErrorKind::Other, "VFS not initialized")
    })
}

// Helper to map VfsError to io::Error
fn map_vfs_error(e: VfsError) -> io::Error {
    debug!("Mapping VfsError to io::Error: {:?}", e);
    match e {
        VfsError::NotFound(msg) => io::Error::new(io::ErrorKind::NotFound, msg),
        VfsError::ConfigError(msg) => io::Error::new(io::ErrorKind::InvalidInput, msg),
        VfsError::StorageError(msg) => io::Error::new(io::ErrorKind::Other, msg),
        VfsError::NetworkError(msg) => io::Error::new(io::ErrorKind::NotConnected, msg),
        VfsError::IoError(err) => err,
        VfsError::IsDirectory(msg) => io::Error::new(io::ErrorKind::Other, msg),
        VfsError::NotDirectory(msg) => io::Error::new(io::ErrorKind::Other, msg),
        VfsError::PermissionDenied(msg) => io::Error::new(io::ErrorKind::PermissionDenied, msg),
        VfsError::AlreadyExists(msg) => io::Error::new(io::ErrorKind::AlreadyExists, msg),
        VfsError::NotSupported(msg) => io::Error::new(io::ErrorKind::Unsupported, msg),
        VfsError::InvalidPath(msg) => io::Error::new(io::ErrorKind::InvalidInput, msg),
        VfsError::Other(msg) => io::Error::new(io::ErrorKind::Other, msg),
    }
}

/// Read a file into a string (async)
pub async fn read_file(path: &str) -> Result<String, io::Error> {
    let vfs = get_vfs().await?;
    let bytes_result = vfs.read_file(path).await;
    match bytes_result {
        Ok(bytes) => String::from_utf8(bytes).map_err(|e| {
            error!("Failed to convert bytes to UTF-8 for {}: {}", path, e);
            io::Error::new(io::ErrorKind::InvalidData, e)
        }),
        Err(e) => {
            warn!("VFS read_file failed for {}: {:?}", path, e);
            Err(map_vfs_error(e))
        }
    }
}

/// Async version of read_file that reads a file into a string asynchronously
/// (This is now the primary implementation)
///
/// # Arguments
/// * `path` - Path to the file to read
///
/// # Returns
/// * `Ok(String)` - The file contents
/// * `Err(io::Error)` - If the file can't be read
pub async fn read_file_async(path: &str) -> Result<String, io::Error> {
    read_file(path).await // Just delegate
}

/// Check if a file exists (async)
pub async fn file_exists(path: &str) -> bool {
    match get_vfs().await {
        Ok(vfs) => {
            debug!("Got VFS instance. Calling vfs.exists...");
            match vfs.exists(path).await {
                Ok(exists) => {
                    debug!("VFS exists check for {} returned: {}", path, exists);
                    exists
                }
                Err(e) => {
                    warn!("VFS exists check failed for {}: {}", path, e);
                    false // If checking fails, assume it doesn't exist?
                }
            }
        }
        Err(e) => {
            error!("Failed to get VFS for existence check on {}: {}", path, e);
            false
        }
    }
}

/// Read the contents of a file as a string (async)
///
/// # Arguments
/// * `path` - Path to the file to read
/// * `base_path` - Optional base path for security checking (IGNORED in VFS context)
///
/// # Returns
/// * `Ok(String)` - The file contents
/// * `Err(io::Error)` - If the file can't be read
async fn file_get<P: AsRef<Path>>(path: P, _base_path: Option<&str>) -> io::Result<String> {
    let path_ref = path.as_ref();
    debug!("file_get called for path: {:?}", path_ref);
    // Convert path to &str
    let path_str = path_ref.to_str().ok_or_else(|| {
        error!("File path {:?} is not a valid UTF-8 string", path_ref);
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "File path is not a valid UTF-8 string",
        )
    })?;

    debug!("Delegating file_get to read_file for path: {}", path_str);
    // Use the async read_file function
    read_file(path_str).await
}

/// Copy a file from source to destination (async)
pub async fn copy_file(src: &str, dst: &str) -> io::Result<()> {
    debug!("copy_file called from src: {} to dst: {}", src, dst);
    debug!("Attempting to get VFS instance...");
    let vfs = get_vfs().await?;

    debug!("Reading source file {} using VFS...", src);
    let content_result = vfs.read_file(src).await;
    let content = match content_result {
        Ok(content) => {
            debug!(
                "Successfully read {} bytes from source: {}",
                content.len(),
                src
            );
            content
        }
        Err(e) => {
            warn!("Failed to read source file {} during copy: {:?}", src, e);
            return Err(map_vfs_error(e));
        }
    };

    debug!("Writing content to destination file {} using VFS...", dst);
    let write_result = vfs.write_file(dst, content).await;
    match write_result {
        Ok(_) => {
            debug!("Successfully wrote to destination: {}", dst);
            Ok(())
        }
        Err(e) => {
            error!("Failed to write to destination file {}: {:?}", dst, e);
            Err(map_vfs_error(e))
        }
    }
}

/// Async version of file_get that reads file contents asynchronously
/// (This is now the primary implementation)
///
/// # Arguments
/// * `path` - Path to the file to read
/// * `base_path` - Optional base path for security checking (IGNORED)
///
/// # Returns
/// * `Ok(String)` - The file contents
/// * `Err(io::Error)` - If the file can't be read
pub async fn file_get_async<P: AsRef<Path>>(
    path: P,
    base_path: Option<&str>,
) -> io::Result<String> {
    let path_ref = path.as_ref();
    // Delegate to the updated file_get
    file_get(path_ref, base_path).await
}

/// Get file attributes (async)
///
/// # Arguments
/// * `path` - Path to the file or directory
///
/// # Returns
/// * `Ok(FileAttributes)` - The file or directory attributes
/// * `Err(io::Error)` - If the attributes can't be read
pub async fn get_file_attributes(path: &str) -> io::Result<crate::vfs::FileAttributes> {
    debug!("get_file_attributes called for path: {}", path);
    debug!("Attempting to get VFS instance...");
    let vfs = get_vfs().await?;
    debug!("Got VFS instance. Calling read_file_attributes...");
    match vfs.read_file_attributes(path).await {
        Ok(attributes) => {
            debug!(
                "Successfully got attributes for path: {}, size: {}",
                path, attributes.size
            );
            Ok(attributes)
        }
        Err(e) => {
            warn!("VFS read_file_attributes failed for {}: {:?}", path, e);
            Err(map_vfs_error(e))
        }
    }
}

/// Create a directory (async)
///
/// # Arguments
/// * `path` - Path to the directory to create
///
/// # Returns
/// * `Ok(())` - If the directory was created successfully
/// * `Err(io::Error)` - If the directory can't be created
pub async fn create_directory(path: &str) -> io::Result<()> {
    debug!("create_directory called for path: {}", path);
    debug!("Attempting to get VFS instance...");
    let vfs = get_vfs().await?;
    debug!("Got VFS instance. Calling create_directory...");
    match vfs.create_directory(path).await {
        Ok(_) => {
            debug!("Successfully created directory: {}", path);
            Ok(())
        }
        Err(e) => {
            warn!("VFS create_directory failed for {}: {:?}", path, e);
            Err(map_vfs_error(e))
        }
    }
}
