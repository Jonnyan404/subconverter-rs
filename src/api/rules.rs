use crate::utils::http_wasm::{web_get_async, HttpResponse, ProxyConfig};
use crate::utils::ini_reader::IniReader;
use crate::vfs::vercel_kv_github::{GitHubConfig, GitHubTreeResponse};
use crate::vfs::{VfsError, VirtualFileSystem};
use case_insensitive_string::CaseInsensitiveString;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Serialize, Deserialize)]
pub struct RulesUpdateRequest {
    pub config_path: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RulesUpdateResult {
    success: bool,
    message: String,
    details: HashMap<String, RepoUpdateResult>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RepoUpdateResult {
    repo_name: String,
    files_updated: Vec<String>,
    errors: Vec<String>,
    status: String,
}

/// Update rules from various GitHub repositories based on configuration
pub async fn update_rules(req: Option<RulesUpdateRequest>) -> Result<HttpResponse, String> {
    log::info!("Handling rules update request");

    // Default config path if not specified
    let config_path = match &req {
        Some(r) => r
            .config_path
            .clone()
            .unwrap_or_else(|| "base/rules_config.conf".to_string()),
        None => "base/rules_config.conf".to_string(),
    };

    // Read the config file
    let vfs = crate::utils::file_wasm::get_vfs()
        .await
        .map_err(|e| format!("Failed to get VFS: {}", e))?;
    let config_content = match vfs.read_file(&config_path).await {
        Ok(content) => {
            // Convert Vec<u8> to String for INI parsing
            String::from_utf8(content)
                .map_err(|e| format!("Invalid UTF-8 in config file: {}", e))?
        }
        Err(e) => {
            log::error!("Failed to read config file {}: {:?}", config_path, e);

            // Try to use default configuration file from www directory
            let default_config_path = "default.conf";
            match vfs.read_file(default_config_path).await {
                Ok(content) => {
                    log::info!("Using default config file from {}", default_config_path);
                    String::from_utf8(content)
                        .map_err(|e| format!("Invalid UTF-8 in default config: {}", e))?
                }
                Err(e2) => {
                    log::error!("Failed to read default config file: {:?}", e2);
                    return Err(format!(
                        "Config file not found: {} and default config also not available",
                        config_path
                    ));
                }
            }
        }
    };

    // Parse the INI config using our custom IniReader
    let mut ini_reader = IniReader::new();
    match ini_reader.parse(&config_content) {
        Ok(_) => {
            log::info!(
                "Successfully parsed config file with {} sections",
                ini_reader.section_count()
            );
        }
        Err(e) => {
            log::error!("Failed to parse config file: {:?}", e);
            return Err(format!("Invalid config file format: {}", e));
        }
    }

    // Process each section in the config
    let mut result = RulesUpdateResult {
        success: true,
        message: "Rules update completed".to_string(),
        details: HashMap::new(),
    };

    // 先收集所有部分名称，避免借用冲突
    let section_names: Vec<String> = ini_reader
        .get_section_names()
        .into_iter()
        .filter(|name| !name.is_empty())
        .cloned() // 创建所有权字符串的副本
        .collect();

    // 处理每个部分
    for section_name in section_names {
        log::info!("Processing section: {}", section_name);

        // Enter the section to read its contents
        if let Err(e) = ini_reader.enter_section(&section_name) {
            log::error!("Failed to enter section {}: {:?}", section_name, e);
            continue;
        }

        // Extract section configuration
        let repo_name = ini_reader.get_current("name");
        let repo_name = if repo_name.is_empty() {
            &section_name
        } else {
            &repo_name
        };

        let url = ini_reader.get_current("url");
        if url.is_empty() {
            log::warn!("Missing URL in section {}, skipping", section_name);
            continue;
        }

        let branch = ini_reader.get_current("branch");
        let branch = if branch.is_empty() { "main" } else { &branch };

        let _commit = ini_reader.get_current("commit");

        let match_patterns_str = ini_reader.get_current("match");
        if match_patterns_str.is_empty() {
            log::warn!(
                "Missing match patterns in section {}, skipping",
                section_name
            );
            continue;
        }
        let match_patterns: Vec<String> = match_patterns_str
            .split('|')
            .map(|s| s.trim().to_string())
            .collect();

        let dest_path = ini_reader.get_current("dest");
        let dest_path = if dest_path.is_empty() {
            format!("base/rules/{}", repo_name)
        } else {
            dest_path
        };

        let keep_tree_str = ini_reader.get_current("keep_tree");
        let keep_tree = keep_tree_str.is_empty() || keep_tree_str == "true";

        // Create repo result entry
        let mut repo_result = RepoUpdateResult {
            repo_name: repo_name.to_string(),
            files_updated: Vec::new(),
            errors: Vec::new(),
            status: "processing".to_string(),
        };

        // Create a GitHubConfig for this repository
        let github_config = GitHubConfig {
            owner: extract_owner_from_url(&url).unwrap_or_else(|| "unknown".to_string()),
            repo: extract_repo_from_url(&url).unwrap_or_else(|| "unknown".to_string()),
            branch: branch.to_string(),
            root_path: "".to_string(), // We'll handle paths manually
            auth_token: None,          // Could add token support in the future
            cache_ttl_seconds: 60,     // Short cache TTL for this operation
        };

        log::info!(
            "Fetching from GitHub: owner={}, repo={}, branch={}",
            github_config.owner,
            github_config.repo,
            github_config.branch
        );

        // Fetch the repository files
        match fetch_repo_files(&github_config).await {
            Ok(tree_response) => {
                // Process matching files
                let updates = process_matching_files(
                    &tree_response,
                    &match_patterns,
                    &dest_path,
                    keep_tree,
                    &github_config,
                )
                .await;

                // Update result
                for (path, success) in updates {
                    if success {
                        repo_result.files_updated.push(path);
                    } else {
                        repo_result
                            .errors
                            .push(format!("Failed to update {}", path));
                    }
                }

                if repo_result.errors.is_empty() {
                    repo_result.status = "success".to_string();
                } else if repo_result.files_updated.is_empty() {
                    repo_result.status = "failed".to_string();
                    result.success = false;
                } else {
                    repo_result.status = "partial".to_string();
                    result.success = false;
                }
            }
            Err(e) => {
                log::error!("Failed to fetch repository {}: {:?}", repo_name, e);
                repo_result
                    .errors
                    .push(format!("Failed to fetch repository: {}", e));
                repo_result.status = "failed".to_string();
                result.success = false;
            }
        }

        // Add repo result to overall results
        result.details.insert(section_name.to_string(), repo_result);
    }

    // Set overall message
    if result.success {
        result.message = "All rules updated successfully".to_string();
    } else {
        result.message =
            "Some rule updates failed. Check details for more information.".to_string();
    }

    // Return result as JSON
    let result_json = serde_json::to_string(&result).unwrap_or_else(|_| "{}".to_string());
    Ok(HttpResponse {
        status: if result.success { 200 } else { 206 }, // OK or Partial Content
        body: result_json,
        headers: {
            let mut headers = HashMap::new();
            headers.insert("Content-Type".to_string(), "application/json".to_string());
            headers
        },
    })
}

async fn fetch_repo_files(config: &GitHubConfig) -> Result<GitHubTreeResponse, String> {
    // Create GitHub API URL
    let api_url = format!(
        "https://api.github.com/repos/{}/{}/git/trees/{}?recursive=1",
        config.owner, config.repo, config.branch
    );

    log::debug!("Fetching GitHub tree from: {}", api_url);

    // Prepare headers
    let mut headers = HashMap::new();
    if let Some(token) = &config.auth_token {
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
                // Parse the JSON response
                match serde_json::from_str::<GitHubTreeResponse>(&response.body) {
                    Ok(tree) => Ok(tree),
                    Err(e) => Err(format!("Failed to parse GitHub tree JSON: {}", e)),
                }
            } else {
                Err(format!(
                    "GitHub API returned error status {}: {}",
                    response.status, response.body
                ))
            }
        }
        Err(e) => Err(format!("GitHub API request failed: {}", e.message)),
    }
}

