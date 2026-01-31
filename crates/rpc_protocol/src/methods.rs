//! RPC method definitions and request/response types

use std::collections::HashMap;

use coding_agents::{AgentType, NormalizedMessage};
use serde::{Deserialize, Serialize};
use task_store::UnitTaskStatus;

/// All RPC method names
#[allow(dead_code)]
pub mod method_names {
    // Task methods
    pub const CREATE_UNIT_TASK: &str = "createUnitTask";
    pub const GET_UNIT_TASK: &str = "getUnitTask";
    pub const LIST_UNIT_TASKS: &str = "listUnitTasks";
    pub const UPDATE_UNIT_TASK_STATUS: &str = "updateUnitTaskStatus";
    pub const DELETE_UNIT_TASK: &str = "deleteUnitTask";
    pub const START_TASK_EXECUTION: &str = "startTaskExecution";
    pub const STOP_TASK_EXECUTION: &str = "stopTaskExecution";

    // Composite task methods
    pub const CREATE_COMPOSITE_TASK: &str = "createCompositeTask";
    pub const GET_COMPOSITE_TASK: &str = "getCompositeTask";
    pub const LIST_COMPOSITE_TASKS: &str = "listCompositeTasks";
    pub const APPROVE_COMPOSITE_PLAN: &str = "approveCompositePlan";
    pub const REJECT_COMPOSITE_PLAN: &str = "rejectCompositePlan";

    // Repository methods
    pub const ADD_REPOSITORY: &str = "addRepository";
    pub const GET_REPOSITORY: &str = "getRepository";
    pub const LIST_REPOSITORIES: &str = "listRepositories";
    pub const REMOVE_REPOSITORY: &str = "removeRepository";

    // Repository group methods
    pub const CREATE_REPOSITORY_GROUP: &str = "createRepositoryGroup";
    pub const GET_REPOSITORY_GROUP: &str = "getRepositoryGroup";
    pub const LIST_REPOSITORY_GROUPS: &str = "listRepositoryGroups";

    // Workspace methods
    pub const CREATE_WORKSPACE: &str = "createWorkspace";
    pub const LIST_WORKSPACES: &str = "listWorkspaces";

    // Execution log methods
    pub const GET_EXECUTION_LOGS: &str = "getExecutionLogs";
    pub const SUBSCRIBE_EXECUTION_LOGS: &str = "subscribeExecutionLogs";
    pub const UNSUBSCRIBE_EXECUTION_LOGS: &str = "unsubscribeExecutionLogs";

    // Secret methods
    pub const SEND_SECRETS: &str = "sendSecrets";

    // Worker methods (internal)
    pub const REGISTER_WORKER: &str = "registerWorker";
    pub const WORKER_HEARTBEAT: &str = "workerHeartbeat";
    pub const ASSIGN_TASK: &str = "assignTask";
    pub const REPORT_TASK_COMPLETE: &str = "reportTaskComplete";
    pub const SEND_EXECUTION_LOG: &str = "sendExecutionLog";
}

// ========== Task Requests/Responses ==========

/// Request to create a unit task
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateUnitTaskRequest {
    /// Repository group ID
    pub repository_group_id: String,
    /// Task title
    pub title: String,
    /// Task prompt
    pub prompt: String,
    /// Optional custom branch name
    pub branch_name: Option<String>,
    /// Optional agent type override
    pub agent_type: Option<AgentType>,
    /// Optional model override
    pub model: Option<String>,
}

/// Response for create unit task
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateUnitTaskResponse {
    /// The created task
    pub task: task_store::UnitTask,
}

/// Request to get a unit task
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetUnitTaskRequest {
    /// Task ID
    pub id: String,
}

/// Response for get unit task
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetUnitTaskResponse {
    /// The task (if found)
    pub task: Option<task_store::UnitTask>,
}

/// Request to list unit tasks
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListUnitTasksRequest {
    /// Optional repository group ID filter
    pub repository_group_id: Option<String>,
    /// Optional status filter
    pub status: Option<UnitTaskStatus>,
    /// Optional limit
    pub limit: Option<u32>,
    /// Optional offset
    pub offset: Option<u32>,
}

/// Response for list unit tasks
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListUnitTasksResponse {
    /// The tasks
    pub tasks: Vec<task_store::UnitTask>,
}

/// Request to update unit task status
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateUnitTaskStatusRequest {
    /// Task ID
    pub id: String,
    /// New status
    pub status: UnitTaskStatus,
}

/// Request to start task execution
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartTaskExecutionRequest {
    /// Task ID
    pub task_id: String,
}

/// Response for start task execution
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartTaskExecutionResponse {
    /// Whether execution started successfully
    pub started: bool,
    /// Session ID for tracking
    pub session_id: Option<String>,
}

/// Request to stop task execution
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StopTaskExecutionRequest {
    /// Task ID
    pub task_id: String,
}

// ========== Composite Task Requests/Responses ==========

/// Request to create a composite task
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateCompositeTaskRequest {
    /// Repository group ID
    pub repository_group_id: String,
    /// Task title
    pub title: String,
    /// Task prompt
    pub prompt: String,
    /// Optional execution agent type
    pub execution_agent_type: Option<AgentType>,
}

/// Response for create composite task
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateCompositeTaskResponse {
    /// The created task
    pub task: task_store::CompositeTask,
}

/// Request to get a composite task
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetCompositeTaskRequest {
    /// Task ID
    pub id: String,
}

