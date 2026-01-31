//! RPC response types.

use serde::{Deserialize, Serialize};

use crate::types::*;

// ============================================================================
// Task Service Responses
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateUnitTaskResponse {
    pub task: UnitTask,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCompositeTaskResponse {
    pub task: CompositeTask,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum GetTaskResponse {
    UnitTask { unit_task: UnitTask },
    CompositeTask { composite_task: CompositeTask },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListTasksResponse {
    pub unit_tasks: Vec<UnitTask>,
    pub composite_tasks: Vec<CompositeTask>,
    pub total_count: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum UpdateTaskStatusResponse {
    UnitTask { unit_task: UnitTask },
    CompositeTask { composite_task: CompositeTask },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteTaskResponse {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryTaskResponse {
    pub task: UnitTask,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApproveTaskResponse {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RejectTaskResponse {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestChangesResponse {
    pub task: UnitTask,
}

// ============================================================================
// Session Service Responses
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetLogResponse {
    pub log: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StopSessionResponse {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmitTtyInputResponse {}

// ============================================================================
// Repository Service Responses
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddRepositoryResponse {
    pub repository: Repository,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListRepositoriesResponse {
    pub repositories: Vec<Repository>,
    pub total_count: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetRepositoryResponse {
    pub repository: Repository,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoveRepositoryResponse {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateRepositoryGroupResponse {
    pub group: RepositoryGroup,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListRepositoryGroupsResponse {
    pub groups: Vec<RepositoryGroup>,
    pub total_count: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateRepositoryGroupResponse {
    pub group: RepositoryGroup,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteRepositoryGroupResponse {}

// ============================================================================
// Workspace Service Responses
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateWorkspaceResponse {
    pub workspace: Workspace,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListWorkspacesResponse {
    pub workspaces: Vec<Workspace>,
    pub total_count: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetWorkspaceResponse {
    pub workspace: Workspace,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateWorkspaceResponse {
    pub workspace: Workspace,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteWorkspaceResponse {}

// ============================================================================
// Todo Service Responses
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListTodoItemsResponse {
    pub items: Vec<TodoItem>,
    pub total_count: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetTodoItemResponse {
    pub item: TodoItem,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateTodoStatusResponse {
    pub item: TodoItem,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DismissTodoResponse {}

// ============================================================================
// Secrets Service Responses
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendSecretsResponse {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClearSecretsResponse {}

// ============================================================================
// Auth Service Responses
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetLoginUrlResponse {
    pub login_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandleCallbackResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: i64,
    pub user: User,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshTokenResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetCurrentUserResponse {
    pub user: User,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogoutResponse {}

// ============================================================================
// Worker Service Responses
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterWorkerResponse {
    pub worker_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatResponse {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnregisterWorkerResponse {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetNextTaskResponse {
    pub task: Option<UnitTask>,
    pub agent_task: Option<AgentTask>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportTaskStatusResponse {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetSecretsResponse {
    pub secrets: Vec<Secret>,
}
