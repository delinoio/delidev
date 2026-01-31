//! Embedded Server for Single Process Mode
//!
//! This module provides an embedded server that handles JSON-RPC requests
//! locally without network communication. It uses the same RPC dispatch
//! logic as the standalone server but runs entirely in-process.

use std::sync::Arc;

use rpc_protocol::{JsonRpcError, JsonRpcRequest, JsonRpcResponse};
use task_store::{MemoryStore, TaskStore};
use tokio::sync::RwLock;

use super::{SingleProcessConfig, SingleProcessError};

/// Embedded server for single-process mode
///
/// This server handles JSON-RPC requests locally without network I/O.
/// It provides the same functionality as the standalone server but
/// communicates via direct function calls instead of HTTP/WebSocket.
pub struct EmbeddedServer {
    /// Task store (SQLite or in-memory)
    store: Arc<dyn TaskStore>,

    /// Log broadcaster for streaming execution logs
    log_broadcaster: Arc<delidev_server::LogBroadcaster>,

    /// Worker registry (for tracking embedded worker)
    worker_registry: Arc<RwLock<delidev_server::WorkerRegistry>>,

    /// Configuration
    config: SingleProcessConfig,
}

impl EmbeddedServer {
    /// Creates a new embedded server
    pub async fn new(config: &SingleProcessConfig) -> Result<Self, SingleProcessError> {
        tracing::info!("Initializing embedded server for single-process mode");

        // Initialize store - use in-memory for now, SQLite can be added later
        // In single-process mode, the desktop app's SQLite database is the source of truth
        let store: Arc<dyn TaskStore> = Arc::new(MemoryStore::new());

        // Initialize log broadcaster
        let log_broadcaster = Arc::new(delidev_server::LogBroadcaster::new());

        // Initialize worker registry with 60 second timeout
        let worker_registry = Arc::new(RwLock::new(delidev_server::WorkerRegistry::new(60)));

        Ok(Self {
            store,
            log_broadcaster,
            worker_registry,
            config: config.clone(),
        })
    }

    /// Returns the task store
    pub fn store(&self) -> &Arc<dyn TaskStore> {
        &self.store
    }

    /// Returns the log broadcaster
    pub fn log_broadcaster(&self) -> &Arc<delidev_server::LogBroadcaster> {
        &self.log_broadcaster
    }

    /// Returns the worker registry
    pub fn worker_registry(&self) -> &Arc<RwLock<delidev_server::WorkerRegistry>> {
        &self.worker_registry
    }

    /// Handles a JSON-RPC request locally
    ///
    /// This method dispatches the request to the appropriate handler
    /// without any network communication.
    pub async fn handle_rpc(&self, request: JsonRpcRequest) -> JsonRpcResponse {
        // Create a temporary AppState for the server RPC handler
        // In a full implementation, this would be cached and reused
        let server_config = delidev_server::ServerConfig {
            single_user_mode: true,
            ..Default::default()
        };

        match delidev_server::AppState::new(server_config).await {
            Ok(state) => {
                // Use the server's RPC handling logic
                // For now, we return a placeholder response
                // The actual implementation would call into the server's rpc module
                self.dispatch_rpc(&state, request).await
            }
            Err(e) => JsonRpcResponse::error(
                request.id,
                JsonRpcError::internal_error(format!("Failed to create server state: {}", e)),
            ),
        }
    }

