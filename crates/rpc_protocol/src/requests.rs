//! RPC request types.

use serde::{Deserialize, Serialize};

use crate::types::*;

// ============================================================================
// Task Service Requests
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateUnitTaskRequest {
    pub repository_group_id: String,
    pub prompt: String,
    pub title: Option<String>,
    pub branch_name: Option<String>,
    pub ai_agent_type: Option<AiAgentType>,
    pub ai_agent_model: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCompositeTaskRequest {
    pub repository_group_id: String,
    pub prompt: String,
    pub title: Option<String>,
    pub execution_agent_type: Option<AiAgentType>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetTaskRequest {
    pub task_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListTasksRequest {
    pub repository_group_id: Option<String>,
    pub unit_status: Option<UnitTaskStatus>,
    pub composite_status: Option<CompositeTaskStatus>,
    pub limit: i32,
    pub offset: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateTaskStatusRequest {
    pub task_id: String,
    pub unit_status: Option<UnitTaskStatus>,
    pub composite_status: Option<CompositeTaskStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteTaskRequest {
    pub task_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryTaskRequest {
    pub task_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApproveTaskRequest {
    pub task_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RejectTaskRequest {
    pub task_id: String,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestChangesRequest {
    pub task_id: String,
    pub feedback: String,
}

// ============================================================================
// Session Service Requests
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetLogRequest {
    pub session_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StopSessionRequest {
    pub session_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmitTtyInputRequest {
    pub request_id: String,
    pub response: String,
}

// ============================================================================
// Repository Service Requests
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddRepositoryRequest {
    pub workspace_id: String,
    pub remote_url: String,
    pub name: Option<String>,
    pub default_branch: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListRepositoriesRequest {
    pub workspace_id: Option<String>,
    pub limit: i32,
    pub offset: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetRepositoryRequest {
    pub repository_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoveRepositoryRequest {
    pub repository_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateRepositoryGroupRequest {
    pub workspace_id: String,
    pub name: Option<String>,
    pub repository_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListRepositoryGroupsRequest {
    pub workspace_id: Option<String>,
    pub limit: i32,
    pub offset: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateRepositoryGroupRequest {
    pub group_id: String,
    pub name: Option<String>,
    pub repository_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteRepositoryGroupRequest {
    pub group_id: String,
}

// ============================================================================
// Workspace Service Requests
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateWorkspaceRequest {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListWorkspacesRequest {
    pub limit: i32,
    pub offset: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetWorkspaceRequest {
    pub workspace_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateWorkspaceRequest {
    pub workspace_id: String,
    pub name: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteWorkspaceRequest {
    pub workspace_id: String,
}

// ============================================================================
// Todo Service Requests
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListTodoItemsRequest {
    pub repository_id: Option<String>,
    pub status: Option<TodoItemStatus>,
    pub limit: i32,
    pub offset: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetTodoItemRequest {
    pub item_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateTodoStatusRequest {
    pub item_id: String,
    pub status: TodoItemStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DismissTodoRequest {
    pub item_id: String,
}

// ============================================================================
// Secrets Service Requests
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendSecretsRequest {
    pub task_id: String,
    pub secrets: Vec<Secret>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClearSecretsRequest {
    pub task_id: String,
}

// ============================================================================
// Auth Service Requests
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetLoginUrlRequest {
    pub redirect_uri: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandleCallbackRequest {
    pub code: String,
    pub state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshTokenRequest {
    pub refresh_token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetCurrentUserRequest {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogoutRequest {}

// ============================================================================
// Worker Service Requests
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterWorkerRequest {
    pub name: String,
    pub endpoint_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatRequest {
    pub worker_id: String,
    pub status: WorkerStatus,
    pub current_task_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnregisterWorkerRequest {
    pub worker_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetNextTaskRequest {
    pub worker_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportTaskStatusRequest {
    pub worker_id: String,
    pub task_id: String,
    pub status: UnitTaskStatus,
    pub output_log: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetSecretsRequest {
    pub worker_id: String,
    pub task_id: String,
}
