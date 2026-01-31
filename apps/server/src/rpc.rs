//! JSON-RPC request handling

use auth::AuthenticatedUser;
use axum::{extract::State, http::StatusCode, Json};
use rpc_protocol::{
    method_names, AddRepositoryRequest, AddRepositoryResponse, ApproveCompositePlanRequest,
    CreateCompositeTaskRequest, CreateCompositeTaskResponse, CreateUnitTaskRequest,
    CreateUnitTaskResponse, GetCompositeTaskRequest, GetCompositeTaskResponse,
    GetExecutionLogsRequest, GetExecutionLogsResponse, GetUnitTaskRequest, GetUnitTaskResponse,
    JsonRpcError, JsonRpcRequest, JsonRpcResponse, ListRepositoriesRequest,
    ListRepositoriesResponse, ListUnitTasksRequest, ListUnitTasksResponse, RegisterWorkerRequest,
    RegisterWorkerResponse, RejectCompositePlanRequest, ReportTaskCompleteRequest,
    SendExecutionLogRequest, SendSecretsRequest, SendSecretsResponse, StartTaskExecutionRequest,
    StartTaskExecutionResponse, StopTaskExecutionRequest, SuccessResponse,
    UpdateUnitTaskStatusRequest, WorkerHeartbeatRequest, WorkerHeartbeatResponse,
};
use task_store::{
    CompositeTask, CompositeTaskStatus, Repository, TaskFilter, UnitTask, UnitTaskStatus,
};
use tracing::info;

use crate::state::AppState;

/// Handle a JSON-RPC request
pub async fn handle_rpc(
    State(state): State<AppState>,
    axum::Extension(user): axum::Extension<Option<AuthenticatedUser>>,
    Json(request): Json<JsonRpcRequest>,
) -> (StatusCode, Json<JsonRpcResponse>) {
    let response = match dispatch_method(&state, &user, &request).await {
        Ok(result) => JsonRpcResponse::success(request.id, result),
        Err(error) => JsonRpcResponse::error(request.id, error),
    };

    (StatusCode::OK, Json(response))
}

/// Dispatch a method call to the appropriate handler
///
/// This function is public so it can be reused by the embedded server
/// in single-process mode without duplicating the dispatch logic.
pub async fn dispatch_method(
    state: &AppState,
    user: &Option<AuthenticatedUser>,
    request: &JsonRpcRequest,
) -> Result<serde_json::Value, JsonRpcError> {
    match request.method.as_str() {
        // Task methods
        method_names::CREATE_UNIT_TASK => {
            let params: CreateUnitTaskRequest = parse_params(&request.params)?;
            handle_create_unit_task(state, params).await
        }
        method_names::GET_UNIT_TASK => {
            let params: GetUnitTaskRequest = parse_params(&request.params)?;
            handle_get_unit_task(state, params).await
        }
        method_names::LIST_UNIT_TASKS => {
            let params: ListUnitTasksRequest =
                parse_params_or_default(&request.params).unwrap_or_default();
            handle_list_unit_tasks(state, params).await
        }
        method_names::UPDATE_UNIT_TASK_STATUS => {
            let params: UpdateUnitTaskStatusRequest = parse_params(&request.params)?;
            handle_update_unit_task_status(state, params).await
        }
        method_names::DELETE_UNIT_TASK => {
            let params: GetUnitTaskRequest = parse_params(&request.params)?;
            handle_delete_unit_task(state, params).await
        }
        method_names::START_TASK_EXECUTION => {
            let params: StartTaskExecutionRequest = parse_params(&request.params)?;
            handle_start_task_execution(state, params).await
        }
        method_names::STOP_TASK_EXECUTION => {
            let params: StopTaskExecutionRequest = parse_params(&request.params)?;
            handle_stop_task_execution(state, params).await
        }

        // Composite task methods
        method_names::CREATE_COMPOSITE_TASK => {
            let params: CreateCompositeTaskRequest = parse_params(&request.params)?;
            handle_create_composite_task(state, params).await
        }
        method_names::GET_COMPOSITE_TASK => {
            let params: GetCompositeTaskRequest = parse_params(&request.params)?;
            handle_get_composite_task(state, params).await
        }
        method_names::APPROVE_COMPOSITE_PLAN => {
            let params: ApproveCompositePlanRequest = parse_params(&request.params)?;
            handle_approve_composite_plan(state, params).await
        }
        method_names::REJECT_COMPOSITE_PLAN => {
            let params: RejectCompositePlanRequest = parse_params(&request.params)?;
            handle_reject_composite_plan(state, params).await
        }

        // Repository methods
        method_names::ADD_REPOSITORY => {
            let params: AddRepositoryRequest = parse_params(&request.params)?;
            handle_add_repository(state, params).await
        }
        method_names::LIST_REPOSITORIES => {
            let params: ListRepositoriesRequest =
                parse_params_or_default(&request.params).unwrap_or_default();
            handle_list_repositories(state, params).await
        }

        // Execution log methods
        method_names::GET_EXECUTION_LOGS => {
            let params: GetExecutionLogsRequest = parse_params(&request.params)?;
            handle_get_execution_logs(state, params).await
        }

        // Secret methods
        method_names::SEND_SECRETS => {
            let params: SendSecretsRequest = parse_params(&request.params)?;
            handle_send_secrets(state, user, params).await
        }

        // Worker methods
        method_names::REGISTER_WORKER => {
            let params: RegisterWorkerRequest = parse_params(&request.params)?;
            handle_register_worker(state, params).await
        }
        method_names::WORKER_HEARTBEAT => {
            let params: WorkerHeartbeatRequest = parse_params(&request.params)?;
            handle_worker_heartbeat(state, params).await
        }
        method_names::REPORT_TASK_COMPLETE => {
            let params: ReportTaskCompleteRequest = parse_params(&request.params)?;
            handle_report_task_complete(state, params).await
        }
        method_names::SEND_EXECUTION_LOG => {
            let params: SendExecutionLogRequest = parse_params(&request.params)?;
            handle_send_execution_log(state, params).await
        }

        method => Err(JsonRpcError::method_not_found(method)),
    }
}

