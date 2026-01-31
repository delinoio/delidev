//! Repository management API endpoints.

use std::sync::Arc;

use axum::{extract::State, Json};
use entities::{Repository, RepositoryGroup, VcsProviderType, VcsType};
use rpc_protocol::{requests::*, responses::*};
use task_store::{RepositoryFilter, TaskStore};
use uuid::Uuid;

use crate::error::{ServerError, ServerResult};
use crate::state::AppState;

/// Converts entity Repository to RPC Repository.
fn entity_to_rpc_repository(repo: &Repository) -> rpc_protocol::Repository {
    rpc_protocol::Repository {
        id: repo.id.to_string(),
        workspace_id: repo.workspace_id.to_string(),
        name: repo.name.clone(),
        remote_url: repo.remote_url.clone(),
        default_branch: repo.default_branch.clone(),
        vcs_type: match repo.vcs_type {
            VcsType::Git => rpc_protocol::VcsType::Git,
        },
        vcs_provider_type: match repo.vcs_provider_type {
            VcsProviderType::Github => rpc_protocol::VcsProviderType::Github,
            VcsProviderType::Gitlab => rpc_protocol::VcsProviderType::Gitlab,
            VcsProviderType::Bitbucket => rpc_protocol::VcsProviderType::Bitbucket,
        },
        created_at: repo.created_at,
        updated_at: repo.updated_at,
    }
}

/// Converts entity RepositoryGroup to RPC RepositoryGroup.
fn entity_to_rpc_repository_group(group: &RepositoryGroup) -> rpc_protocol::RepositoryGroup {
    rpc_protocol::RepositoryGroup {
        id: group.id.to_string(),
        workspace_id: group.workspace_id.to_string(),
        name: group.name.clone(),
        repository_ids: group.repository_ids.iter().map(|id| id.to_string()).collect(),
        created_at: group.created_at,
        updated_at: group.updated_at,
    }
}

/// Detects VCS provider from remote URL.
fn detect_provider(remote_url: &str) -> VcsProviderType {
    if remote_url.contains("github.com") {
        VcsProviderType::Github
    } else if remote_url.contains("gitlab.com") {
        VcsProviderType::Gitlab
    } else if remote_url.contains("bitbucket.org") {
        VcsProviderType::Bitbucket
    } else {
        // Default to GitHub for unknown providers
        VcsProviderType::Github
    }
}

/// Extracts repository name from remote URL.
fn extract_repo_name(remote_url: &str) -> String {
    remote_url
        .rsplit('/')
        .next()
        .unwrap_or("unknown")
        .trim_end_matches(".git")
        .to_string()
}

/// Adds a repository.
pub async fn add_repository<S: TaskStore>(
    State(state): State<Arc<AppState<S>>>,
    Json(request): Json<AddRepositoryRequest>,
) -> ServerResult<Json<AddRepositoryResponse>> {
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

    let name = request.name.unwrap_or_else(|| extract_repo_name(&request.remote_url));
    let provider = detect_provider(&request.remote_url);

    let repo = Repository::new(
        workspace_id,
        name,
        request.remote_url,
        provider,
    );

    let repo = state.store.create_repository(repo).await?;

    tracing::info!(repo_id = %repo.id, "Repository added");

    Ok(Json(AddRepositoryResponse {
        repository: entity_to_rpc_repository(&repo),
    }))
}

/// Lists repositories.
pub async fn list_repositories<S: TaskStore>(
    State(state): State<Arc<AppState<S>>>,
    Json(request): Json<ListRepositoriesRequest>,
) -> ServerResult<Json<ListRepositoriesResponse>> {
    let filter = RepositoryFilter {
        workspace_id: request.workspace_id.as_ref().and_then(|id| id.parse().ok()),
        limit: Some(request.limit as u32),
        offset: Some(request.offset as u32),
    };

    let (repos, total) = state.store.list_repositories(filter).await?;

    Ok(Json(ListRepositoriesResponse {
        repositories: repos.iter().map(entity_to_rpc_repository).collect(),
        total_count: total as i32,
    }))
}

/// Gets a repository by ID.
pub async fn get_repository<S: TaskStore>(
    State(state): State<Arc<AppState<S>>>,
    Json(request): Json<GetRepositoryRequest>,
) -> ServerResult<Json<GetRepositoryResponse>> {
    let repo_id: Uuid = request
        .repository_id
        .parse()
        .map_err(|_| ServerError::InvalidRequest("Invalid repository_id".to_string()))?;

    let repo = state
        .store
        .get_repository(repo_id)
        .await?
        .ok_or_else(|| ServerError::NotFound("Repository not found".to_string()))?;

    Ok(Json(GetRepositoryResponse {
        repository: entity_to_rpc_repository(&repo),
    }))
}

