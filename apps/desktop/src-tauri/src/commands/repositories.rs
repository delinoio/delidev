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

    let repo = Repository::new(
        uuid::Uuid::new_v4().to_string(),
        name,
        String::new(), // Empty local path for server mode
        remote_url,
        provider,
    )
    .with_default_branch(default_branch.unwrap_or_else(|| "main".to_string()));

    state
        .repository_service
        .create(&repo)
        .await
        .map_err(|e| e.to_string())?;

    Ok(repo)
}

/// Parses a git repository URL and extracts name and provider
fn parse_repository_url(url: &str) -> Result<(String, VCSProviderType), String> {
    // Detect provider from URL
    let provider = Repository::detect_provider_from_url(url).ok_or(
        "Could not detect VCS provider from URL. Supported providers: GitHub, GitLab, Bitbucket",
    )?;

    // Extract repo name from URL
    // Handles formats like:
    // - https://github.com/owner/repo.git
    // - https://github.com/owner/repo
    // - git@github.com:owner/repo.git
    let name = url
        .trim_end_matches(".git")
        .split('/')
        .next_back()
        .or_else(|| {
            // Handle SSH format: git@github.com:owner/repo.git
            url.trim_end_matches(".git")
                .split(':')
                .next_back()
                .and_then(|s| s.split('/').next_back())
        })
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
