//! Entity types for the task store

use chrono::{DateTime, Utc};
use coding_agents::AgentType;
use serde::{Deserialize, Serialize};

/// UnitTask status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum UnitTaskStatus {
    /// AI is working
    #[default]
    InProgress,
    /// AI work complete, awaiting human review
    InReview,
    /// Human approved, ready to merge or create PR
    Approved,
    /// PR created, awaiting merge
    PrOpen,
    /// PR merged
    Done,
    /// Rejected and discarded
    Rejected,
}

impl UnitTaskStatus {
    /// Converts the status to a string for storage
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::InProgress => "in_progress",
            Self::InReview => "in_review",
            Self::Approved => "approved",
            Self::PrOpen => "pr_open",
            Self::Done => "done",
            Self::Rejected => "rejected",
        }
    }

    /// Parses a status from a string
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "in_progress" => Some(Self::InProgress),
            "in_review" => Some(Self::InReview),
            "approved" => Some(Self::Approved),
            "pr_open" => Some(Self::PrOpen),
            "done" => Some(Self::Done),
            "rejected" => Some(Self::Rejected),
            _ => None,
        }
    }
}

/// A single task unit visible to users
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnitTask {
    /// Unique identifier
    pub id: String,
    /// Task title/description
    pub title: String,
    /// Task prompt for the AI agent
    pub prompt: String,
    /// Associated AgentTask ID
    pub agent_task_id: String,
    /// Custom branch name (uses template if not specified)
    pub branch_name: Option<String>,
    /// Created PR URL
    pub linked_pr_url: Option<String>,
    /// Base commit hash
    pub base_commit: Option<String>,
    /// End commit hash
    pub end_commit: Option<String>,
    /// Current status
    pub status: UnitTaskStatus,
    /// Repository Group ID
    pub repository_group_id: String,
    /// List of auto-fix attempt AgentTask IDs
    pub auto_fix_task_ids: Vec<String>,
    /// Parent CompositeTask ID (if part of a composite task)
    pub composite_task_id: Option<String>,
    /// Whether the last execution failed
    pub last_execution_failed: bool,
    /// Created timestamp
    pub created_at: DateTime<Utc>,
    /// Updated timestamp
    pub updated_at: DateTime<Utc>,
}

impl UnitTask {
    /// Creates a new unit task
    pub fn new(
        title: impl Into<String>,
        prompt: impl Into<String>,
        agent_task_id: impl Into<String>,
        repository_group_id: impl Into<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            title: title.into(),
            prompt: prompt.into(),
            agent_task_id: agent_task_id.into(),
            branch_name: None,
            linked_pr_url: None,
            base_commit: None,
            end_commit: None,
            status: UnitTaskStatus::default(),
            repository_group_id: repository_group_id.into(),
            auto_fix_task_ids: Vec::new(),
            composite_task_id: None,
            last_execution_failed: false,
            created_at: now,
            updated_at: now,
        }
    }

    /// Sets the branch name
    pub fn with_branch_name(mut self, branch_name: impl Into<String>) -> Self {
        self.branch_name = Some(branch_name.into());
        self
    }
}

/// CompositeTask status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum CompositeTaskStatus {
    /// Planning task is generating PLAN.yaml
    #[default]
    Planning,
    /// Waiting for user approval
    PendingApproval,
    /// Tasks are executing
    InProgress,
    /// All tasks completed
    Done,
    /// User rejected the plan
    Rejected,
}

impl CompositeTaskStatus {
    /// Converts the status to a string for storage
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Planning => "planning",
            Self::PendingApproval => "pending_approval",
            Self::InProgress => "in_progress",
            Self::Done => "done",
            Self::Rejected => "rejected",
        }
    }

    /// Parses a status from a string
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "planning" => Some(Self::Planning),
            "pending_approval" => Some(Self::PendingApproval),
            "in_progress" => Some(Self::InProgress),
            "done" => Some(Self::Done),
            "rejected" => Some(Self::Rejected),
            _ => None,
        }
    }
}

