use crate::vfs::VfsError;
use serde::Deserialize;

/// GitHub API tree response structure
#[derive(Debug, Deserialize)]
pub struct GitHubTreeResponse {
    pub tree: Vec<GitHubTreeItem>,
    pub truncated: bool,
}

/// GitHub API tree item structure
#[derive(Debug, Deserialize, Clone)]
pub struct GitHubTreeItem {
    pub path: String,
    #[serde(rename = "type")]
    pub type_field: String,
    pub size: Option<usize>,
}

// Configuration for GitHub raw content source
#[derive(Clone, Debug)]
pub struct GitHubConfig {
    pub owner: String,
    pub repo: String,
    pub branch: String,
    pub root_path: String,
    pub auth_token: Option<String>,
    pub cache_ttl_seconds: u64,
}

impl GitHubConfig {
    pub fn from_env() -> Result<Self, VfsError> {
        #[cfg(target_arch = "wasm32")]
        {
            use crate::vfs::vercel_kv_js_bindings::getenv;

            Ok(Self {
                owner: getenv("VFS_GITHUB_OWNER", "lonelam"),
                repo: getenv("VFS_GITHUB_REPO", "subconverter-rs"),
                branch: getenv("VFS_GITHUB_BRANCH", "main"),
                root_path: getenv("VFS_GITHUB_ROOT_PATH", "base"),
                auth_token: {
                    let token = getenv("GITHUB_TOKEN", "");
                    if token.is_empty() {
                        None
                    } else {
                        Some(token)
                    }
                },
                // Default cache TTL is 15 minutes (can be overridden with environment variable)
                cache_ttl_seconds: {
                    let ttl_str = getenv("GITHUB_CACHE_TTL", "900");
                    ttl_str.parse().unwrap_or(900)
                },
            })
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            Ok(Self {
                owner: std::env::var("VFS_GITHUB_OWNER").unwrap_or_else(|_| "lonelam".to_string()),
                repo: std::env::var("VFS_GITHUB_REPO")
                    .unwrap_or_else(|_| "subconverter-rs".to_string()),
                branch: std::env::var("VFS_GITHUB_BRANCH").unwrap_or_else(|_| "main".to_string()),
                root_path: std::env::var("VFS_GITHUB_ROOT_PATH")
                    .unwrap_or_else(|_| "base".to_string()),
                auth_token: std::env::var("GITHUB_TOKEN").ok(),
                // Default cache TTL is 15 minutes (can be overridden with environment variable)
                cache_ttl_seconds: std::env::var("GITHUB_CACHE_TTL")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(900),
            })
        }
    }

    pub fn get_raw_url(&self, file_path: &str) -> String {
        let base = format!(
            "https://raw.githubusercontent.com/{}/{}/{}",
            self.owner, self.repo, self.branch
        );
        let full_path = if self.root_path.is_empty() {
            file_path.to_string()
        } else {
            format!("{}/{}", self.root_path.trim_matches('/'), file_path)
        };
        format!("{}/{}", base, full_path.trim_start_matches('/'))
    }

    pub fn get_api_url(&self, endpoint: &str) -> String {
        format!(
            "https://api.github.com/repos/{}/{}{}",
            self.owner, self.repo, endpoint
        )
    }
}
