use std::{path::PathBuf, sync::Arc};

use tauri::State;

use crate::{
    entities::{Repository, VCSProviderType},
    services::{AppState, RepositoryService},
};

/// Lists all registered repositories
#[tauri::command]
pub async fn list_repositories(state: State<'_, Arc<AppState>>) -> Result<Vec<Repository>, String> {
    state
        .repository_service
        .list()
        .await
        .map_err(|e| e.to_string())
}

/// Gets a repository by ID
#[tauri::command]
pub async fn get_repository(
    state: State<'_, Arc<AppState>>,
    id: String,
) -> Result<Option<Repository>, String> {
    state
        .repository_service
        .get(&id)
        .await
        .map_err(|e| e.to_string())
}

/// Adds a repository from local path
#[tauri::command]
pub async fn add_repository(
    state: State<'_, Arc<AppState>>,
    path: String,
) -> Result<Repository, String> {
    let path_buf = PathBuf::from(&path);

    // Canonicalize to resolve symlinks and get absolute path
    let canonical_path = path_buf
        .canonicalize()
        .map_err(|e| format!("Failed to resolve path '{}': {}", path, e))?;
    let canonical_path_str = canonical_path.to_string_lossy().to_string();

    // Check if already registered
    if state
        .repository_service
        .exists_by_path(&canonical_path_str)
        .await
        .map_err(|e| e.to_string())?
    {
        return Err("Repository already registered".to_string());
    }

    // Detect repository info from path
    let (remote_url, name, provider) = RepositoryService::detect_from_path(&canonical_path).ok_or(
        "Could not detect repository info. Is this a valid git repository with an 'origin' remote?",
    )?;

    let default_branch = RepositoryService::detect_default_branch(&canonical_path);

    let repo = Repository::new(
        uuid::Uuid::new_v4().to_string(),
        name,
        canonical_path_str,
        remote_url,
        provider,
    )
    .with_default_branch(default_branch);

    state
        .repository_service
        .create(&repo)
        .await
        .map_err(|e| e.to_string())?;

    Ok(repo)
}

/// Removes a repository (does not delete files)
#[tauri::command]
pub async fn remove_repository(state: State<'_, Arc<AppState>>, id: String) -> Result<(), String> {
    state
        .repository_service
        .delete(&id)
        .await
        .map_err(|e| e.to_string())
}

/// Adds a repository from a remote URL (for server mode)
/// This is used when the desktop app is connected to a remote server
/// and local paths are not available
#[tauri::command]
pub async fn add_repository_by_url(
    state: State<'_, Arc<AppState>>,
    remote_url: String,
    default_branch: Option<String>,
) -> Result<Repository, String> {
    // Validate URL format
    validate_git_url(&remote_url)?;

    // Parse repository info from URL
    let (name, provider) = parse_repository_url(&remote_url)?;

    // Check if already registered by remote URL
    if state
        .repository_service
        .exists_by_remote_url(&remote_url)
        .await
        .map_err(|e| e.to_string())?
    {
        return Err("Repository already registered".to_string());
    }

    // Validate default branch if provided
    let branch = default_branch.unwrap_or_else(|| "main".to_string());
    validate_branch_name(&branch)?;

    let repo = Repository::new(
        uuid::Uuid::new_v4().to_string(),
        name,
        String::new(), // Empty local path for server mode
        remote_url,
        provider,
    )
    .with_default_branch(branch);

    state
        .repository_service
        .create(&repo)
        .await
        .map_err(|e| e.to_string())?;

    Ok(repo)
}