async fn process_matching_files(
    tree: &GitHubTreeResponse,
    patterns: &[String],
    dest_path: &str,
    keep_tree: bool,
    config: &GitHubConfig,
) -> Vec<(String, bool)> {
    let mut results = Vec::new();
    let vfs = crate::utils::file_wasm::get_vfs()
        .await
        .expect("Failed to get VFS");

    // Create base destination directory
    if let Err(e) = vfs.create_directory(dest_path).await {
        // Properly handle error conditions appropriately
        // VfsError doesn't have AlreadyExists variant, check if it's a different error type
        // For now, let's check if it's IoError with kind AlreadyExists or Other error with message containing "already exists"
        let is_already_exists = match &e {
            VfsError::IoError(io_err) if io_err.kind() == std::io::ErrorKind::AlreadyExists => true,
            VfsError::Other(msg) if msg.to_lowercase().contains("already exists") => true,
            VfsError::Other(msg) if msg.to_lowercase().contains("already") => true,
            _ => false,
        };

        if is_already_exists {
            // This is fine, directory already exists
            log::debug!("Directory already exists: {}", dest_path);
        } else {
            log::error!(
                "Failed to create destination directory {}: {:?}",
                dest_path,
                e
            );
            return vec![(dest_path.to_string(), false)];
        }
    }

    for item in &tree.tree {
        // Only process files
        if item.type_field != "blob" {
            continue;
        }

        let file_path = &item.path;

        // Check if this file matches any of our patterns
        if !matches_any_pattern(file_path, patterns) {
            continue;
        }

        log::info!("Found matching file: {}", file_path);

        // Determine destination path
        let file_dest_path = if keep_tree {
            // Keep directory structure
            let mut full_dest = dest_path.to_string();
            if !full_dest.ends_with('/') {
                full_dest.push('/');
            }
            full_dest.push_str(file_path);
            full_dest
        } else {
            // Just keep filename
            let filename = Path::new(file_path)
                .file_name()
                .map(|f| f.to_string_lossy().to_string())
                .unwrap_or_else(|| file_path.clone());

            let mut full_dest = dest_path.to_string();
            if !full_dest.ends_with('/') {
                full_dest.push('/');
            }
            full_dest.push_str(&filename);
            full_dest
        };

        // Create parent directories if needed
        if keep_tree {
            let parent_dir = Path::new(&file_dest_path)
                .parent()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default();

            if !parent_dir.is_empty() {
                if let Err(e) = vfs.create_directory(&parent_dir).await {
                    // 同样处理已存在目录的情况
                    let is_already_exists = match &e {
                        VfsError::IoError(io_err)
                            if io_err.kind() == std::io::ErrorKind::AlreadyExists =>
                        {
                            true
                        }
                        VfsError::Other(msg) if msg.to_lowercase().contains("already exists") => {
                            true
                        }
                        VfsError::Other(msg) if msg.to_lowercase().contains("already") => true,
                        _ => false,
                    };

                    if is_already_exists {
                        // This is fine, directory already exists
                        log::debug!("Directory already exists: {}", parent_dir);
                    } else {
                        log::error!("Failed to create directory {}: {:?}", parent_dir, e);
                        results.push((file_dest_path, false));
                        continue;
                    }
                }
            }
        }

        // Fetch file content
        let file_url = format!(
            "https://raw.githubusercontent.com/{}/{}/{}/{}",
            config.owner, config.repo, config.branch, file_path
        );

        match fetch_file_content(&file_url).await {
            Ok(content) => {
                // Write file - 转换 String 为 Vec<u8>
                match vfs.write_file(&file_dest_path, content.into_bytes()).await {
                    Ok(_) => {
                        log::info!("Successfully wrote file: {}", file_dest_path);
                        results.push((file_dest_path, true));
                    }
                    Err(e) => {
                        log::error!("Failed to write file {}: {:?}", file_dest_path, e);
                        results.push((file_dest_path, false));
                    }
                }
            }
            Err(e) => {
                log::error!("Failed to fetch file content {}: {}", file_path, e);
                results.push((file_dest_path, false));
            }
        }
    }

    results
}