/// Parse JSON-RPC params
fn parse_params<T: serde::de::DeserializeOwned>(
    params: &serde_json::Value,
) -> Result<T, JsonRpcError> {
    serde_json::from_value(params.clone()).map_err(|e| JsonRpcError::invalid_params(e.to_string()))
}

/// Parse params or return default
fn parse_params_or_default<T: serde::de::DeserializeOwned + Default>(
    params: &serde_json::Value,
) -> Result<T, JsonRpcError> {
    if params.is_null() {
        Ok(T::default())
    } else {
        parse_params(params)
    }
}

// ========== Task Handlers ==========

async fn handle_create_unit_task(
    state: &AppState,
    params: CreateUnitTaskRequest,
) -> Result<serde_json::Value, JsonRpcError> {
    let agent_task_id = uuid::Uuid::new_v4().to_string();
    let task = UnitTask::new(
        &params.title,
        &params.prompt,
        &agent_task_id,
        &params.repository_group_id,
    );

    state
        .store
        .create_unit_task(&task)
        .await
        .map_err(|e| JsonRpcError::internal_error(e.to_string()))?;

    info!(task_id = %task.id, "Created unit task");

    let response = CreateUnitTaskResponse { task };
    serde_json::to_value(response).map_err(|e| JsonRpcError::internal_error(e.to_string()))
}

async fn handle_get_unit_task(
    state: &AppState,
    params: GetUnitTaskRequest,
) -> Result<serde_json::Value, JsonRpcError> {
    let task = state
        .store
        .get_unit_task(&params.id)
        .await
        .map_err(|e| JsonRpcError::internal_error(e.to_string()))?;

    let response = GetUnitTaskResponse { task };
    serde_json::to_value(response).map_err(|e| JsonRpcError::internal_error(e.to_string()))
}

async fn handle_list_unit_tasks(
    state: &AppState,
    params: ListUnitTasksRequest,
) -> Result<serde_json::Value, JsonRpcError> {
    let mut filter = TaskFilter::new();
    if let Some(ref group_id) = params.repository_group_id {
        filter = filter.with_repository_group(group_id);
    }
    if let Some(status) = params.status {
        filter = filter.with_status(status);
    }

    let tasks = state
        .store
        .list_unit_tasks(&filter)
        .await
        .map_err(|e| JsonRpcError::internal_error(e.to_string()))?;

    let response = ListUnitTasksResponse { tasks };
    serde_json::to_value(response).map_err(|e| JsonRpcError::internal_error(e.to_string()))
}

async fn handle_update_unit_task_status(
    state: &AppState,
    params: UpdateUnitTaskStatusRequest,
) -> Result<serde_json::Value, JsonRpcError> {
    state
        .store
        .update_unit_task_status(&params.id, params.status)
        .await
        .map_err(|e| JsonRpcError::internal_error(e.to_string()))?;

    info!(task_id = %params.id, status = ?params.status, "Updated task status");

    let response = SuccessResponse::default();
    serde_json::to_value(response).map_err(|e| JsonRpcError::internal_error(e.to_string()))
}

