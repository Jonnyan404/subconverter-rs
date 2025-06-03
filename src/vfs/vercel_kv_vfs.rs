use super::VirtualFileSystem;
use crate::vfs::vercel_kv_github::GitHubConfig;
use crate::vfs::vercel_kv_store::VercelKvStore;
use crate::vfs::vercel_kv_types::*;
use crate::vfs::VfsError;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct VercelKvVfs {
    pub(crate) store: Arc<VercelKvStore>,
    pub(crate) github_config: GitHubConfig,
}

impl VercelKvVfs {
    pub fn new() -> Result<Self, VfsError> {
        // Initialize GitHub config from environment
        let github_config = match GitHubConfig::from_env() {
            Ok(config) => {
                log::info!(
                    "Initialized GitHub config: repo={}/{}, branch={}, root_path={}",
                    config.owner,
                    config.repo,
                    config.branch,
                    config.root_path
                );
                config
            }
            Err(e) => {
                log::warn!("Failed to initialize GitHub config: {}", e);
                return Err(e);
            }
        };

        Ok(Self {
            store: Arc::new(VercelKvStore::new()),
            github_config,
        })
    }

    // Internal helper to get memory cache from store
    pub(crate) fn memory_cache(&self) -> Arc<RwLock<HashMap<String, Vec<u8>>>> {
        self.store.get_memory_cache()
    }

    // Internal helper to get metadata cache from store
    pub(crate) fn metadata_cache(&self) -> Arc<RwLock<HashMap<String, FileAttributes>>> {
        self.store.get_metadata_cache()
    }

    /// Internal implementation for initializing GitHub load.
    /// Checks if root directory metadata exists and triggers load if not.
    /// Returns true if load was triggered, false otherwise.
    pub(crate) async fn initialize_github_load_impl(&self) -> Result<bool, VfsError> {
        log::debug!("Checking if GitHub load initialization is needed for root...");
        let mut github_load_triggered = false;

        // Check cache first
        let cached_metadata_exists = self.store.exists_in_metadata_cache("").await; // Check cache for root
        let mut skip_github_due_to_cache = false;
        if cached_metadata_exists {
            // If cached, assume it's populated enough, skip GitHub load trigger.
            log::debug!("Root directory metadata found in cache, skipping GitHub load trigger.");
            skip_github_due_to_cache = true;
        }

        // Check KV store if not found in cache
        if !skip_github_due_to_cache {
            match self.store.read_directory_metadata_from_kv("").await {
                Ok(metadata) if !metadata.files.is_empty() => {
                    log::debug!("Root directory metadata found in KV and is not empty, skipping GitHub load trigger.");
                    // No need to skip explicitly, load won't happen below
                }
                Ok(_) => {
                    log::debug!(
                        "Root directory metadata in KV is empty or missing. Triggering load..."
                    );
                    // Use shallow=true, recursive=true for initial population
                    match self.load_github_directory_impl(true, true).await {
                        Ok(_) => {
                            log::info!("GitHub load triggered successfully for root directory during initialization.");
                            github_load_triggered = true;
                        }
                        Err(load_err) => {
                            log::error!(
                                "GitHub load trigger failed during initialization: {:?}. VFS might be empty.",
                                load_err
                            );
                            return Err(load_err); // Propagate the error if initialization fails
                        }
                    }
                }
                Err(e) => {
                    log::warn!("Error checking root metadata in KV during initialization: {:?}. Attempting load anyway.", e);
                    // Attempt load even if the check failed
                    match self.load_github_directory_impl(true, true).await {
                        Ok(_) => {
                            log::info!("GitHub load triggered successfully for root directory after KV check error.");
                            github_load_triggered = true;
                        }
                        Err(load_err) => {
                            log::error!(
                                "GitHub load trigger failed after KV check error: {:?}. VFS might be empty.",
                                load_err
                            );
                            return Err(load_err); // Propagate the error
                        }
                    }
                }
            }
        }

        log::debug!(
            "GitHub load initialization check complete. Triggered: {}",
            github_load_triggered
        );
        Ok(github_load_triggered)
    }
}

impl VirtualFileSystem for VercelKvVfs {
    fn read_file(
        &self,
        path: &str,
    ) -> impl std::future::Future<Output = Result<Vec<u8>, VfsError>> {
        async move { self.read_file_impl(path).await }
    }

    fn write_file(
        &self,
        path: &str,
        content: Vec<u8>,
    ) -> impl std::future::Future<Output = Result<(), VfsError>> {
        async move { self.write_file_impl(path, content).await }
    }

    fn exists(&self, path: &str) -> impl std::future::Future<Output = Result<bool, VfsError>> {
        async move { self.exists_impl(path).await }
    }

    fn delete_file(&self, path: &str) -> impl std::future::Future<Output = Result<(), VfsError>> {
        async move { self.delete_file_impl(path).await }
    }

    fn read_file_attributes(
        &self,
        path: &str,
    ) -> impl std::future::Future<Output = Result<FileAttributes, VfsError>> {
        async move { self.read_file_attributes_impl(path).await }
    }

    fn list_directory(
        &self,
        path: &str,
    ) -> impl std::future::Future<Output = Result<Vec<DirectoryEntry>, VfsError>> {
        async move { self.list_directory_impl(path, false).await }
    }

    fn list_directory_skip_github(
        &self,
        path: &str,
    ) -> impl std::future::Future<Output = Result<Vec<DirectoryEntry>, VfsError>> {
        async move { self.list_directory_impl(path, true).await }
    }

    fn create_directory(
        &self,
        path: &str,
    ) -> impl std::future::Future<Output = Result<(), VfsError>> {
        async move { self.create_directory_impl(path).await }
    }

    fn load_github_directory(
        &self,
        _directory_path: &str,
        shallow: bool,
    ) -> impl std::future::Future<Output = Result<LoadDirectoryResult, VfsError>> {
        async move { self.load_github_directory_impl(shallow, true).await }
    }

    fn load_github_directory_flat(
        &self,
        _directory_path: &str,
        shallow: bool,
    ) -> impl std::future::Future<Output = Result<LoadDirectoryResult, VfsError>> {
        async move { self.load_github_directory_impl(shallow, false).await }
    }

    fn initialize_github_load(&self) -> impl std::future::Future<Output = Result<bool, VfsError>> {
        async move { self.initialize_github_load_impl().await }
    }
}
