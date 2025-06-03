use crate::utils::http_wasm::{web_get_async, ProxyConfig};
use crate::utils::string::normalize_dir_path;
use crate::utils::system::safe_system_time;
use crate::vfs::vercel_kv_github::GitHubTreeResponse;
use crate::vfs::vercel_kv_helpers::*;
use crate::vfs::vercel_kv_store::{create_directory_attributes, create_file_attributes};
use crate::vfs::vercel_kv_types::*;
use crate::vfs::vercel_kv_vfs::VercelKvVfs;
use crate::vfs::VfsError;
use case_insensitive_string::CaseInsensitiveString;
use futures::future::{join_all, BoxFuture, FutureExt};
use futures::stream::StreamExt;
use std::collections::{HashMap, HashSet};
use std::time::UNIX_EPOCH;

impl VercelKvVfs {
    /// Load all files from a GitHub repository directory
    pub(crate) async fn load_github_directory_impl(
        &self,
        shallow: bool,
        recursive: bool,
    ) -> Result<LoadDirectoryResult, VfsError> {
        log::info!(
            "Starting load_github_directory (shallow: {}, recursive: {})",
            shallow,
            recursive
        );

        // Add an env var check to allow triggering a test panic
        if std::option_env!("RUST_TEST_PANIC").is_some() {
            log::warn!("Triggering intentional panic for stack trace testing");
            panic!("This is an intentional test panic to verify stack trace capture");
        }

        log::info!("Loading all files from GitHub configured root path");

        // Generate cache key for this directory lookup
        let cache_key = get_github_tree_cache_key(
            &self.github_config.owner,
            &self.github_config.repo,
            &self.github_config.branch,
            recursive,
        );

        // Check if we have cached data
        let current_time = safe_system_time()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let mut response_text = None;

        if let Ok(Some(cache)) = self.store.read_github_tree_cache(&cache_key).await {
            if !cache.is_expired(current_time) {
                log::debug!("Using cached GitHub tree data from KV");
                response_text = Some(cache.data);
            } else {
                log::debug!("GitHub tree cache is expired");
            }
        }

        // If no valid cache, fetch from GitHub API
        if response_text.is_none() {
            // When recursive=0, API returns only direct children of the tree
            // When recursive=1, API returns all descendants recursively
            let api_url = format!(
                "https://api.github.com/repos/{}/{}/git/trees/{}?recursive={}",
                self.github_config.owner,
                self.github_config.repo,
                self.github_config.branch,
                if recursive { "1" } else { "0" }
            );

            log::debug!("Fetching GitHub directory tree from: {}", api_url);

            // Prepare headers with authorization if token is available
            let mut headers = HashMap::new();
            if let Some(token) = &self.github_config.auth_token {
                headers.insert(
                    CaseInsensitiveString::new("Authorization"),
                    format!("token {}", token),
                );
            }
            headers.insert(
                CaseInsensitiveString::new("Accept"),
                "application/vnd.github.v3+json".to_string(),
            );
            headers.insert(
                CaseInsensitiveString::new("User-Agent"),
                "subconverter-rs".to_string(),
            );

            // Make the request
            let proxy_config = ProxyConfig::default();
            let fetch_result = web_get_async(&api_url, &proxy_config, Some(&headers)).await;

            match fetch_result {
                Ok(response) => {
                    // Check if the response is successful (2xx)
                    if (200..300).contains(&response.status) {
                        log::debug!("Successfully fetched GitHub API response");

                        // Check if we got rate limit headers
                        if let Some(rate_limit) = response.headers.get("x-ratelimit-remaining") {
                            log::info!("GitHub API rate limit remaining: {}", rate_limit);
                        }

                        response_text = Some(response.body);

                        // Cache the result
                        let cache = GitHubTreeCache {
                            data: response_text.as_ref().unwrap().clone(),
                            created_at: current_time,
                            ttl: self.github_config.cache_ttl_seconds,
                        };

                        // Store cache in background
                        self.store
                            .write_github_tree_cache_background(cache_key.clone(), cache);
                    } else {
                        log::error!(
                            "GitHub API returned error status {}: {}",
                            response.status,
                            response.body
                        );
                        return Err(VfsError::NetworkError(format!(
                            "GitHub API returned error status {}: {}",
                            response.status, response.body
                        )));
                    }
                }
                Err(e) => {
                    log::error!("Error fetching GitHub API: {}", e.message);
                    return Err(VfsError::NetworkError(format!(
                        "GitHub API request failed: {}",
                        e.message
                    )));
                }
            }
        }

        // Parse the response to get file information
        let response_text = match response_text {
            Some(text) => text,
            None => {
                return Err(VfsError::NetworkError(
                    "Failed to get GitHub API response".to_string(),
                ))
            }
        };

        let tree_response: GitHubTreeResponse =
            match serde_json::from_str::<GitHubTreeResponse>(&response_text) {
                Ok(tree) => {
                    log::debug!(
                        "Successfully parsed GitHub tree JSON with {} items",
                        tree.tree.len()
                    );
                    tree
                }
                Err(e) => {
                    log::error!("Error parsing GitHub tree JSON: {:?}", e);
                    return Err(VfsError::Other(format!(
                        "Failed to parse GitHub tree JSON: {}",
                        e
                    )));
                }
            };

        // Check if the tree was truncated (too large)
        if tree_response.truncated {
            log::warn!("GitHub tree response was truncated. Some files might be missing.");
        }

        let root_path_prefix = if self.github_config.root_path.is_empty() {
            "".to_string()
        } else {
            format!("{}/", self.github_config.root_path.trim_matches('/'))
        };
        log::debug!("Root path prefix: '{}'", root_path_prefix);

        // Group files by their parent directory for batch updates
        let mut files_by_parent: HashMap<String, Vec<FileAttributes>> = HashMap::new();
        let mut directories_to_create = HashSet::new();

        for item in &tree_response.tree {
            // Handle both blob (file) and tree (directory) items
            let is_directory = item.type_field == "tree";
            if item.type_field != "blob" && !is_directory {
                continue; // Skip other item types
            }

            // Account for root_path from config
            let relative_path = if item.path.starts_with(&root_path_prefix) {
                item.path[root_path_prefix.len()..].to_string()
            } else {
                // Skip if not under the configured root path
                continue;
            };

            // Track all parent directories for this file or directory
            let mut current_parent_dir = get_parent_directory(&relative_path);
            while !current_parent_dir.is_empty() {
                log::trace!("Tracking parent directory: {}", current_parent_dir);
                directories_to_create.insert(current_parent_dir.clone());
                current_parent_dir = get_parent_directory(&current_parent_dir);
            }
            // Ensure root is tracked if any items exist
            directories_to_create.insert("".to_string());

            if is_directory {
                let current_dir_path = normalize_dir_path(&relative_path);
                log::trace!("Found directory from GitHub tree: {}", current_dir_path);
                directories_to_create.insert(current_dir_path.clone());

                // Create attributes for the directory itself and add to parent's map
                let parent_dir = get_parent_directory(&relative_path);
                let dirname = get_filename(&relative_path); // Get the directory name itself
                if !dirname.is_empty() {
                    // Avoid adding root dir to its own (non-existent) parent map
                    let attributes = create_directory_attributes(&relative_path, "cloud");
                    log::trace!(
                        "Adding directory attributes for '{}' to parent '{}' map",
                        relative_path,
                        parent_dir
                    );
                    files_by_parent
                        .entry(parent_dir)
                        .or_default()
                        .push(attributes);
                }
            } else {
                // It's a file
                log::trace!("Found file from GitHub tree: {}", relative_path);
                let parent_dir = get_parent_directory(&relative_path);
                let filename = get_filename(&relative_path);
                if filename.is_empty() {
                    continue;
                }

                let size_estimate = item.size.unwrap_or(0);
                let source_type = if shallow { "placeholder" } else { "cloud" };
                let attributes = create_file_attributes(&relative_path, size_estimate, source_type);

                files_by_parent
                    .entry(parent_dir)
                    .or_default()
                    .push(attributes);
            }
        }

        let total_files_found = files_by_parent.values().map(|v| v.len()).sum::<usize>();
        log::info!(
            "Found {} files across {} parent directories and {} total directories to create",
            total_files_found,
            files_by_parent.len(),
            directories_to_create.len()
        );

        // Create directory futures concurrently
        let directory_futures = directories_to_create
            .iter()
            .map(|dir| {
                let vfs = self.clone(); // Clone self for use in the async block
                let dir_clone = dir.clone();
                async move {
                    if dir_clone.is_empty() {
                        // Skip creating the actual root "" path, it implicitly exists.
                        // Still need to handle metadata below if required? Consider implications.
                        // For now, just skip the create call.
                        return Ok(());
                    }

                    log::debug!("Ensuring directory exists: {}", dir_clone);
                    let result = vfs.create_directory_impl(&dir_clone).await; // create_directory_impl handles existence check
                    if let Err(e) = result {
                        log::warn!("Failed to ensure directory {}: {:?}", dir_clone, e);
                        Err(VfsError::Other(format!(
                            "Failed to ensure directory {}: {:?}",
                            dir_clone, e
                        )))
                    } else {
                        // Cache directory attributes after ensuring it exists
                        let dir_attributes = create_directory_attributes(&dir_clone, "cloud");
                        vfs.store
                            .write_to_metadata_cache(&dir_clone, dir_attributes)
                            .await;
                        Ok(()) // Indicate success
                    }
                }
            })
            .collect::<Vec<_>>();

        log::info!(
            "Waiting for {} directory creation tasks...",
            directory_futures.len()
        );
        let dir_start_time = safe_system_time(); // Use safe_system_time
        let directory_results = join_all(directory_futures).await;
        let dir_duration = safe_system_time()
            .duration_since(dir_start_time)
            .unwrap_or_default(); // Calculate duration using SystemTime
        log::debug!("Directory creation tasks finished in {:.2?}", dir_duration);
        let dir_failures = directory_results.iter().filter(|r| r.is_err()).count();
        if dir_failures > 0 {
            log::warn!(
                "{} directory creation tasks failed (check previous logs).",
                dir_failures
            );
            // Decide if we should return an error or continue
            // For now, continue as before, but log clearly.
        }

        log::info!(
            "Processing {} files concurrently (shallow: {} / deep: {})",
            total_files_found,
            shallow,
            !shallow
        );

        // --- File Processing ---
        let mut final_loaded_files: Vec<LoadedFile> = Vec::new();
        let mut successes = 0;
        let mut failures = 0;

        if shallow {
            // --- Shallow Mode: Synchronous metadata updates per directory ---
            log::info!("Processing files in shallow mode (synchronous metadata update per dir)...");
            for (parent_dir, files) in files_by_parent {
                let vfs = self.clone(); // Clone VFS for this iteration
                let parent_dir_clone = parent_dir.clone();
                log::debug!(
                    "Shallow updating metadata for directory: {}",
                    parent_dir_clone
                );

                let mut current_batch_success = true; // Track success for this specific directory

                // Perform read-modify-write synchronously for this directory
                match vfs
                    .store
                    .read_directory_metadata_from_kv(&parent_dir_clone)
                    .await
                {
                    Ok(mut dir_metadata) => {
                        let mut files_in_batch = 0;
                        for file_attrs in files {
                            files_in_batch += 1;
                            // Add placeholder to final result list immediately
                            final_loaded_files.push(LoadedFile {
                                path: file_attrs.path.clone(),
                                size: file_attrs.size,
                                is_placeholder: true,
                                is_directory: false,
                            });
                            // Update the metadata map
                            dir_metadata
                                .files
                                .insert(get_filename(&file_attrs.path), file_attrs.clone());
                            // Update memory cache (still useful)
                            vfs.store
                                .write_to_metadata_cache(&file_attrs.path, file_attrs.clone())
                                .await;
                        }

                        // Write the updated metadata back SYNCHRONOUSLY
                        let write_result = vfs
                            .store
                            .write_directory_metadata_to_kv(&parent_dir_clone, &dir_metadata)
                            .await;

                        if let Err(e) = write_result {
                            log::error!(
                                "Failed to write updated metadata for dir '{}': {:?}",
                                parent_dir_clone,
                                e
                            );
                            current_batch_success = false;
                            failures += files_in_batch; // Mark all files in this batch as failed
                        } else {
                            successes += files_in_batch; // Mark all files as successful for this batch
                        }
                    }
                    Err(e) => {
                        log::error!("Failed to read initial metadata for dir '{}': {:?}. Skipping update for {} files.", parent_dir_clone, e, files.len());
                        current_batch_success = false;
                        failures += files.len(); // Count files as failures if initial read failed
                                                 // Add placeholders to results anyway, even if metadata write fails?
                                                 // Let's add them, as they exist conceptually.
                        for file_attrs in files {
                            final_loaded_files.push(LoadedFile {
                                path: file_attrs.path.clone(),
                                size: file_attrs.size,
                                is_placeholder: true,
                                is_directory: false,
                            });
                        }
                    }
                }
            }
        } else {
            // --- Deep Mode: Use buffer_unordered for concurrent reads and collect results ---
            log::info!("Processing files in deep mode using buffer_unordered...");
            const CONCURRENT_LIMIT: usize = 10;

            let file_paths_to_read: Vec<String> = files_by_parent
                .into_values()
                .flatten()
                .map(|attrs| attrs.path)
                .collect();

            let file_read_futures = file_paths_to_read.into_iter().map(|file_path| {
                let vfs = self.clone();
                async move {
                    // This future now returns Result<LoadedFile, VfsError>
                    log::debug!("Deep processing file: {}", file_path);
                    match vfs.read_file_impl(&file_path).await {
                        Ok(content) => Ok(LoadedFile {
                            path: file_path.clone(),
                            size: content.len(),
                            is_placeholder: false,
                            is_directory: false,
                        }),
                        Err(e) => {
                            log::warn!("Failed to deep load file {}: {:?}", file_path, e);
                            Err(e) // Propagate the error
                        }
                    }
                }
            });

            let stream =
                futures::stream::iter(file_read_futures).buffer_unordered(CONCURRENT_LIMIT);

            // Collect all results from the stream
            let results: Vec<Result<LoadedFile, VfsError>> = stream.collect().await;

            // Process the collected results
            for result in results {
                match result {
                    Ok(loaded_file) => {
                        successes += 1;
                        final_loaded_files.push(loaded_file); // Add successful results to the final vec
                    }
                    Err(_) => {
                        failures += 1;
                    }
                }
            }
        }

        log::info!(
            "Finished processing files: {} successes initiated/completed, {} failures detected",
            successes,
            failures
        );

        // Add successfully created/ensured directories to the result
        for dir in &directories_to_create {
            if !dir.is_empty() {
                let dir_path_with_slash = normalize_dir_path(dir);
                if self
                    .store
                    .exists_in_metadata_cache(&dir_path_with_slash)
                    .await
                {
                    // Push directly to the final vec
                    final_loaded_files.push(LoadedFile {
                        path: dir_path_with_slash,
                        size: 0,
                        is_placeholder: false,
                        is_directory: true,
                    });
                } else {
                    log::debug!("Skipping directory {} in results as it might have failed creation or not cached.", dir_path_with_slash);
                }
            }
        }
        if directories_to_create.contains("") {
            // Push directly to the final vec
            final_loaded_files.push(LoadedFile {
                path: "".to_string(),
                size: 0,
                is_placeholder: false,
                is_directory: true,
            });
        }

        Ok(LoadDirectoryResult {
            total_files: total_files_found,
            successful_files: successes,
            failed_files: failures,
            loaded_files: final_loaded_files, // Use the collected results
        })
    }

