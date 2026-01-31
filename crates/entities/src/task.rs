//! Task-related entity definitions.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::AiAgentType;

/// Status of a UnitTask.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UnitTaskStatus {
    /// AI is working.
    InProgress,
    /// AI work complete, awaiting human review.
    InReview,
    /// Human approved.
    Approved,
    /// PR created.
    PrOpen,
    /// PR merged.
    Done,
    /// Rejected and discarded.
    Rejected,
}

impl Default for UnitTaskStatus {
    fn default() -> Self {
        Self::InProgress
    }
}

/// A single task unit visible to users.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnitTask {
    /// Unique identifier.
    pub id: Uuid,
    /// Associated RepositoryGroup ID.
    pub repository_group_id: Uuid,
    /// Associated AgentTask ID.
    pub agent_task_id: Uuid,
    /// Task prompt.
    pub prompt: String,
    /// Optional task title.
    pub title: Option<String>,
    /// Custom branch name.
    pub branch_name: Option<String>,
    /// Created PR URL.
    pub linked_pr_url: Option<String>,
    /// Base commit hash.
    pub base_commit: Option<String>,
    /// End commit hash.
    pub end_commit: Option<String>,
    /// Auto-fix AgentTask IDs.
    pub auto_fix_task_ids: Vec<Uuid>,
    /// Current status.
    pub status: UnitTaskStatus,
    /// When this record was created.
    pub created_at: DateTime<Utc>,
    /// When this record was last updated.
    pub updated_at: DateTime<Utc>,
}

impl UnitTask {
    /// Creates a new unit task.
    pub fn new(repository_group_id: Uuid, agent_task_id: Uuid, prompt: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            repository_group_id,
            agent_task_id,
            prompt: prompt.into(),
            title: None,
            branch_name: None,
            linked_pr_url: None,
            base_commit: None,
            end_commit: None,
            auto_fix_task_ids: Vec::new(),
            status: UnitTaskStatus::InProgress,
            created_at: now,
            updated_at: now,
        }
    }

    /// Sets the title for this task.
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Sets the branch name for this task.
    pub fn with_branch_name(mut self, branch_name: impl Into<String>) -> Self {
        self.branch_name = Some(branch_name.into());
        self
    }
}

/// Status of a CompositeTask.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompositeTaskStatus {
    /// Generating PLAN.yaml.
    Planning,
    /// Waiting for user approval.
    PendingApproval,
    /// Tasks are executing.
    InProgress,
    /// All tasks completed.
    Done,
    /// User rejected the plan.
    Rejected,
}

impl Default for CompositeTaskStatus {
    fn default() -> Self {
        Self::Planning
    }
}

/// A node in a CompositeTask graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompositeTaskNode {
    /// Unique identifier.
    pub id: Uuid,
    /// Associated CompositeTask ID.
    pub composite_task_id: Uuid,
    /// Associated UnitTask ID.
    pub unit_task_id: Uuid,
    /// IDs of nodes this node depends on.
    pub depends_on_ids: Vec<Uuid>,
    /// When this record was created.
    pub created_at: DateTime<Utc>,
}

impl CompositeTaskNode {
    /// Creates a new composite task node.
    pub fn new(composite_task_id: Uuid, unit_task_id: Uuid) -> Self {
        Self {
            id: Uuid::new_v4(),
            composite_task_id,
            unit_task_id,
            depends_on_ids: Vec::new(),
            created_at: Utc::now(),
        }
    }

    /// Adds a dependency to this node.
    pub fn depends_on(&mut self, node_id: Uuid) {
        self.depends_on_ids.push(node_id);
    }
}