async fn handle_delete_unit_task(
    state: &AppState,
    params: GetUnitTaskRequest,
) -> Result<serde_json::Value, JsonRpcError> {
    state
        .store
        .delete_unit_task(&params.id)
        .await
        .map_err(|e| JsonRpcError::internal_error(e.to_string()))?;

    info!(task_id = %params.id, "Deleted task");

    let response = SuccessResponse::default();
    serde_json::to_value(response).map_err(|e| JsonRpcError::internal_error(e.to_string()))
}

async fn handle_start_task_execution(
    state: &AppState,
    params: StartTaskExecutionRequest,
) -> Result<serde_json::Value, JsonRpcError> {
    // Find a worker to execute the task
    let registry = state.worker_registry.read().await;
    let worker = registry
        .select_worker_for_task()
        .ok_or_else(|| JsonRpcError::internal_error("No workers available".to_string()))?;

    let worker_id = worker.id.clone();
    let session_id = uuid::Uuid::new_v4().to_string();
    drop(registry);

    // Assign task to worker
    let mut registry = state.worker_registry.write().await;
    registry
        .assign_task(&params.task_id, &worker_id)
        .map_err(|e| JsonRpcError::internal_error(e.to_string()))?;

    info!(
        task_id = %params.task_id,
        worker_id = %worker_id,
        session_id = %session_id,
        "Started task execution"
    );

    let response = StartTaskExecutionResponse {
        started: true,
        session_id: Some(session_id),
    };
    serde_json::to_value(response).map_err(|e| JsonRpcError::internal_error(e.to_string()))
}

async fn handle_stop_task_execution(
    state: &AppState,
    params: StopTaskExecutionRequest,
) -> Result<serde_json::Value, JsonRpcError> {
    let mut registry = state.worker_registry.write().await;
    registry.remove_task_assignment(&params.task_id);

    info!(task_id = %params.task_id, "Stopped task execution");

    let response = SuccessResponse::default();
    serde_json::to_value(response).map_err(|e| JsonRpcError::internal_error(e.to_string()))
}

// ========== Composite Task Handlers ==========

async fn handle_create_composite_task(
    state: &AppState,
    params: CreateCompositeTaskRequest,
) -> Result<serde_json::Value, JsonRpcError> {
    let task = CompositeTask::new(
        &params.title,
        &params.prompt,
        "planning-task-id", // Placeholder
        &params.repository_group_id,
    );

    state
        .store
        .create_composite_task(&task)
        .await
        .map_err(|e| JsonRpcError::internal_error(e.to_string()))?;

    info!(task_id = %task.id, "Created composite task");

    let response = CreateCompositeTaskResponse { task };
    serde_json::to_value(response).map_err(|e| JsonRpcError::internal_error(e.to_string()))
}

async fn handle_get_composite_task(
    state: &AppState,
    params: GetCompositeTaskRequest,
) -> Result<serde_json::Value, JsonRpcError> {
    let task = state
        .store
        .get_composite_task(&params.id)
        .await
        .map_err(|e| JsonRpcError::internal_error(e.to_string()))?;

    let response = GetCompositeTaskResponse { task };
    serde_json::to_value(response).map_err(|e| JsonRpcError::internal_error(e.to_string()))
}

async fn handle_approve_composite_plan(
    state: &AppState,
    params: ApproveCompositePlanRequest,
) -> Result<serde_json::Value, JsonRpcError> {
    state
        .store
        .update_composite_task_status(&params.id, CompositeTaskStatus::InProgress)
        .await
        .map_err(|e| JsonRpcError::internal_error(e.to_string()))?;

    info!(task_id = %params.id, "Approved composite task plan");

    let response = SuccessResponse::default();
    serde_json::to_value(response).map_err(|e| JsonRpcError::internal_error(e.to_string()))
}

async fn handle_reject_composite_plan(
    state: &AppState,
    params: RejectCompositePlanRequest,
) -> Result<serde_json::Value, JsonRpcError> {
    state
        .store
        .update_composite_task_status(&params.id, CompositeTaskStatus::Rejected)
        .await
        .map_err(|e| JsonRpcError::internal_error(e.to_string()))?;

    info!(task_id = %params.id, reason = ?params.reason, "Rejected composite task plan");

    let response = SuccessResponse::default();
    serde_json::to_value(response).map_err(|e| JsonRpcError::internal_error(e.to_string()))
}

// ========== Repository Handlers ==========