/// Validates that a URL is a valid git repository URL
fn validate_git_url(url: &str) -> Result<(), String> {
    let url = url.trim();

    if url.is_empty() {
        return Err("URL cannot be empty".to_string());
    }

    // Check for common dangerous patterns
    if url.contains("..") || url.contains('\0') || url.contains('\n') || url.contains('\r') {
        return Err("URL contains invalid characters".to_string());
    }

    // Check for supported URL schemes
    let is_https = url.starts_with("https://");
    let is_http = url.starts_with("http://");
    let is_ssh = url.starts_with("git@") || url.starts_with("ssh://");

    if !is_https && !is_http && !is_ssh {
        return Err(
            "Invalid URL scheme. Supported schemes: https://, http://, git@, ssh://".to_string(),
        );
    }

    // Basic URL structure validation for HTTPS/HTTP
    if is_https || is_http {
        // URL should have a host and path
        let without_scheme = if is_https {
            url.strip_prefix("https://").unwrap()
        } else {
            url.strip_prefix("http://").unwrap()
        };

        if without_scheme.is_empty() || !without_scheme.contains('/') {
            return Err("Invalid URL format: missing host or path".to_string());
        }

        // Check for minimum path components (host/owner/repo)
        let parts: Vec<&str> = without_scheme
            .split('/')
            .filter(|s| !s.is_empty())
            .collect();
        if parts.len() < 2 {
            return Err("Invalid URL format: expected host/owner/repo".to_string());
        }
    }

    // Basic validation for SSH URLs
    if is_ssh {
        if url.starts_with("git@") {
            // git@host:owner/repo format
            if !url.contains(':') || !url.contains('/') {
                return Err("Invalid SSH URL format: expected git@host:owner/repo".to_string());
            }
        } else {
            // ssh://git@host/owner/repo format
            let without_scheme = url.strip_prefix("ssh://").unwrap();
            if without_scheme.is_empty() || without_scheme.split('/').count() < 3 {
                return Err("Invalid SSH URL format".to_string());
            }
        }
    }

    Ok(())
}

/// Validates that a branch name is valid
fn validate_branch_name(branch: &str) -> Result<(), String> {
    if branch.is_empty() {
        return Err("Branch name cannot be empty".to_string());
    }

    // Git branch name validation rules
    if branch.starts_with('-')
        || branch.starts_with('.')
        || branch.ends_with('.')
        || branch.ends_with('/')
        || branch.contains("..")
        || branch.contains("//")
        || branch.contains("@{")
        || branch.contains('\\')
        || branch.contains('\0')
        || branch.contains(' ')
        || branch.contains('~')
        || branch.contains('^')
        || branch.contains(':')
        || branch.contains('?')
        || branch.contains('*')
        || branch.contains('[')
    {
        return Err("Invalid branch name: contains disallowed characters or patterns".to_string());
    }

    // Check for reasonable length
    if branch.len() > 255 {
        return Err("Branch name is too long (max 255 characters)".to_string());
    }

    Ok(())
}

/// Parses a git repository URL and extracts name and provider
fn parse_repository_url(url: &str) -> Result<(String, VCSProviderType), String> {
    let url = url.trim();

    // Detect provider from URL
    let provider = Repository::detect_provider_from_url(url).ok_or(
        "Could not detect VCS provider from URL. Supported providers: GitHub, GitLab, Bitbucket",
    )?;

    // Extract repo name from URL
    // Handles formats like:
    // - https://github.com/owner/repo.git
    // - https://github.com/owner/repo
    // - https://github.com/owner/repo/
    // - git@github.com:owner/repo.git
    // - ssh://git@github.com/owner/repo.git

    // First, clean up the URL: remove trailing slashes and .git suffix
    let cleaned = url.trim_end_matches('/').trim_end_matches(".git");

    // Try to extract from HTTPS/HTTP URLs first
    let name = if cleaned.contains("://") {
        // HTTPS/HTTP or ssh:// URL format
        cleaned.split('/').rfind(|s| !s.is_empty())
    } else if cleaned.contains(':') {
        // SSH format: git@github.com:owner/repo
        cleaned
            .split(':')
            .next_back()
            .and_then(|path| path.split('/').rfind(|s| !s.is_empty()))
    } else {
        None
    };

    let name = name
        .ok_or("Could not extract repository name from URL")?
        .to_string();

    if name.is_empty() {
        return Err("Could not extract repository name from URL".to_string());
    }

    Ok((name, provider))
}

/// Validates if a path is a valid git repository
#[tauri::command]
pub fn validate_repository_path(path: String) -> Result<RepositoryInfo, String> {
    let path_buf = PathBuf::from(&path);

    if !path_buf.exists() {
        return Err("Path does not exist".to_string());
    }

    // Canonicalize to resolve symlinks and get absolute path
    let canonical_path = path_buf
        .canonicalize()
        .map_err(|e| format!("Failed to resolve path '{}': {}", path, e))?;

    let (remote_url, name, provider) = RepositoryService::detect_from_path(&canonical_path)
        .ok_or("Not a valid git repository or missing 'origin' remote")?;

    let default_branch = RepositoryService::detect_default_branch(&canonical_path);

    Ok(RepositoryInfo {
        name,
        remote_url,
        provider,
        default_branch,
    })
}

