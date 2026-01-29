use std::sync::Arc;

use tauri::State;

use crate::{entities::Workspace, services::AppState};

/// Lists all workspaces
#[tauri::command]
pub async fn list_workspaces(state: State<'_, Arc<AppState>>) -> Result<Vec<Workspace>, String> {
    state
        .workspace_service
        .list()
        .await
        .map_err(|e| e.to_string())
}

/// Gets a workspace by ID
#[tauri::command]
pub async fn get_workspace(
    state: State<'_, Arc<AppState>>,
    id: String,
) -> Result<Option<Workspace>, String> {
    state
        .workspace_service
        .get(&id)
        .await
        .map_err(|e| e.to_string())
}

/// Creates a new workspace
#[tauri::command]
pub async fn create_workspace(
    state: State<'_, Arc<AppState>>,
    name: String,
    description: Option<String>,
) -> Result<Workspace, String> {
    let workspace_id = uuid::Uuid::new_v4().to_string();
    let mut workspace = Workspace::new(workspace_id, name);

    if let Some(desc) = description {
        workspace = workspace.with_description(desc);
    }

    state
        .workspace_service
        .create(&workspace)
        .await
        .map_err(|e| e.to_string())?;

    Ok(workspace)
}

/// Updates a workspace
#[tauri::command]
pub async fn update_workspace(
    state: State<'_, Arc<AppState>>,
    id: String,
    name: String,
    description: Option<String>,
) -> Result<Workspace, String> {
    let mut workspace = state
        .workspace_service
        .get(&id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or("Workspace not found")?;

    workspace.name = name;
    workspace.description = description;
    workspace.updated_at = chrono::Utc::now();

    state
        .workspace_service
        .update(&workspace)
        .await
        .map_err(|e| e.to_string())?;

    Ok(workspace)
}

/// Deletes a workspace
#[tauri::command]
pub async fn delete_workspace(state: State<'_, Arc<AppState>>, id: String) -> Result<(), String> {
    state
        .workspace_service
        .delete(&id)
        .await
        .map_err(|e| e.to_string())
}

/// Adds a repository to a workspace
#[tauri::command]
pub async fn add_repository_to_workspace(
    state: State<'_, Arc<AppState>>,
    workspace_id: String,
    repository_id: String,
) -> Result<(), String> {
    state
        .workspace_service
        .add_repository(&workspace_id, &repository_id)
        .await
        .map_err(|e| e.to_string())
}

/// Removes a repository from a workspace
#[tauri::command]
pub async fn remove_repository_from_workspace(
    state: State<'_, Arc<AppState>>,
    workspace_id: String,
    repository_id: String,
) -> Result<(), String> {
    state
        .workspace_service
        .remove_repository(&workspace_id, &repository_id)
        .await
        .map_err(|e| e.to_string())
}

/// Lists repositories in a workspace
#[tauri::command]
pub async fn list_workspace_repositories(
    state: State<'_, Arc<AppState>>,
    workspace_id: String,
) -> Result<Vec<crate::entities::Repository>, String> {
    let repo_ids = state
        .workspace_service
        .list_repository_ids(&workspace_id)
        .await
        .map_err(|e| e.to_string())?;

    let mut repos = Vec::new();
    for id in repo_ids {
        if let Ok(Some(repo)) = state.repository_service.get(&id).await {
            repos.push(repo);
        }
    }

    Ok(repos)
}

/// Gets the default workspace (creates one if it doesn't exist)
#[tauri::command]
pub async fn get_default_workspace(state: State<'_, Arc<AppState>>) -> Result<Workspace, String> {
    state
        .workspace_service
        .get_or_create_default()
        .await
        .map_err(|e| e.to_string())
}