    /// Load information about a specific file from GitHub without downloading content
    /// This still relies on the GitHub Tree Cache, which is independent of the new metadata storage.
    pub(crate) async fn load_github_file_info_impl(
        &self,
        file_path: &str,
    ) -> Result<LoadedFile, VfsError> {
        log::debug!("Loading GitHub file info for: {}", file_path);

        let normalized_path = normalize_path(file_path);

        // Normalize the path for GitHub API
        let api_path = if normalized_path.starts_with('/') {
            normalized_path[1..].to_string()
        } else {
            normalized_path.clone()
        };

        // Account for root_path from config
        let api_path_with_root = if self.github_config.root_path.is_empty() {
            api_path
        } else {
            let root_path = self.github_config.root_path.trim_matches('/');
            format!("{}/{}", root_path, api_path)
        };

        // Cache key for GitHub tree API
        let cache_key = get_github_tree_cache_key(
            &self.github_config.owner,
            &self.github_config.repo,
            &self.github_config.branch,
            true, // Always use recursive tree for file info
        );

        // Check if we have cached data
        let current_time = safe_system_time()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let mut response_text = None;

        if let Ok(Some(cache)) = self.store.read_github_tree_cache(&cache_key).await {
            if !cache.is_expired(current_time) {
                log::debug!("Using cached GitHub tree data for file info");
                response_text = Some(cache.data);
            } else {
                log::debug!("GitHub tree cache is expired");
            }
        }

        // If no valid cache, fetch from GitHub API
        if response_text.is_none() {
            // Create GitHub API URL to get file info
            // Use the trees API to get file size without downloading the content
            let url = format!(
                "https://api.github.com/repos/{owner}/{repo}/git/trees/{branch}?recursive=1",
                owner = self.github_config.owner,
                repo = self.github_config.repo,
                branch = self.github_config.branch
            );

            log::debug!("Fetching GitHub tree from: {}", url);

            // Prepare headers with authorization if token is available
            let mut headers = HashMap::new();
            if let Some(token) = &self.github_config.auth_token {
                headers.insert(
                    CaseInsensitiveString::new("Authorization"),
                    format!("token {}", token),
                );
            }
            headers.insert(
                CaseInsensitiveString::new("Accept"),
                "application/vnd.github.v3+json".to_string(),
            );
            headers.insert(
                CaseInsensitiveString::new("User-Agent"),
                "subconverter-rs".to_string(),
            );

            // Make the request
            let proxy_config = ProxyConfig::default();
            let fetch_result = web_get_async(&url, &proxy_config, Some(&headers)).await;

            match fetch_result {
                Ok(response) => {
                    // Check if the response is successful (2xx)
                    if (200..300).contains(&response.status) {
                        log::debug!("Successfully fetched GitHub API response for file info");

                        // Check if we got rate limit headers
                        if let Some(rate_limit) = response.headers.get("x-ratelimit-remaining") {
                            log::info!("GitHub API rate limit remaining: {}", rate_limit);
                        }

                        response_text = Some(response.body);

                        // Cache the result
                        let cache = GitHubTreeCache {
                            data: response_text.as_ref().unwrap().clone(),
                            created_at: current_time,
                            ttl: self.github_config.cache_ttl_seconds,
                        };

                        // Store cache in background
                        self.store
                            .write_github_tree_cache_background(cache_key.clone(), cache);
                    } else {
                        log::error!(
                            "GitHub API returned error status {}: {}",
                            response.status,
                            response.body
                        );
                        return Err(VfsError::NetworkError(format!(
                            "GitHub API returned error status {}: {}",
                            response.status, response.body
                        )));
                    }
                }
                Err(e) => {
                    log::error!("Error fetching GitHub API for file info: {}", e.message);
                    return Err(VfsError::NetworkError(format!(
                        "GitHub API request failed: {}",
                        e.message
                    )));
                }
            }
        }

        // Parse the response to get file information
        let response_text = match response_text {
            Some(text) => text,
            None => {
                return Err(VfsError::NetworkError(
                    "Failed to get GitHub API response".to_string(),
                ))
            }
        };

        // Parse the tree response
        let tree_response: GitHubTreeResponse =
            match serde_json::from_str::<GitHubTreeResponse>(&response_text) {
                Ok(tree) => tree,
                Err(e) => {
                    return Err(VfsError::Other(format!(
                        "Failed to parse GitHub tree JSON: {}",
                        e
                    )))
                }
            };

        // Find the file in the tree
        for item in &tree_response.tree {
            // Skip directories
            if item.type_field != "blob" {
                continue;
            }

            // Check if this is the file we're looking for
            if item.path == api_path_with_root {
                // Found the file, get its size
                let size = item.size.unwrap_or(0);

                log::debug!(
                    "Found file in GitHub tree: {} with size {}",
                    item.path,
                    size
                );

                return Ok(LoadedFile {
                    path: normalized_path,
                    size,
                    is_placeholder: false, // This function effectively loads the info, so not placeholder
                    is_directory: false,
                });
            }
        }

        // File not found in the tree
        log::debug!("File not found in GitHub tree: {}", api_path_with_root);
        Err(VfsError::NotFound(format!(
            "File not found in GitHub repo: {}",
            file_path
        )))
    }
}
