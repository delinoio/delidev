//! Secrets management API endpoints.

use std::sync::Arc;

use axum::{extract::State, Json};
use rpc_protocol::{requests::*, responses::*};
use task_store::TaskStore;
use uuid::Uuid;

use crate::error::{ServerError, ServerResult};
use crate::state::AppState;

/// Sends secrets from the client to the server for task execution.
pub async fn send_secrets<S: TaskStore>(
    State(state): State<Arc<AppState<S>>>,
    Json(request): Json<SendSecretsRequest>,
) -> ServerResult<Json<SendSecretsResponse>> {
    let task_id: Uuid = request
        .task_id
        .parse()
        .map_err(|_| ServerError::InvalidRequest("Invalid task_id".to_string()))?;

    // Verify task exists
    let task_exists = state.store.get_unit_task(task_id).await?.is_some()
        || state.store.get_composite_task(task_id).await?.is_some();

    if !task_exists {
        return Err(ServerError::NotFound("Task not found".to_string()));
    }

    // Store secrets in cache
    let mut cache = state.secrets_cache.write().await;
    cache.store(task_id, request.secrets);

    tracing::info!(task_id = %task_id, "Secrets stored for task");

    Ok(Json(SendSecretsResponse {}))
}

/// Clears cached secrets for a task.
pub async fn clear_secrets<S: TaskStore>(
    State(state): State<Arc<AppState<S>>>,
    Json(request): Json<ClearSecretsRequest>,
) -> ServerResult<Json<ClearSecretsResponse>> {
    let task_id: Uuid = request
        .task_id
        .parse()
        .map_err(|_| ServerError::InvalidRequest("Invalid task_id".to_string()))?;

    let mut cache = state.secrets_cache.write().await;
    cache.clear(&task_id);

    tracing::info!(task_id = %task_id, "Secrets cleared for task");

    Ok(Json(ClearSecretsResponse {}))
}