    /// Dispatches an RPC request to the appropriate handler
    async fn dispatch_rpc(
        &self,
        state: &delidev_server::AppState,
        request: JsonRpcRequest,
    ) -> JsonRpcResponse {
        // The server crate's rpc module handles all the dispatching
        // We need to call into it directly here
        use rpc_protocol::method_names;

        let result: Result<serde_json::Value, JsonRpcError> = match request.method.as_str() {
            // Task methods
            method_names::CREATE_UNIT_TASK => {
                self.handle_create_unit_task(state, &request.params).await
            }
            method_names::GET_UNIT_TASK => self.handle_get_unit_task(state, &request.params).await,
            method_names::LIST_UNIT_TASKS => {
                self.handle_list_unit_tasks(state, &request.params).await
            }
            method_names::UPDATE_UNIT_TASK_STATUS => {
                self.handle_update_unit_task_status(state, &request.params)
                    .await
            }
            method_names::DELETE_UNIT_TASK => {
                self.handle_delete_unit_task(state, &request.params).await
            }
            method_names::START_TASK_EXECUTION => {
                self.handle_start_task_execution(&request.params).await
            }
            method_names::STOP_TASK_EXECUTION => {
                self.handle_stop_task_execution(&request.params).await
            }

            // Composite task methods
            method_names::CREATE_COMPOSITE_TASK => {
                self.handle_create_composite_task(state, &request.params)
                    .await
            }
            method_names::GET_COMPOSITE_TASK => {
                self.handle_get_composite_task(state, &request.params).await
            }
            method_names::APPROVE_COMPOSITE_PLAN => {
                self.handle_approve_composite_plan(state, &request.params)
                    .await
            }
            method_names::REJECT_COMPOSITE_PLAN => {
                self.handle_reject_composite_plan(state, &request.params)
                    .await
            }

            // Repository methods
            method_names::ADD_REPOSITORY => {
                self.handle_add_repository(state, &request.params).await
            }
            method_names::LIST_REPOSITORIES => {
                self.handle_list_repositories(state, &request.params).await
            }

            // Execution log methods
            method_names::GET_EXECUTION_LOGS => {
                self.handle_get_execution_logs(&request.params).await
            }

            // Secret methods - in single process mode, secrets are already local
            method_names::SEND_SECRETS => {
                Ok(serde_json::json!({ "accepted": true }))
            }

            // Worker methods - not needed in single process mode
            method_names::REGISTER_WORKER
            | method_names::WORKER_HEARTBEAT
            | method_names::REPORT_TASK_COMPLETE
            | method_names::SEND_EXECUTION_LOG => {
                // These are internal methods used by remote workers
                // In single-process mode, we handle them as no-ops
                Ok(serde_json::json!({ "success": true }))
            }

            method => Err(JsonRpcError::method_not_found(method)),
        };

        match result {
            Ok(value) => JsonRpcResponse::success(request.id, value),
            Err(error) => JsonRpcResponse::error(request.id, error),
        }
    }

    // ========== Task Handlers ==========

    async fn handle_create_unit_task(
        &self,
        state: &delidev_server::AppState,
        params: &serde_json::Value,
    ) -> Result<serde_json::Value, JsonRpcError> {
        use rpc_protocol::{CreateUnitTaskRequest, CreateUnitTaskResponse};
        use task_store::UnitTask;

        let params: CreateUnitTaskRequest =
            serde_json::from_value(params.clone()).map_err(|e| JsonRpcError::invalid_params(e.to_string()))?;

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

        tracing::info!(task_id = %task.id, "Created unit task in single-process mode");

        let response = CreateUnitTaskResponse { task };
        serde_json::to_value(response).map_err(|e| JsonRpcError::internal_error(e.to_string()))
    }

    async fn handle_get_unit_task(
        &self,
        state: &delidev_server::AppState,
        params: &serde_json::Value,
    ) -> Result<serde_json::Value, JsonRpcError> {
        use rpc_protocol::{GetUnitTaskRequest, GetUnitTaskResponse};

        let params: GetUnitTaskRequest =
            serde_json::from_value(params.clone()).map_err(|e| JsonRpcError::invalid_params(e.to_string()))?;

        let task = state
            .store
            .get_unit_task(&params.id)
            .await
            .map_err(|e| JsonRpcError::internal_error(e.to_string()))?;

        let response = GetUnitTaskResponse { task };
        serde_json::to_value(response).map_err(|e| JsonRpcError::internal_error(e.to_string()))
    }