async fn handle_add_repository(
    state: &AppState,
    params: AddRepositoryRequest,
) -> Result<serde_json::Value, JsonRpcError> {
    let repo = Repository::new(&params.name, &params.remote_url, &params.local_path);

    state
        .store
        .create_repository(&repo)
        .await
        .map_err(|e| JsonRpcError::internal_error(e.to_string()))?;

    info!(repo_id = %repo.id, name = %repo.name, "Added repository");

    let response = AddRepositoryResponse { repository: repo };
    serde_json::to_value(response).map_err(|e| JsonRpcError::internal_error(e.to_string()))
}

async fn handle_list_repositories(
    state: &AppState,
    _params: ListRepositoriesRequest,
) -> Result<serde_json::Value, JsonRpcError> {
    let repositories = state
        .store
        .list_repositories()
        .await
        .map_err(|e| JsonRpcError::internal_error(e.to_string()))?;

    let response = ListRepositoriesResponse { repositories };
    serde_json::to_value(response).map_err(|e| JsonRpcError::internal_error(e.to_string()))
}

// ========== Execution Log Handlers ==========

async fn handle_get_execution_logs(
    _state: &AppState,
    _params: GetExecutionLogsRequest,
) -> Result<serde_json::Value, JsonRpcError> {
    // For now, return empty logs (would be stored in database in full
    // implementation)
    let response = GetExecutionLogsResponse { logs: Vec::new() };
    serde_json::to_value(response).map_err(|e| JsonRpcError::internal_error(e.to_string()))
}

// ========== Secret Handlers ==========

async fn handle_send_secrets(
    _state: &AppState,
    _user: &Option<AuthenticatedUser>,
    params: SendSecretsRequest,
) -> Result<serde_json::Value, JsonRpcError> {
    // Store secrets for the task (would be forwarded to worker when task starts)
    // For now, just acknowledge receipt
    info!(task_id = %params.task_id, secret_count = %params.secrets.len(), "Received secrets");

    let response = SendSecretsResponse { accepted: true };
    serde_json::to_value(response).map_err(|e| JsonRpcError::internal_error(e.to_string()))
}

// ========== Worker Handlers ==========

async fn handle_register_worker(
    state: &AppState,
    params: RegisterWorkerRequest,
) -> Result<serde_json::Value, JsonRpcError> {
    let mut registry = state.worker_registry.write().await;
    registry.register(params.worker_id.clone(), params.address, params.capacity);

    info!(worker_id = %params.worker_id, "Registered worker");

    let response = RegisterWorkerResponse {
        registered: true,
        worker_id: params.worker_id,
    };
    serde_json::to_value(response).map_err(|e| JsonRpcError::internal_error(e.to_string()))
}

async fn handle_worker_heartbeat(
    state: &AppState,
    params: WorkerHeartbeatRequest,
) -> Result<serde_json::Value, JsonRpcError> {
    let mut registry = state.worker_registry.write().await;
    registry
        .heartbeat(&params.worker_id, params.current_load)
        .map_err(|e| JsonRpcError::internal_error(e.to_string()))?;

    let response = WorkerHeartbeatResponse { accepted: true };
    serde_json::to_value(response).map_err(|e| JsonRpcError::internal_error(e.to_string()))
}

async fn handle_report_task_complete(
    state: &AppState,
    params: ReportTaskCompleteRequest,
) -> Result<serde_json::Value, JsonRpcError> {
    // Update task status
    let new_status = if params.success {
        UnitTaskStatus::InReview
    } else {
        UnitTaskStatus::InProgress // Keep in progress for retry
    };

    state
        .store
        .update_unit_task_status(&params.task_id, new_status)
        .await
        .map_err(|e| JsonRpcError::internal_error(e.to_string()))?;

    // Remove task assignment
    let mut registry = state.worker_registry.write().await;
    registry.remove_task_assignment(&params.task_id);

    info!(
        task_id = %params.task_id,
        success = %params.success,
        "Task completed"
    );

    let response = SuccessResponse::default();
    serde_json::to_value(response).map_err(|e| JsonRpcError::internal_error(e.to_string()))
}

async fn handle_send_execution_log(
    state: &AppState,
    params: SendExecutionLogRequest,
) -> Result<serde_json::Value, JsonRpcError> {
    // Broadcast log to subscribers
    state
        .log_broadcaster
        .broadcast(&params.task_id, &params.session_id, params.message);

    let response = SuccessResponse::default();
    serde_json::to_value(response).map_err(|e| JsonRpcError::internal_error(e.to_string()))
}