/// Removes a repository.
pub async fn remove_repository<S: TaskStore>(
    State(state): State<Arc<AppState<S>>>,
    Json(request): Json<RemoveRepositoryRequest>,
) -> ServerResult<Json<RemoveRepositoryResponse>> {
    let repo_id: Uuid = request
        .repository_id
        .parse()
        .map_err(|_| ServerError::InvalidRequest("Invalid repository_id".to_string()))?;

    // Verify repository exists
    state
        .store
        .get_repository(repo_id)
        .await?
        .ok_or_else(|| ServerError::NotFound("Repository not found".to_string()))?;

    state.store.delete_repository(repo_id).await?;

    tracing::info!(repo_id = %repo_id, "Repository removed");

    Ok(Json(RemoveRepositoryResponse {}))
}

/// Creates a repository group.
pub async fn create_repository_group<S: TaskStore>(
    State(state): State<Arc<AppState<S>>>,
    Json(request): Json<CreateRepositoryGroupRequest>,
) -> ServerResult<Json<CreateRepositoryGroupResponse>> {
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

    // Parse repository IDs
    let repository_ids: Vec<Uuid> = request
        .repository_ids
        .iter()
        .map(|id| {
            id.parse()
                .map_err(|_| ServerError::InvalidRequest(format!("Invalid repository_id: {}", id)))
        })
        .collect::<Result<Vec<_>, _>>()?;

    // Verify all repositories exist
    for repo_id in &repository_ids {
        state
            .store
            .get_repository(*repo_id)
            .await?
            .ok_or_else(|| ServerError::NotFound(format!("Repository {} not found", repo_id)))?;
    }

    let mut group = RepositoryGroup::new(workspace_id);
    group.name = request.name;
    group.repository_ids = repository_ids;

    let group = state.store.create_repository_group(group).await?;

    tracing::info!(group_id = %group.id, "Repository group created");

    Ok(Json(CreateRepositoryGroupResponse {
        group: entity_to_rpc_repository_group(&group),
    }))
}

/// Lists repository groups.
pub async fn list_repository_groups<S: TaskStore>(
    State(state): State<Arc<AppState<S>>>,
    Json(request): Json<ListRepositoryGroupsRequest>,
) -> ServerResult<Json<ListRepositoryGroupsResponse>> {
    let workspace_id = request.workspace_id.as_ref().and_then(|id| id.parse().ok());

    let groups = state.store.list_repository_groups(workspace_id).await?;

    Ok(Json(ListRepositoryGroupsResponse {
        groups: groups.iter().map(entity_to_rpc_repository_group).collect(),
        total_count: groups.len() as i32,
    }))
}

/// Updates a repository group.
pub async fn update_repository_group<S: TaskStore>(
    State(state): State<Arc<AppState<S>>>,
    Json(request): Json<UpdateRepositoryGroupRequest>,
) -> ServerResult<Json<UpdateRepositoryGroupResponse>> {
    let group_id: Uuid = request
        .group_id
        .parse()
        .map_err(|_| ServerError::InvalidRequest("Invalid group_id".to_string()))?;

    let mut group = state
        .store
        .get_repository_group(group_id)
        .await?
        .ok_or_else(|| ServerError::NotFound("Repository group not found".to_string()))?;

    // Update fields
    if let Some(name) = request.name {
        group.name = Some(name);
    }

    // Parse and validate repository IDs
    let repository_ids: Vec<Uuid> = request
        .repository_ids
        .iter()
        .map(|id| {
            id.parse()
                .map_err(|_| ServerError::InvalidRequest(format!("Invalid repository_id: {}", id)))
        })
        .collect::<Result<Vec<_>, _>>()?;

    group.repository_ids = repository_ids;
    group.updated_at = chrono::Utc::now();

    let group = state.store.update_repository_group(group).await?;

    tracing::info!(group_id = %group_id, "Repository group updated");

    Ok(Json(UpdateRepositoryGroupResponse {
        group: entity_to_rpc_repository_group(&group),
    }))
}

/// Deletes a repository group.
pub async fn delete_repository_group<S: TaskStore>(
    State(state): State<Arc<AppState<S>>>,
    Json(request): Json<DeleteRepositoryGroupRequest>,
) -> ServerResult<Json<DeleteRepositoryGroupResponse>> {
    let group_id: Uuid = request
        .group_id
        .parse()
        .map_err(|_| ServerError::InvalidRequest("Invalid group_id".to_string()))?;

    // Verify group exists
    state
        .store
        .get_repository_group(group_id)
        .await?
        .ok_or_else(|| ServerError::NotFound("Repository group not found".to_string()))?;

    state.store.delete_repository_group(group_id).await?;

    tracing::info!(group_id = %group_id, "Repository group deleted");

    Ok(Json(DeleteRepositoryGroupResponse {}))
}