#[derive(serde::Serialize)]
pub struct RepositoryInfo {
    pub name: String,
    pub remote_url: String,
    pub provider: VCSProviderType,
    pub default_branch: String,
}

/// Lists files in a repository for autocomplete
/// Uses git ls-files to get tracked files, respecting .gitignore
/// Note: This command only works for repositories with a local path.
/// For repositories added via URL (server mode), this will return an error.
#[tauri::command]
pub async fn list_repository_files(
    state: State<'_, Arc<AppState>>,
    repository_id: String,
    query: Option<String>,
    limit: Option<usize>,
) -> Result<Vec<String>, String> {
    let repo = state
        .repository_service
        .get(&repository_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Repository not found".to_string())?;

    // Check if repository has a local path (not available for URL-only
    // repositories)
    if repo.local_path.is_empty() {
        return Err(
            "Repository does not have a local path. File listing is not available for \
             repositories added via URL."
                .to_string(),
        );
    }

    let repo_path = PathBuf::from(&repo.local_path);
    if !repo_path.exists() {
        return Err("Repository path does not exist".to_string());
    }

    // Use git ls-files to get tracked files
    let output = tokio::process::Command::new("git")
        .args(["ls-files"])
        .current_dir(&repo_path)
        .output()
        .await
        .map_err(|e| format!("Failed to run git ls-files: {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "git ls-files failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let files_str = String::from_utf8_lossy(&output.stdout);
    let mut files: Vec<String> = files_str.lines().map(|s| s.to_string()).collect();

    // Filter by query if provided (fuzzy match)
    if let Some(q) = query {
        let q_lower = q.to_lowercase();
        files.retain(|f| {
            let f_lower = f.to_lowercase();
            // Match if query appears anywhere in the path
            // or if the file name contains the query
            f_lower.contains(&q_lower)
                || f.split('/')
                    .next_back()
                    .map(|name| name.to_lowercase().contains(&q_lower))
                    .unwrap_or(false)
        });

        // Sort by relevance: exact filename matches first, then by path length
        files.sort_by(|a, b| {
            let a_name = a.split('/').next_back().unwrap_or(a).to_lowercase();
            let b_name = b.split('/').next_back().unwrap_or(b).to_lowercase();
            let a_exact = a_name == q_lower;
            let b_exact = b_name == q_lower;
            let a_starts = a_name.starts_with(&q_lower);
            let b_starts = b_name.starts_with(&q_lower);

            match (a_exact, b_exact) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => match (a_starts, b_starts) {
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                    _ => a.len().cmp(&b.len()),
                },
            }
        });
    }

    // Apply limit
    let limit = limit.unwrap_or(50);
    files.truncate(limit);

    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;

    mod validate_git_url {
        use super::*;

        #[test]
        fn test_accepts_valid_https_urls() {
            assert!(validate_git_url("https://github.com/owner/repo").is_ok());
            assert!(validate_git_url("https://github.com/owner/repo.git").is_ok());
            assert!(validate_git_url("https://gitlab.com/owner/repo").is_ok());
            assert!(validate_git_url("https://bitbucket.org/owner/repo").is_ok());
        }

        #[test]
        fn test_accepts_valid_ssh_urls() {
            assert!(validate_git_url("git@github.com:owner/repo.git").is_ok());
            assert!(validate_git_url("git@gitlab.com:owner/repo.git").is_ok());
            assert!(validate_git_url("ssh://git@github.com/owner/repo.git").is_ok());
        }

        #[test]
        fn test_rejects_empty_url() {
            assert!(validate_git_url("").is_err());
            assert!(validate_git_url("   ").is_err());
        }

        #[test]
        fn test_rejects_invalid_scheme() {
            assert!(validate_git_url("ftp://github.com/owner/repo").is_err());
            assert!(validate_git_url("file:///path/to/repo").is_err());
            assert!(validate_git_url("github.com/owner/repo").is_err());
        }

        #[test]
        fn test_rejects_dangerous_patterns() {
            assert!(validate_git_url("https://github.com/../etc/passwd").is_err());
            assert!(validate_git_url("https://github.com/owner/repo\0name").is_err());
            assert!(validate_git_url("https://github.com/owner\n/repo").is_err());
            assert!(validate_git_url("https://github.com/owner\r/repo").is_err());
        }

        #[test]
        fn test_rejects_incomplete_https_urls() {
            assert!(validate_git_url("https://").is_err());
            assert!(validate_git_url("https://github.com").is_err());
            assert!(validate_git_url("https://github.com/").is_err());
        }

        #[test]
        fn test_rejects_invalid_ssh_urls() {
            assert!(validate_git_url("git@github.com").is_err());
            assert!(validate_git_url("git@github.com:owner").is_err());
        }
    }

    mod validate_branch_name {
        use super::*;

        #[test]
        fn test_accepts_valid_branch_names() {
            assert!(validate_branch_name("main").is_ok());
            assert!(validate_branch_name("master").is_ok());
            assert!(validate_branch_name("feature/new-feature").is_ok());
            assert!(validate_branch_name("fix-123").is_ok());
            assert!(validate_branch_name("release-v1.0.0").is_ok());
        }

        #[test]
        fn test_rejects_empty_branch_name() {
            assert!(validate_branch_name("").is_err());
        }

        #[test]
        fn test_rejects_invalid_branch_names() {
            assert!(validate_branch_name("-feature").is_err());
            assert!(validate_branch_name(".hidden").is_err());
            assert!(validate_branch_name("branch.").is_err());
            assert!(validate_branch_name("branch/").is_err());
            assert!(validate_branch_name("branch..name").is_err());
            assert!(validate_branch_name("branch//name").is_err());
            assert!(validate_branch_name("branch@{name}").is_err());
            assert!(validate_branch_name("branch\\name").is_err());
            assert!(validate_branch_name("branch name").is_err());
            assert!(validate_branch_name("branch~name").is_err());
            assert!(validate_branch_name("branch^name").is_err());
            assert!(validate_branch_name("branch:name").is_err());
            assert!(validate_branch_name("branch?name").is_err());
            assert!(validate_branch_name("branch*name").is_err());
            assert!(validate_branch_name("branch[name").is_err());
        }

        #[test]
        fn test_rejects_too_long_branch_name() {
            let long_name = "a".repeat(256);
            assert!(validate_branch_name(&long_name).is_err());
        }
    }

    mod parse_repository_url {
        use super::*;

        #[test]
        fn test_parses_https_github_urls() {
            let (name, provider) = parse_repository_url("https://github.com/owner/repo").unwrap();
            assert_eq!(name, "repo");
            assert_eq!(provider, VCSProviderType::GitHub);
        }

        #[test]
        fn test_parses_https_github_urls_with_git_suffix() {
            let (name, provider) =
                parse_repository_url("https://github.com/owner/repo.git").unwrap();
            assert_eq!(name, "repo");
            assert_eq!(provider, VCSProviderType::GitHub);
        }

        #[test]
        fn test_parses_https_github_urls_with_trailing_slash() {
            let (name, provider) = parse_repository_url("https://github.com/owner/repo/").unwrap();
            assert_eq!(name, "repo");
            assert_eq!(provider, VCSProviderType::GitHub);
        }

        #[test]
        fn test_parses_https_gitlab_urls() {
            let (name, provider) = parse_repository_url("https://gitlab.com/owner/repo").unwrap();
            assert_eq!(name, "repo");
            assert_eq!(provider, VCSProviderType::GitLab);
        }

        #[test]
        fn test_parses_https_bitbucket_urls() {
            let (name, provider) =
                parse_repository_url("https://bitbucket.org/owner/repo").unwrap();
            assert_eq!(name, "repo");
            assert_eq!(provider, VCSProviderType::Bitbucket);
        }

        #[test]
        fn test_parses_ssh_github_urls() {
            let (name, provider) = parse_repository_url("git@github.com:owner/repo.git").unwrap();
            assert_eq!(name, "repo");
            assert_eq!(provider, VCSProviderType::GitHub);
        }

        #[test]
        fn test_parses_ssh_protocol_urls() {
            let (name, provider) =
                parse_repository_url("ssh://git@github.com/owner/repo.git").unwrap();
            assert_eq!(name, "repo");
            assert_eq!(provider, VCSProviderType::GitHub);
        }

        #[test]
        fn test_handles_whitespace() {
            let (name, provider) =
                parse_repository_url("  https://github.com/owner/repo  ").unwrap();
            assert_eq!(name, "repo");
            assert_eq!(provider, VCSProviderType::GitHub);
        }

        #[test]
        fn test_rejects_unknown_provider() {
            let result = parse_repository_url("https://custom-git.example.com/owner/repo");
            assert!(result.is_err());
            assert!(result
                .unwrap_err()
                .contains("Could not detect VCS provider"));
        }
    }
}