    async fn handle_list_unit_tasks(
        &self,
        state: &delidev_server::AppState,
        params: &serde_json::Value,
    ) -> Result<serde_json::Value, JsonRpcError> {
        use rpc_protocol::{ListUnitTasksRequest, ListUnitTasksResponse};
        use task_store::TaskFilter;

        let params: ListUnitTasksRequest = if params.is_null() {
            ListUnitTasksRequest::default()
        } else {
            serde_json::from_value(params.clone()).map_err(|e| JsonRpcError::invalid_params(e.to_string()))?
        };

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
        &self,
        state: &delidev_server::AppState,
        params: &serde_json::Value,
    ) -> Result<serde_json::Value, JsonRpcError> {
        use rpc_protocol::{SuccessResponse, UpdateUnitTaskStatusRequest};

        let params: UpdateUnitTaskStatusRequest =
            serde_json::from_value(params.clone()).map_err(|e| JsonRpcError::invalid_params(e.to_string()))?;

        state
            .store
            .update_unit_task_status(&params.id, params.status)
            .await
            .map_err(|e| JsonRpcError::internal_error(e.to_string()))?;

        tracing::info!(task_id = %params.id, status = ?params.status, "Updated task status");

        let response = SuccessResponse::default();
        serde_json::to_value(response).map_err(|e| JsonRpcError::internal_error(e.to_string()))
    }

    async fn handle_delete_unit_task(
        &self,
        state: &delidev_server::AppState,
        params: &serde_json::Value,
    ) -> Result<serde_json::Value, JsonRpcError> {
        use rpc_protocol::{GetUnitTaskRequest, SuccessResponse};

        let params: GetUnitTaskRequest =
            serde_json::from_value(params.clone()).map_err(|e| JsonRpcError::invalid_params(e.to_string()))?;

        state
            .store
            .delete_unit_task(&params.id)
            .await
            .map_err(|e| JsonRpcError::internal_error(e.to_string()))?;

        tracing::info!(task_id = %params.id, "Deleted task");

        let response = SuccessResponse::default();
        serde_json::to_value(response).map_err(|e| JsonRpcError::internal_error(e.to_string()))
    }

    async fn handle_start_task_execution(
        &self,
        params: &serde_json::Value,
    ) -> Result<serde_json::Value, JsonRpcError> {
        use rpc_protocol::{StartTaskExecutionRequest, StartTaskExecutionResponse};

        let params: StartTaskExecutionRequest =
            serde_json::from_value(params.clone()).map_err(|e| JsonRpcError::invalid_params(e.to_string()))?;

        // In single-process mode, task execution is handled by the desktop app's
        // AgentExecutionService directly. This RPC just signals that execution should start.
        let session_id = uuid::Uuid::new_v4().to_string();

        tracing::info!(
            task_id = %params.task_id,
            session_id = %session_id,
            "Starting task execution in single-process mode"
        );

        let response = StartTaskExecutionResponse {
            started: true,
            session_id: Some(session_id),
        };
        serde_json::to_value(response).map_err(|e| JsonRpcError::internal_error(e.to_string()))
    }

    async fn handle_stop_task_execution(
        &self,
        params: &serde_json::Value,
    ) -> Result<serde_json::Value, JsonRpcError> {
        use rpc_protocol::{StopTaskExecutionRequest, SuccessResponse};

        let params: StopTaskExecutionRequest =
            serde_json::from_value(params.clone()).map_err(|e| JsonRpcError::invalid_params(e.to_string()))?;

        tracing::info!(task_id = %params.task_id, "Stopping task execution");

        let response = SuccessResponse::default();
        serde_json::to_value(response).map_err(|e| JsonRpcError::internal_error(e.to_string()))
    }

    // ========== Composite Task Handlers ==========

    async fn handle_create_composite_task(
        &self,
        state: &delidev_server::AppState,
        params: &serde_json::Value,
    ) -> Result<serde_json::Value, JsonRpcError> {
        use rpc_protocol::{CreateCompositeTaskRequest, CreateCompositeTaskResponse};
        use task_store::CompositeTask;

        let params: CreateCompositeTaskRequest =
            serde_json::from_value(params.clone()).map_err(|e| JsonRpcError::invalid_params(e.to_string()))?;

        let task = CompositeTask::new(
            &params.title,
            &params.prompt,
            "planning-task-id",
            &params.repository_group_id,
        );

        state
            .store
            .create_composite_task(&task)
            .await
            .map_err(|e| JsonRpcError::internal_error(e.to_string()))?;

        tracing::info!(task_id = %task.id, "Created composite task");

        let response = CreateCompositeTaskResponse { task };
        serde_json::to_value(response).map_err(|e| JsonRpcError::internal_error(e.to_string()))
    }