fn matches_any_pattern(file_path: &str, patterns: &[String]) -> bool {
    for pattern in patterns {
        // Simple glob-style pattern matching
        if pattern_matches(pattern, file_path) {
            return true;
        }
    }
    false
}

fn pattern_matches(pattern: &str, path: &str) -> bool {
    // Convert glob pattern to regex
    // This is a simplified implementation - for a production system you might want a real glob library
    let pattern_parts: Vec<&str> = pattern.split('/').collect();
    let path_parts: Vec<&str> = path.split('/').collect();

    // Check if the pattern depth matches the path depth
    if pattern_parts.contains(&"**") {
        // ** means any level of directories
        true
    } else if pattern_parts.len() != path_parts.len() && !pattern.ends_with("/**") {
        false
    } else {
        // Check each part
        for (i, pattern_part) in pattern_parts.iter().enumerate() {
            if pattern_part == &"**" {
                return true; // Match everything under this
            }

            if i >= path_parts.len() {
                return false;
            }

            if pattern_part != &"*" && *pattern_part != path_parts[i] && !pattern_part.contains('*')
            {
                return false;
            }

            // Simple wildcard handling
            if pattern_part.contains('*') && pattern_part != &"**" {
                let re_pattern = pattern_part.replace('*', ".*");
                let re = regex::Regex::new(&format!("^{}$", re_pattern))
                    .unwrap_or_else(|_| regex::Regex::new(".*").unwrap());
                if !re.is_match(path_parts[i]) {
                    return false;
                }
            }
        }
        true
    }
}

