//! Worker management API endpoints (internal).

use std::sync::Arc;

use axum::{extract::State, Json};
use rpc_protocol::{
    requests::*,
    responses::*,
    WorkerStatus as RpcWorkerStatus, UnitTaskStatus,
};
use task_store::TaskStore;
use uuid::Uuid;

use crate::error::{ServerError, ServerResult};
use crate::services::worker_registry::WorkerStatus;
use crate::state::AppState;

/// Converts RPC WorkerStatus to registry WorkerStatus.
fn to_registry_status(status: RpcWorkerStatus) -> WorkerStatus {
    match status {
        RpcWorkerStatus::Unspecified | RpcWorkerStatus::Idle => WorkerStatus::Idle,
        RpcWorkerStatus::Busy => WorkerStatus::Busy,
        RpcWorkerStatus::Unhealthy => WorkerStatus::Unhealthy,
    }
}

/// Registers a new worker.
pub async fn register_worker<S: TaskStore>(
    State(state): State<Arc<AppState<S>>>,
    Json(request): Json<RegisterWorkerRequest>,
) -> ServerResult<Json<RegisterWorkerResponse>> {
    let mut registry = state.worker_registry.write().await;
    let worker_id = registry.register(&request.name, &request.endpoint_url);

    tracing::info!(
        worker_id = %worker_id,
        name = %request.name,
        endpoint = %request.endpoint_url,
        "Worker registered"
    );

    Ok(Json(RegisterWorkerResponse {
        worker_id: worker_id.to_string(),
    }))
}

/// Handles worker heartbeat.
pub async fn heartbeat<S: TaskStore>(
    State(state): State<Arc<AppState<S>>>,
    Json(request): Json<HeartbeatRequest>,
) -> ServerResult<Json<HeartbeatResponse>> {
    let worker_id: Uuid = request
        .worker_id
        .parse()
        .map_err(|_| ServerError::InvalidRequest("Invalid worker_id".to_string()))?;

    let current_task_id = request
        .current_task_id
        .as_ref()
        .and_then(|id| id.parse().ok());

    let mut registry = state.worker_registry.write().await;
    let success = registry.heartbeat(worker_id, to_registry_status(request.status), current_task_id);

    if !success {
        return Err(ServerError::NotFound("Worker not found".to_string()));
    }

    Ok(Json(HeartbeatResponse {}))
}

/// Unregisters a worker.
pub async fn unregister_worker<S: TaskStore>(
    State(state): State<Arc<AppState<S>>>,
    Json(request): Json<UnregisterWorkerRequest>,
) -> ServerResult<Json<UnregisterWorkerResponse>> {
    let worker_id: Uuid = request
        .worker_id
        .parse()
        .map_err(|_| ServerError::InvalidRequest("Invalid worker_id".to_string()))?;

    let mut registry = state.worker_registry.write().await;
    registry.unregister(worker_id);

    tracing::info!(worker_id = %worker_id, "Worker unregistered");

    Ok(Json(UnregisterWorkerResponse {}))
}

/// Gets the next task for a worker to execute.
pub async fn get_next_task<S: TaskStore>(
    State(state): State<Arc<AppState<S>>>,
    Json(request): Json<GetNextTaskRequest>,
) -> ServerResult<Json<GetNextTaskResponse>> {
    let worker_id: Uuid = request
        .worker_id
        .parse()
        .map_err(|_| ServerError::InvalidRequest("Invalid worker_id".to_string()))?;

    // Verify worker exists
    {
        let registry = state.worker_registry.read().await;
        if registry.get(worker_id).is_none() {
            return Err(ServerError::NotFound("Worker not found".to_string()));
        }
    }

    // Find a task that needs execution (status = in_progress but no active session)
    let filter = task_store::TaskFilter {
        unit_status: Some(entities::UnitTaskStatus::InProgress),
        ..Default::default()
    };

    let (tasks, _) = state.store.list_unit_tasks(filter).await?;

    // Find a task that isn't already assigned to a worker
    for task in tasks {
        // Check if task is already assigned
        let registry = state.worker_registry.read().await;
        let already_assigned = registry
            .all_workers()
            .any(|w| w.current_task_id == Some(task.id));

        if !already_assigned {
            // Assign task to worker
            drop(registry);
            let mut registry = state.worker_registry.write().await;
            if registry.assign_task(worker_id, task.id) {
                // Get agent task
                let agent_task = state.store.get_agent_task(task.agent_task_id).await?;

                let rpc_task = rpc_protocol::UnitTask {
                    id: task.id.to_string(),
                    repository_group_id: task.repository_group_id.to_string(),
                    agent_task_id: task.agent_task_id.to_string(),
                    prompt: task.prompt.clone(),
                    title: task.title.clone(),
                    branch_name: task.branch_name.clone(),
                    linked_pr_url: task.linked_pr_url.clone(),
                    base_commit: task.base_commit.clone(),
                    end_commit: task.end_commit.clone(),
                    auto_fix_task_ids: task.auto_fix_task_ids.iter().map(|id| id.to_string()).collect(),
                    status: UnitTaskStatus::InProgress,
                    created_at: task.created_at,
                    updated_at: task.updated_at,
                };

                let rpc_agent_task = agent_task.map(|at| rpc_protocol::AgentTask {
                    id: at.id.to_string(),
                    base_remotes: at.base_remotes.iter().map(|br| rpc_protocol::BaseRemote {
                        git_remote_dir_path: br.git_remote_dir_path.clone(),
                        git_branch_name: br.git_branch_name.clone(),
                    }).collect(),
                    agent_sessions: vec![],
                    ai_agent_type: at.ai_agent_type.map(|t| match t {
                        entities::AiAgentType::ClaudeCode => rpc_protocol::AiAgentType::ClaudeCode,
                        entities::AiAgentType::OpenCode => rpc_protocol::AiAgentType::OpenCode,
                        entities::AiAgentType::GeminiCli => rpc_protocol::AiAgentType::GeminiCli,
                        entities::AiAgentType::CodexCli => rpc_protocol::AiAgentType::CodexCli,
                        entities::AiAgentType::Aider => rpc_protocol::AiAgentType::Aider,
                        entities::AiAgentType::Amp => rpc_protocol::AiAgentType::Amp,
                    }),
                    ai_agent_model: at.ai_agent_model,
                    created_at: at.created_at,
                });

                tracing::info!(worker_id = %worker_id, task_id = %task.id, "Task assigned to worker");

                return Ok(Json(GetNextTaskResponse {
                    task: Some(rpc_task),
                    agent_task: rpc_agent_task,
                }));
            }
        }
    }

    // No tasks available
    Ok(Json(GetNextTaskResponse {
        task: None,
        agent_task: None,
    }))
}