/// A task node belonging to a CompositeTask
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompositeTaskNode {
    /// Unique identifier (within the plan)
    pub id: String,
    /// Associated UnitTask ID
    pub unit_task_id: String,
    /// List of dependent node IDs
    pub depends_on: Vec<String>,
}

impl CompositeTaskNode {
    /// Creates a new node
    pub fn new(id: impl Into<String>, unit_task_id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            unit_task_id: unit_task_id.into(),
            depends_on: Vec::new(),
        }
    }

    /// Sets the dependencies
    pub fn with_dependencies(mut self, deps: Vec<String>) -> Self {
        self.depends_on = deps;
        self
    }
}

/// Task graph-based Agent Orchestrator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompositeTask {
    /// Unique identifier
    pub id: String,
    /// Task title/description
    pub title: String,
    /// Original prompt from user
    pub prompt: String,
    /// AgentTask ID for generating PLAN.yaml
    pub planning_task_id: String,
    /// List of task nodes
    pub nodes: Vec<CompositeTaskNode>,
    /// Current status
    pub status: CompositeTaskStatus,
    /// Repository Group ID
    pub repository_group_id: String,
    /// Plan file path
    pub plan_file_path: Option<String>,
    /// Plan YAML content
    pub plan_yaml_content: Option<String>,
    /// AI agent type for executing UnitTasks
    pub execution_agent_type: Option<AgentType>,
    /// Created timestamp
    pub created_at: DateTime<Utc>,
    /// Updated timestamp
    pub updated_at: DateTime<Utc>,
}

impl CompositeTask {
    /// Creates a new composite task
    pub fn new(
        title: impl Into<String>,
        prompt: impl Into<String>,
        planning_task_id: impl Into<String>,
        repository_group_id: impl Into<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            title: title.into(),
            prompt: prompt.into(),
            planning_task_id: planning_task_id.into(),
            nodes: Vec::new(),
            status: CompositeTaskStatus::default(),
            repository_group_id: repository_group_id.into(),
            plan_file_path: None,
            plan_yaml_content: None,
            execution_agent_type: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Adds a node to the task
    pub fn add_node(&mut self, node: CompositeTaskNode) {
        self.nodes.push(node);
    }

    /// Returns nodes that can be executed (no pending dependencies)
    pub fn executable_nodes(&self, completed_node_ids: &[String]) -> Vec<&CompositeTaskNode> {
        self.nodes
            .iter()
            .filter(|node| {
                node.depends_on
                    .iter()
                    .all(|dep| completed_node_ids.contains(dep))
                    && !completed_node_ids.contains(&node.id)
            })
            .collect()
    }
}

/// Version Control System types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum VcsType {
    #[default]
    Git,
}

/// VCS hosting provider types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum VcsProviderType {
    #[default]
    Github,
    Gitlab,
    Bitbucket,
}

impl VcsProviderType {
    /// Converts to string for storage
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Github => "github",
            Self::Gitlab => "gitlab",
            Self::Bitbucket => "bitbucket",
        }
    }

    /// Parses from string
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "github" => Some(Self::Github),
            "gitlab" => Some(Self::Gitlab),
            "bitbucket" => Some(Self::Bitbucket),
            _ => None,
        }
    }
}

/// A managed repository
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Repository {
    /// Unique identifier
    pub id: String,
    /// Version control system type
    pub vcs_type: VcsType,
    /// VCS hosting provider type
    pub vcs_provider_type: VcsProviderType,
    /// Remote URL
    pub remote_url: String,
    /// Repository name
    pub name: String,
    /// Local filesystem path
    pub local_path: String,
    /// Default branch name
    pub default_branch: String,
    /// Created timestamp
    pub created_at: DateTime<Utc>,
}

