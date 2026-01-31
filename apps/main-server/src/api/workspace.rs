//! Workspace management API endpoints.

use std::sync::Arc;

use axum::{extract::State, Json};
use entities::Workspace;
use rpc_protocol::{requests::*, responses::*};
use task_store::{TaskStore, WorkspaceFilter};
use uuid::Uuid;

use crate::error::{ServerError, ServerResult};
use crate::state::AppState;

/// Converts entity Workspace to RPC Workspace.
fn entity_to_rpc_workspace(workspace: &Workspace) -> rpc_protocol::Workspace {
    rpc_protocol::Workspace {
        id: workspace.id.to_string(),
        name: workspace.name.clone(),
        description: workspace.description.clone(),
        user_id: workspace.user_id.map(|id| id.to_string()),
        created_at: workspace.created_at,
        updated_at: workspace.updated_at,
    }
}

/// Creates a workspace.
pub async fn create_workspace<S: TaskStore>(
    State(state): State<Arc<AppState<S>>>,
    Json(request): Json<CreateWorkspaceRequest>,
) -> ServerResult<Json<CreateWorkspaceResponse>> {
    let mut workspace = Workspace::new(request.name);
    workspace.description = request.description;

    let workspace = state.store.create_workspace(workspace).await?;

    tracing::info!(workspace_id = %workspace.id, "Workspace created");

    Ok(Json(CreateWorkspaceResponse {
        workspace: entity_to_rpc_workspace(&workspace),
    }))
}

/// Lists workspaces.
pub async fn list_workspaces<S: TaskStore>(
    State(state): State<Arc<AppState<S>>>,
    Json(request): Json<ListWorkspacesRequest>,
) -> ServerResult<Json<ListWorkspacesResponse>> {
    let filter = WorkspaceFilter {
        user_id: None, // TODO: Add user_id filtering when auth is integrated
        limit: Some(request.limit as u32),
        offset: Some(request.offset as u32),
    };

    let (workspaces, total) = state.store.list_workspaces(filter).await?;

    Ok(Json(ListWorkspacesResponse {
        workspaces: workspaces.iter().map(entity_to_rpc_workspace).collect(),
        total_count: total as i32,
    }))
}

/// Gets a workspace by ID.
pub async fn get_workspace<S: TaskStore>(
    State(state): State<Arc<AppState<S>>>,
    Json(request): Json<GetWorkspaceRequest>,
) -> ServerResult<Json<GetWorkspaceResponse>> {
    let workspace_id: Uuid = request
        .workspace_id
        .parse()
        .map_err(|_| ServerError::InvalidRequest("Invalid workspace_id".to_string()))?;

    let workspace = state
        .store
        .get_workspace(workspace_id)
        .await?
        .ok_or_else(|| ServerError::NotFound("Workspace not found".to_string()))?;

    Ok(Json(GetWorkspaceResponse {
        workspace: entity_to_rpc_workspace(&workspace),
    }))
}

/// Updates a workspace.
pub async fn update_workspace<S: TaskStore>(
    State(state): State<Arc<AppState<S>>>,
    Json(request): Json<UpdateWorkspaceRequest>,
) -> ServerResult<Json<UpdateWorkspaceResponse>> {
    let workspace_id: Uuid = request
        .workspace_id
        .parse()
        .map_err(|_| ServerError::InvalidRequest("Invalid workspace_id".to_string()))?;

    let mut workspace = state
        .store
        .get_workspace(workspace_id)
        .await?
        .ok_or_else(|| ServerError::NotFound("Workspace not found".to_string()))?;

    // Update fields
    if let Some(name) = request.name {
        workspace.name = name;
    }
    if let Some(description) = request.description {
        workspace.description = Some(description);
    }
    workspace.updated_at = chrono::Utc::now();

    let workspace = state.store.update_workspace(workspace).await?;

    tracing::info!(workspace_id = %workspace_id, "Workspace updated");

    Ok(Json(UpdateWorkspaceResponse {
        workspace: entity_to_rpc_workspace(&workspace),
    }))
}

/// Deletes a workspace.
pub async fn delete_workspace<S: TaskStore>(
    State(state): State<Arc<AppState<S>>>,
    Json(request): Json<DeleteWorkspaceRequest>,
) -> ServerResult<Json<DeleteWorkspaceResponse>> {
    let workspace_id: Uuid = request
        .workspace_id
        .parse()
        .map_err(|_| ServerError::InvalidRequest("Invalid workspace_id".to_string()))?;

    // Verify workspace exists
    state
        .store
        .get_workspace(workspace_id)
        .await?
        .ok_or_else(|| ServerError::NotFound("Workspace not found".to_string()))?;

    state.store.delete_workspace(workspace_id).await?;

    tracing::info!(workspace_id = %workspace_id, "Workspace deleted");

    Ok(Json(DeleteWorkspaceResponse {}))
}