/// Response for get composite task
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetCompositeTaskResponse {
    /// The task (if found)
    pub task: Option<task_store::CompositeTask>,
}

/// Request to approve a composite task plan
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApproveCompositePlanRequest {
    /// Task ID
    pub id: String,
}

/// Request to reject a composite task plan
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RejectCompositePlanRequest {
    /// Task ID
    pub id: String,
    /// Optional rejection reason
    pub reason: Option<String>,
}

// ========== Repository Requests/Responses ==========

/// Request to add a repository
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddRepositoryRequest {
    /// Repository name
    pub name: String,
    /// Remote URL
    pub remote_url: String,
    /// Local path
    pub local_path: String,
    /// Default branch
    pub default_branch: Option<String>,
    /// VCS provider type
    pub vcs_provider_type: Option<String>,
}

/// Response for add repository
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddRepositoryResponse {
    /// The created repository
    pub repository: task_store::Repository,
}

/// Request to list repositories
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListRepositoriesRequest {
    /// Optional workspace ID filter
    pub workspace_id: Option<String>,
}

/// Response for list repositories
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListRepositoriesResponse {
    /// The repositories
    pub repositories: Vec<task_store::Repository>,
}

// ========== Execution Log Requests/Responses ==========

/// Request to get execution logs
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetExecutionLogsRequest {
    /// Session ID
    pub session_id: String,
    /// Optional offset
    pub offset: Option<u32>,
    /// Optional limit
    pub limit: Option<u32>,
}

/// Response for get execution logs
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetExecutionLogsResponse {
    /// The logs
    pub logs: Vec<NormalizedMessage>,
}

/// Request to subscribe to execution logs
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubscribeExecutionLogsRequest {
    /// Task ID
    pub task_id: String,
}

/// Execution log notification (sent over WebSocket)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionLogNotification {
    /// Task ID
    pub task_id: String,
    /// Session ID
    pub session_id: String,
    /// The log message
    pub message: NormalizedMessage,
}

// ========== Secret Requests/Responses ==========

/// Request to send secrets for task execution
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendSecretsRequest {
    /// Task ID
    pub task_id: String,
    /// Secrets (key -> value)
    pub secrets: HashMap<String, String>,
}

/// Response for send secrets
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendSecretsResponse {
    /// Whether secrets were accepted
    pub accepted: bool,
}

// ========== Worker Requests/Responses ==========

/// Request to register a worker
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisterWorkerRequest {
    /// Worker ID
    pub worker_id: String,
    /// Worker address
    pub address: String,
    /// Worker capacity
    pub capacity: WorkerCapacity,
}

/// Worker capacity information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkerCapacity {
    /// Maximum concurrent tasks
    pub max_concurrent_tasks: u32,
    /// Available memory in bytes
    pub available_memory: u64,
    /// Available CPU cores
    pub available_cpus: u32,
}

/// Response for register worker
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisterWorkerResponse {
    /// Whether registration was successful
    pub registered: bool,
    /// Assigned worker ID (may be different from requested)
    pub worker_id: String,
}

/// Request for worker heartbeat
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkerHeartbeatRequest {
    /// Worker ID
    pub worker_id: String,
    /// Current load
    pub current_load: WorkerLoad,
}

/// Worker load information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkerLoad {
    /// Current number of running tasks
    pub running_tasks: u32,
    /// CPU usage percentage (0-100)
    pub cpu_usage: u8,
    /// Memory usage percentage (0-100)
    pub memory_usage: u8,
}

/// Response for worker heartbeat
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkerHeartbeatResponse {
    /// Whether the heartbeat was accepted
    pub accepted: bool,
}

/// Request to assign a task to a worker
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssignTaskRequest {
    /// Task ID
    pub task_id: String,
    /// Secrets for the task
    pub secrets: HashMap<String, String>,
}

/// Response for assign task
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssignTaskResponse {
    /// Whether the task was accepted
    pub accepted: bool,
}

/// Request to report task completion
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportTaskCompleteRequest {
    /// Task ID
    pub task_id: String,
    /// Whether execution was successful
    pub success: bool,
    /// Summary of what was done
    pub summary: Option<String>,
    /// Exit code
    pub exit_code: Option<i32>,
}

/// Request to send an execution log
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendExecutionLogRequest {
    /// Task ID
    pub task_id: String,
    /// Session ID
    pub session_id: String,
    /// The log message
    pub message: NormalizedMessage,
}

// ========== Generic Response ==========

/// Generic success response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SuccessResponse {
    /// Whether the operation was successful
    pub success: bool,
}

impl Default for SuccessResponse {
    fn default() -> Self {
        Self { success: true }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_unit_task_request_serialization() {
        let request = CreateUnitTaskRequest {
            repository_group_id: "group-1".to_string(),
            title: "Test Task".to_string(),
            prompt: "Do something".to_string(),
            branch_name: None,
            agent_type: Some(AgentType::ClaudeCode),
            model: None,
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("repositoryGroupId"));
        assert!(json.contains("Test Task"));
    }

    #[test]
    fn test_worker_capacity_serialization() {
        let capacity = WorkerCapacity {
            max_concurrent_tasks: 4,
            available_memory: 8 * 1024 * 1024 * 1024,
            available_cpus: 8,
        };

        let json = serde_json::to_string(&capacity).unwrap();
        let deserialized: WorkerCapacity = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.max_concurrent_tasks, 4);
    }
}