/// Reports task execution status from a worker.
pub async fn report_task_status<S: TaskStore>(
    State(state): State<Arc<AppState<S>>>,
    Json(request): Json<ReportTaskStatusRequest>,
) -> ServerResult<Json<ReportTaskStatusResponse>> {
    let worker_id: Uuid = request
        .worker_id
        .parse()
        .map_err(|_| ServerError::InvalidRequest("Invalid worker_id".to_string()))?;

    let task_id: Uuid = request
        .task_id
        .parse()
        .map_err(|_| ServerError::InvalidRequest("Invalid task_id".to_string()))?;

    // Update task status
    let mut task = state
        .store
        .get_unit_task(task_id)
        .await?
        .ok_or_else(|| ServerError::NotFound("Task not found".to_string()))?;

    task.status = match request.status {
        UnitTaskStatus::InProgress => entities::UnitTaskStatus::InProgress,
        UnitTaskStatus::InReview => entities::UnitTaskStatus::InReview,
        UnitTaskStatus::Approved => entities::UnitTaskStatus::Approved,
        UnitTaskStatus::PrOpen => entities::UnitTaskStatus::PrOpen,
        UnitTaskStatus::Done => entities::UnitTaskStatus::Done,
        UnitTaskStatus::Rejected => entities::UnitTaskStatus::Rejected,
        _ => entities::UnitTaskStatus::InProgress,
    };
    task.updated_at = chrono::Utc::now();

    state.store.update_unit_task(task).await?;

    // Mark worker as idle if task is completed
    if matches!(
        request.status,
        UnitTaskStatus::InReview | UnitTaskStatus::Done | UnitTaskStatus::Rejected
    ) {
        let mut registry = state.worker_registry.write().await;
        registry.complete_task(worker_id);
    }

    tracing::info!(
        worker_id = %worker_id,
        task_id = %task_id,
        status = ?request.status,
        "Task status reported"
    );

    Ok(Json(ReportTaskStatusResponse {}))
}

/// Gets secrets for a task (called by worker when task starts).
pub async fn get_secrets<S: TaskStore>(
    State(state): State<Arc<AppState<S>>>,
    Json(request): Json<GetSecretsRequest>,
) -> ServerResult<Json<GetSecretsResponse>> {
    let worker_id: Uuid = request
        .worker_id
        .parse()
        .map_err(|_| ServerError::InvalidRequest("Invalid worker_id".to_string()))?;

    let task_id: Uuid = request
        .task_id
        .parse()
        .map_err(|_| ServerError::InvalidRequest("Invalid task_id".to_string()))?;

    // Verify worker exists and is assigned this task
    {
        let registry = state.worker_registry.read().await;
        let worker = registry
            .get(worker_id)
            .ok_or_else(|| ServerError::NotFound("Worker not found".to_string()))?;

        if worker.current_task_id != Some(task_id) {
            return Err(ServerError::PermissionDenied(
                "Worker is not assigned to this task".to_string(),
            ));
        }
    }

    // Get secrets from cache
    let cache = state.secrets_cache.read().await;
    let secrets = cache.get(&task_id).cloned().unwrap_or_default();

    tracing::info!(
        worker_id = %worker_id,
        task_id = %task_id,
        secret_count = secrets.len(),
        "Secrets retrieved for task"
    );

    Ok(Json(GetSecretsResponse { secrets }))
}