/// Task graph-based Agent Orchestrator.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompositeTask {
    /// Unique identifier.
    pub id: Uuid,
    /// Associated RepositoryGroup ID.
    pub repository_group_id: Uuid,
    /// AgentTask ID for generating PLAN.yaml.
    pub planning_task_id: Uuid,
    /// Task prompt.
    pub prompt: String,
    /// Optional task title.
    pub title: Option<String>,
    /// List of task node IDs.
    pub node_ids: Vec<Uuid>,
    /// Current status.
    pub status: CompositeTaskStatus,
    /// Agent type for UnitTasks.
    pub execution_agent_type: Option<AiAgentType>,
    /// When this record was created.
    pub created_at: DateTime<Utc>,
    /// When this record was last updated.
    pub updated_at: DateTime<Utc>,
}

impl CompositeTask {
    /// Creates a new composite task.
    pub fn new(
        repository_group_id: Uuid,
        planning_task_id: Uuid,
        prompt: impl Into<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            repository_group_id,
            planning_task_id,
            prompt: prompt.into(),
            title: None,
            node_ids: Vec::new(),
            status: CompositeTaskStatus::Planning,
            execution_agent_type: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Sets the title for this task.
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Sets the execution agent type.
    pub fn with_execution_agent_type(mut self, agent_type: AiAgentType) -> Self {
        self.execution_agent_type = Some(agent_type);
        self
    }
}

/// A generic task type that can be either a UnitTask or CompositeTask.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Task {
    /// A unit task.
    Unit(UnitTask),
    /// A composite task.
    Composite(CompositeTask),
}

impl Task {
    /// Returns the task ID.
    pub fn id(&self) -> Uuid {
        match self {
            Task::Unit(t) => t.id,
            Task::Composite(t) => t.id,
        }
    }

    /// Returns the repository group ID.
    pub fn repository_group_id(&self) -> Uuid {
        match self {
            Task::Unit(t) => t.repository_group_id,
            Task::Composite(t) => t.repository_group_id,
        }
    }

    /// Returns the task title if set.
    pub fn title(&self) -> Option<&str> {
        match self {
            Task::Unit(t) => t.title.as_deref(),
            Task::Composite(t) => t.title.as_deref(),
        }
    }

    /// Returns the task prompt.
    pub fn prompt(&self) -> &str {
        match self {
            Task::Unit(t) => &t.prompt,
            Task::Composite(t) => &t.prompt,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unit_task_creation() {
        let repo_group_id = Uuid::new_v4();
        let agent_task_id = Uuid::new_v4();
        let task = UnitTask::new(repo_group_id, agent_task_id, "Fix the bug")
            .with_title("Bug Fix")
            .with_branch_name("fix/bug-123");

        assert_eq!(task.repository_group_id, repo_group_id);
        assert_eq!(task.agent_task_id, agent_task_id);
        assert_eq!(task.prompt, "Fix the bug");
        assert_eq!(task.title, Some("Bug Fix".to_string()));
        assert_eq!(task.branch_name, Some("fix/bug-123".to_string()));
        assert_eq!(task.status, UnitTaskStatus::InProgress);
    }

    #[test]
    fn test_composite_task_creation() {
        let repo_group_id = Uuid::new_v4();
        let planning_task_id = Uuid::new_v4();
        let task =
            CompositeTask::new(repo_group_id, planning_task_id, "Implement authentication")
                .with_title("Auth System")
                .with_execution_agent_type(AiAgentType::ClaudeCode);

        assert_eq!(task.repository_group_id, repo_group_id);
        assert_eq!(task.planning_task_id, planning_task_id);
        assert_eq!(task.prompt, "Implement authentication");
        assert_eq!(task.title, Some("Auth System".to_string()));
        assert_eq!(task.status, CompositeTaskStatus::Planning);
        assert_eq!(task.execution_agent_type, Some(AiAgentType::ClaudeCode));
    }

    #[test]
    fn test_task_enum() {
        let repo_group_id = Uuid::new_v4();
        let agent_task_id = Uuid::new_v4();
        let unit_task = UnitTask::new(repo_group_id, agent_task_id, "Test prompt");
        let task = Task::Unit(unit_task.clone());

        assert_eq!(task.id(), unit_task.id);
        assert_eq!(task.repository_group_id(), repo_group_id);
        assert_eq!(task.prompt(), "Test prompt");
    }
}
