//! RPC type definitions.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// VCS type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VcsType {
    Unspecified,
    Git,
}

/// VCS provider type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VcsProviderType {
    Unspecified,
    Github,
    Gitlab,
    Bitbucket,
}

/// AI agent type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AiAgentType {
    Unspecified,
    ClaudeCode,
    OpenCode,
    GeminiCli,
    CodexCli,
    Aider,
    Amp,
}

/// Unit task status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UnitTaskStatus {
    Unspecified,
    InProgress,
    InReview,
    Approved,
    PrOpen,
    Done,
    Rejected,
}

/// Composite task status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompositeTaskStatus {
    Unspecified,
    Planning,
    PendingApproval,
    InProgress,
    Done,
    Rejected,
}

/// TTY input type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TtyInputType {
    Unspecified,
    Text,
    Select,
    Confirm,
    Password,
}

/// TTY input status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TtyInputStatus {
    Unspecified,
    Pending,
    Responded,
    Timeout,
    Cancelled,
}

/// Worker status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkerStatus {
    Unspecified,
    Idle,
    Busy,
    Unhealthy,
}

/// Todo item type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TodoItemType {
    Unspecified,
    IssueTriage,
    PrReview,
}

/// Todo item status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TodoItemStatus {
    Unspecified,
    Pending,
    InProgress,
    Completed,
    Dismissed,
}

/// Base remote information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseRemote {
    pub git_remote_dir_path: String,
    pub git_branch_name: String,
}

/// Agent session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSession {
    pub id: String,
    pub agent_task_id: String,
    pub ai_agent_type: AiAgentType,
    pub ai_agent_model: Option<String>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub output_log: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Agent task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTask {
    pub id: String,
    pub base_remotes: Vec<BaseRemote>,
    pub agent_sessions: Vec<AgentSession>,
    pub ai_agent_type: Option<AiAgentType>,
    pub ai_agent_model: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Unit task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnitTask {
    pub id: String,
    pub repository_group_id: String,
    pub agent_task_id: String,
    pub prompt: String,
    pub title: Option<String>,
    pub branch_name: Option<String>,
    pub linked_pr_url: Option<String>,
    pub base_commit: Option<String>,
    pub end_commit: Option<String>,
    pub auto_fix_task_ids: Vec<String>,
    pub status: UnitTaskStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Composite task node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompositeTaskNode {
    pub id: String,
    pub composite_task_id: String,
    pub unit_task_id: String,
    pub depends_on_ids: Vec<String>,
    pub created_at: DateTime<Utc>,
}

/// Composite task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompositeTask {
    pub id: String,
    pub repository_group_id: String,
    pub planning_task_id: String,
    pub prompt: String,
    pub title: Option<String>,
    pub node_ids: Vec<String>,
    pub status: CompositeTaskStatus,
    pub execution_agent_type: Option<AiAgentType>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Repository.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Repository {
    pub id: String,
    pub workspace_id: String,
    pub name: String,
    pub remote_url: String,
    pub default_branch: String,
    pub vcs_type: VcsType,
    pub vcs_provider_type: VcsProviderType,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Repository group.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryGroup {
    pub id: String,
    pub workspace_id: String,
    pub name: Option<String>,
    pub repository_ids: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Workspace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workspace {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub user_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// User.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub email: String,
    pub name: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// TTY input request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TtyInputRequest {
    pub id: String,
    pub task_id: String,
    pub session_id: String,
    pub prompt: String,
    pub input_type: TtyInputType,
    pub options: Vec<String>,
    pub status: TtyInputStatus,
    pub response: Option<String>,
    pub created_at: DateTime<Utc>,
    pub responded_at: Option<DateTime<Utc>>,
}

/// Worker.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Worker {
    pub id: String,
    pub name: String,
    pub endpoint_url: String,
    pub status: WorkerStatus,
    pub last_heartbeat: DateTime<Utc>,
    pub current_task_id: Option<String>,
    pub registered_at: DateTime<Utc>,
}

/// Issue triage data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueTriageData {
    pub issue_url: String,
    pub issue_title: String,
    pub suggested_labels: Vec<String>,
    pub suggested_assignees: Vec<String>,
}

/// PR review data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrReviewData {
    pub pr_url: String,
    pub pr_title: String,
    pub changed_files_count: i32,
    pub ai_summary: Option<String>,
}

/// Todo item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoItem {
    pub id: String,
    pub item_type: TodoItemType,
    pub status: TodoItemStatus,
    pub repository_id: String,
    pub issue_triage: Option<IssueTriageData>,
    pub pr_review: Option<PrReviewData>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Secret key-value pair.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Secret {
    pub key: String,
    pub value: String,
}
