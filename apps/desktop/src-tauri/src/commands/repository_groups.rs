use std::sync::Arc;

use tauri::State;

use crate::{entities::RepositoryGroup, services::AppState};

/// Lists all repository groups
#[tauri::command]
pub async fn list_repository_groups(
    state: State<'_, Arc<AppState>>,
    workspace_id: Option<String>,
) -> Result<Vec<RepositoryGroup>, String> {
    match workspace_id {
        Some(id) => state
            .repository_group_service
            .list_by_workspace(&id)
            .await
            .map_err(|e| e.to_string()),
        None => state
            .repository_group_service
            .list()
            .await
            .map_err(|e| e.to_string()),
    }
}

/// Gets a repository group by ID
#[tauri::command]
pub async fn get_repository_group(
    state: State<'_, Arc<AppState>>,
    id: String,
) -> Result<Option<RepositoryGroup>, String> {
    state
        .repository_group_service
        .get(&id)
        .await
        .map_err(|e| e.to_string())
}

/// Creates a new repository group
#[tauri::command]
pub async fn create_repository_group(
    state: State<'_, Arc<AppState>>,
    workspace_id: String,
    name: Option<String>,
    repository_ids: Vec<String>,
) -> Result<RepositoryGroup, String> {
    let group_id = uuid::Uuid::new_v4().to_string();
    let mut group = RepositoryGroup::new(group_id.clone(), workspace_id);

    if let Some(n) = name {
        group = group.with_name(n);
    }

    group.repository_ids = repository_ids;

    state
        .repository_group_service
        .create(&group)
        .await
        .map_err(|e| e.to_string())?;

    Ok(group)
}

/// Updates a repository group
#[tauri::command]
pub async fn update_repository_group(
    state: State<'_, Arc<AppState>>,
    id: String,
    name: Option<String>,
) -> Result<RepositoryGroup, String> {
    let mut group = state
        .repository_group_service
        .get(&id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or("Repository group not found")?;

    group.name = name;
    group.updated_at = chrono::Utc::now();

    state
        .repository_group_service
        .update(&group)
        .await
        .map_err(|e| e.to_string())?;

    Ok(group)
}

/// Deletes a repository group
#[tauri::command]
pub async fn delete_repository_group(
    state: State<'_, Arc<AppState>>,
    id: String,
) -> Result<(), String> {
    state
        .repository_group_service
        .delete(&id)
        .await
        .map_err(|e| e.to_string())
}

/// Adds a repository to a repository group
#[tauri::command]
pub async fn add_repository_to_group(
    state: State<'_, Arc<AppState>>,
    group_id: String,
    repository_id: String,
) -> Result<(), String> {
    state
        .repository_group_service
        .add_repository(&group_id, &repository_id)
        .await
        .map_err(|e| e.to_string())
}

/// Removes a repository from a repository group
#[tauri::command]
pub async fn remove_repository_from_group(
    state: State<'_, Arc<AppState>>,
    group_id: String,
    repository_id: String,
) -> Result<(), String> {
    state
        .repository_group_service
        .remove_repository(&group_id, &repository_id)
        .await
        .map_err(|e| e.to_string())
}

/// Gets or creates a single-repo group for a repository
/// This is used when creating tasks for a single repository
#[tauri::command]
pub async fn get_or_create_single_repo_group(
    state: State<'_, Arc<AppState>>,
    workspace_id: String,
    repository_id: String,
) -> Result<String, String> {
    state
        .repository_group_service
        .get_or_create_single_repo_group(&workspace_id, &repository_id)
        .await
        .map_err(|e| e.to_string())
}

/// Lists files in all repositories of a repository group for autocomplete
/// Uses git ls-files to get tracked files, respecting .gitignore
#[tauri::command]
pub async fn list_repository_group_files(
    state: State<'_, Arc<AppState>>,
    repository_group_id: String,
    query: Option<String>,
    limit: Option<usize>,
) -> Result<Vec<String>, String> {
    use std::path::PathBuf;

    // Get the repository group
    let group = state
        .repository_group_service
        .get(&repository_group_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Repository group not found".to_string())?;

    let mut all_files: Vec<String> = Vec::new();

    // Collect files from all repositories in the group
    for repo_id in &group.repository_ids {
        let repo = match state
            .repository_service
            .get(repo_id)
            .await
            .map_err(|e| e.to_string())?
        {
            Some(r) => r,
            None => continue, // Skip if repository not found
        };

        let repo_path = PathBuf::from(&repo.local_path);
        if !repo_path.exists() {
            continue; // Skip if repository path doesn't exist
        }

        // Use git ls-files to get tracked files
        let output = match tokio::process::Command::new("git")
            .args(["ls-files"])
            .current_dir(&repo_path)
            .output()
            .await
        {
            Ok(o) => o,
            Err(_) => continue, // Skip on error
        };

        if !output.status.success() {
            continue; // Skip on git error
        }

        let files_str = String::from_utf8_lossy(&output.stdout);
        // Prefix files with repository name for multi-repo groups
        let repo_prefix = if group.repository_ids.len() > 1 {
            format!("{}:", repo.name)
        } else {
            String::new()
        };

        for line in files_str.lines() {
            all_files.push(format!("{}{}", repo_prefix, line));
        }
    }

    // Filter by query if provided (fuzzy match)
    if let Some(q) = query {
        let q_lower = q.to_lowercase();
        all_files.retain(|f| {
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
        all_files.sort_by(|a, b| {
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
    all_files.truncate(limit);

    Ok(all_files)
}