impl Repository {
    /// Creates a new repository
    pub fn new(
        name: impl Into<String>,
        remote_url: impl Into<String>,
        local_path: impl Into<String>,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            vcs_type: VcsType::default(),
            vcs_provider_type: VcsProviderType::default(),
            remote_url: remote_url.into(),
            name: name.into(),
            local_path: local_path.into(),
            default_branch: "main".to_string(),
            created_at: Utc::now(),
        }
    }
}

/// A logical grouping of repositories
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workspace {
    /// Unique identifier
    pub id: String,
    /// Workspace name
    pub name: String,
    /// Optional description
    pub description: Option<String>,
    /// Created timestamp
    pub created_at: DateTime<Utc>,
    /// Updated timestamp
    pub updated_at: DateTime<Utc>,
}

impl Workspace {
    /// Creates a new workspace
    pub fn new(name: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.into(),
            description: None,
            created_at: now,
            updated_at: now,
        }
    }
}

/// A group of repositories that tasks operate on
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryGroup {
    /// Unique identifier
    pub id: String,
    /// Group name (None for single-repo groups)
    pub name: Option<String>,
    /// Parent workspace ID
    pub workspace_id: String,
    /// List of repository IDs
    pub repository_ids: Vec<String>,
    /// Created timestamp
    pub created_at: DateTime<Utc>,
    /// Updated timestamp
    pub updated_at: DateTime<Utc>,
}

impl RepositoryGroup {
    /// Creates a new repository group
    pub fn new(workspace_id: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: None,
            workspace_id: workspace_id.into(),
            repository_ids: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }

    /// Sets the group name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Adds a repository to the group
    pub fn add_repository(&mut self, repository_id: impl Into<String>) {
        self.repository_ids.push(repository_id.into());
    }
}

/// Filter for listing tasks
#[derive(Debug, Clone, Default)]
pub struct TaskFilter {
    /// Filter by repository group ID
    pub repository_group_id: Option<String>,
    /// Filter by status
    pub status: Option<UnitTaskStatus>,
    /// Filter by composite task ID
    pub composite_task_id: Option<String>,
    /// Limit number of results
    pub limit: Option<u32>,
    /// Offset for pagination
    pub offset: Option<u32>,
}

impl TaskFilter {
    /// Creates a new empty filter
    pub fn new() -> Self {
        Self::default()
    }

    /// Filters by repository group
    pub fn with_repository_group(mut self, id: impl Into<String>) -> Self {
        self.repository_group_id = Some(id.into());
        self
    }

    /// Filters by status
    pub fn with_status(mut self, status: UnitTaskStatus) -> Self {
        self.status = Some(status);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unit_task_creation() {
        let task = UnitTask::new("Test Task", "Do something", "agent-1", "repo-group-1")
            .with_branch_name("feature/test");

        assert_eq!(task.title, "Test Task");
        assert_eq!(task.branch_name, Some("feature/test".to_string()));
        assert_eq!(task.status, UnitTaskStatus::InProgress);
    }

    #[test]
    fn test_composite_task_executable_nodes() {
        let mut task = CompositeTask::new("Complex Task", "Build feature", "planning-1", "repo-1");

        task.add_node(CompositeTaskNode::new("node-1", "unit-1"));
        task.add_node(
            CompositeTaskNode::new("node-2", "unit-2")
                .with_dependencies(vec!["node-1".to_string()]),
        );

        // Initially only node-1 is executable
        let executable = task.executable_nodes(&[]);
        assert_eq!(executable.len(), 1);
        assert_eq!(executable[0].id, "node-1");

        // After node-1 completes, node-2 becomes executable
        let executable = task.executable_nodes(&["node-1".to_string()]);
        assert_eq!(executable.len(), 1);
        assert_eq!(executable[0].id, "node-2");
    }

    #[test]
    fn test_status_serialization() {
        assert_eq!(UnitTaskStatus::InProgress.as_str(), "in_progress");
        assert_eq!(
            UnitTaskStatus::parse("in_progress"),
            Some(UnitTaskStatus::InProgress)
        );
    }
}