    async fn handle_get_composite_task(
        &self,
        state: &delidev_server::AppState,
        params: &serde_json::Value,
    ) -> Result<serde_json::Value, JsonRpcError> {
        use rpc_protocol::{GetCompositeTaskRequest, GetCompositeTaskResponse};

        let params: GetCompositeTaskRequest =
            serde_json::from_value(params.clone()).map_err(|e| JsonRpcError::invalid_params(e.to_string()))?;

        let task = state
            .store
            .get_composite_task(&params.id)
            .await
            .map_err(|e| JsonRpcError::internal_error(e.to_string()))?;

        let response = GetCompositeTaskResponse { task };
        serde_json::to_value(response).map_err(|e| JsonRpcError::internal_error(e.to_string()))
    }

    async fn handle_approve_composite_plan(
        &self,
        state: &delidev_server::AppState,
        params: &serde_json::Value,
    ) -> Result<serde_json::Value, JsonRpcError> {
        use rpc_protocol::{ApproveCompositePlanRequest, SuccessResponse};
        use task_store::CompositeTaskStatus;

        let params: ApproveCompositePlanRequest =
            serde_json::from_value(params.clone()).map_err(|e| JsonRpcError::invalid_params(e.to_string()))?;

        state
            .store
            .update_composite_task_status(&params.id, CompositeTaskStatus::InProgress)
            .await
            .map_err(|e| JsonRpcError::internal_error(e.to_string()))?;

        tracing::info!(task_id = %params.id, "Approved composite task plan");

        let response = SuccessResponse::default();
        serde_json::to_value(response).map_err(|e| JsonRpcError::internal_error(e.to_string()))
    }

    async fn handle_reject_composite_plan(
        &self,
        state: &delidev_server::AppState,
        params: &serde_json::Value,
    ) -> Result<serde_json::Value, JsonRpcError> {
        use rpc_protocol::{RejectCompositePlanRequest, SuccessResponse};
        use task_store::CompositeTaskStatus;

        let params: RejectCompositePlanRequest =
            serde_json::from_value(params.clone()).map_err(|e| JsonRpcError::invalid_params(e.to_string()))?;

        state
            .store
            .update_composite_task_status(&params.id, CompositeTaskStatus::Rejected)
            .await
            .map_err(|e| JsonRpcError::internal_error(e.to_string()))?;

        tracing::info!(task_id = %params.id, reason = ?params.reason, "Rejected composite task plan");

        let response = SuccessResponse::default();
        serde_json::to_value(response).map_err(|e| JsonRpcError::internal_error(e.to_string()))
    }

    // ========== Repository Handlers ==========

    async fn handle_add_repository(
        &self,
        state: &delidev_server::AppState,
        params: &serde_json::Value,
    ) -> Result<serde_json::Value, JsonRpcError> {
        use rpc_protocol::{AddRepositoryRequest, AddRepositoryResponse};
        use task_store::Repository;

        let params: AddRepositoryRequest =
            serde_json::from_value(params.clone()).map_err(|e| JsonRpcError::invalid_params(e.to_string()))?;

        let repo = Repository::new(&params.name, &params.remote_url, &params.local_path);

        state
            .store
            .create_repository(&repo)
            .await
            .map_err(|e| JsonRpcError::internal_error(e.to_string()))?;

        tracing::info!(repo_id = %repo.id, name = %repo.name, "Added repository");

        let response = AddRepositoryResponse { repository: repo };
        serde_json::to_value(response).map_err(|e| JsonRpcError::internal_error(e.to_string()))
    }

    async fn handle_list_repositories(
        &self,
        state: &delidev_server::AppState,
        _params: &serde_json::Value,
    ) -> Result<serde_json::Value, JsonRpcError> {
        use rpc_protocol::ListRepositoriesResponse;

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
        &self,
        _params: &serde_json::Value,
    ) -> Result<serde_json::Value, JsonRpcError> {
        use rpc_protocol::GetExecutionLogsResponse;

        // In single-process mode, logs are streamed via Tauri events
        // Historical logs would be retrieved from the desktop app's log storage
        let response = GetExecutionLogsResponse { logs: Vec::new() };
        serde_json::to_value(response).map_err(|e| JsonRpcError::internal_error(e.to_string()))
    }
}