async fn fetch_file_content(url: &str) -> Result<String, String> {
    log::debug!("Fetching file from: {}", url);

    let proxy_config = ProxyConfig::default();
    let fetch_result = web_get_async(url, &proxy_config, None).await;

    match fetch_result {
        Ok(response) => {
            if (200..300).contains(&response.status) {
                Ok(response.body)
            } else {
                Err(format!(
                    "HTTP error: status {}: {}",
                    response.status, response.body
                ))
            }
        }
        Err(e) => Err(format!("Request failed: {}", e.message)),
    }
}

fn extract_owner_from_url(url: &str) -> Option<String> {
    // Extract owner from GitHub URL
    // Example: https://github.com/ACL4SSR/ACL4SSR
    if !url.contains("github.com") {
        return None;
    }

    let parts: Vec<&str> = url.split('/').collect();
    if parts.len() < 4 {
        return None;
    }

    // Find the index of "github.com"
    let github_index = parts
        .iter()
        .position(|&p| p == "github.com" || p.ends_with("github.com"))?;
    if github_index + 1 < parts.len() {
        Some(parts[github_index + 1].to_string())
    } else {
        None
    }
}

fn extract_repo_from_url(url: &str) -> Option<String> {
    // Extract repo from GitHub URL
    // Example: https://github.com/ACL4SSR/ACL4SSR
    if !url.contains("github.com") {
        return None;
    }

    let parts: Vec<&str> = url.split('/').collect();
    if parts.len() < 5 {
        return None;
    }

    // Find the index of "github.com"
    let github_index = parts
        .iter()
        .position(|&p| p == "github.com" || p.ends_with("github.com"))?;
    if github_index + 2 < parts.len() {
        Some(parts[github_index + 2].to_string())
    } else {
        None
    }
}
