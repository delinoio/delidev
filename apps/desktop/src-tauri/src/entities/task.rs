use serde::{Deserialize, Serialize};

use super::AIAgentType;

/// UnitTask status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
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
    /// Base commit hash of the default branch when task was created
    /// Used to compute accurate diff even after default branch advances
    pub base_commit: Option<String>,
    /// End commit hash of the task branch when task completed
    /// Used with base_commit to compute accurate diff of only this task's
    /// changes
    pub end_commit: Option<String>,
    /// List of auto-fix attempt AgentTask IDs
    pub auto_fix_task_ids: Vec<String>,
    /// Current status
    pub status: UnitTaskStatus,
    /// Repository Group ID
    pub repository_group_id: String,
    /// Created timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Updated timestamp
    pub updated_at: chrono::DateTime<chrono::Utc>,
    /// Whether the task's container is currently running
    /// This field is populated by the API layer when retrieving tasks
    /// and reflects the actual execution state
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_executing: Option<bool>,
    /// Parent CompositeTask ID if this UnitTask belongs to a CompositeTask
    /// This field is populated dynamically when fetching the task
    #[serde(skip_serializing_if = "Option::is_none")]
    pub composite_task_id: Option<String>,
    /// Whether the last execution attempt failed
    /// Used to persist execution failure state across page refreshes
    #[serde(default)]
    pub last_execution_failed: bool,
}

impl UnitTask {
    pub fn new(
        id: String,
        title: String,
        prompt: String,
        agent_task_id: String,
        repository_group_id: String,
    ) -> Self {
        let now = chrono::Utc::now();
        Self {
            id,
            title,
            prompt,
            agent_task_id,
            branch_name: None,
            linked_pr_url: None,
            base_commit: None,
            end_commit: None,
            auto_fix_task_ids: Vec::new(),
            status: UnitTaskStatus::default(),
            repository_group_id,
            created_at: now,
            updated_at: now,
            is_executing: None,
            composite_task_id: None,
            last_execution_failed: false,
        }
    }

    pub fn with_branch_name(mut self, branch_name: impl Into<String>) -> Self {
        self.branch_name = Some(branch_name.into());
        self
    }
}

/// CompositeTask status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum CompositeTaskStatus {
    /// planningTask is generating PLAN.yaml
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
    pub fn new(id: String, unit_task_id: String) -> Self {
        Self {
            id,
            unit_task_id,
            depends_on: Vec::new(),
        }
    }

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
    /// Plan file path (PLAN-{random}.yaml)
    pub plan_file_path: Option<String>,
    /// Plan YAML content (stored in DB for persistence after file is removed)
    pub plan_yaml_content: Option<String>,
    /// AI agent type for executing UnitTasks (uses global default if not
    /// specified)
    pub execution_agent_type: Option<AIAgentType>,
    /// Created timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Updated timestamp
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl CompositeTask {
    pub fn new(
        id: String,
        title: String,
        prompt: String,
        planning_task_id: String,
        repository_group_id: String,
    ) -> Self {
        let now = chrono::Utc::now();
        Self {
            id,
            title,
            prompt,
            planning_task_id,
            nodes: Vec::new(),
            status: CompositeTaskStatus::default(),
            repository_group_id,
            plan_file_path: None,
            plan_yaml_content: None,
            execution_agent_type: None,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn with_execution_agent_type(mut self, agent_type: AIAgentType) -> Self {
        self.execution_agent_type = Some(agent_type);
        self
    }

    pub fn add_node(&mut self, node: CompositeTaskNode) {
        self.nodes.push(node);
    }

    /// Returns nodes that can be executed (no pending dependencies)
    pub fn executable_nodes(&self, completed_node_ids: &[String]) -> Vec<&CompositeTaskNode> {
        self.nodes
            .iter()
            .filter(|node| {
                // Node is executable if all its dependencies are completed
                node.depends_on
                    .iter()
                    .all(|dep| completed_node_ids.contains(dep))
                    && !completed_node_ids.contains(&node.id)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod composite_task {
        use super::*;

        #[test]
        fn test_executable_nodes_returns_all_when_no_dependencies() {
            let mut task = CompositeTask::new(
                "comp-1".to_string(),
                "Complex Task".to_string(),
                "Build a feature".to_string(),
                "planning-1".to_string(),
                "repo-1".to_string(),
            );

            task.add_node(CompositeTaskNode::new(
                "node-1".to_string(),
                "unit-1".to_string(),
            ));
            task.add_node(CompositeTaskNode::new(
                "node-2".to_string(),
                "unit-2".to_string(),
            ));

            let executable = task.executable_nodes(&[]);
            assert_eq!(executable.len(), 2);
        }

        #[test]
        fn test_executable_nodes_respects_dependency_chain() {
            let mut task = CompositeTask::new(
                "comp-1".to_string(),
                "Complex Task".to_string(),
                "Build a feature".to_string(),
                "planning-1".to_string(),
                "repo-1".to_string(),
            );

            // node-1 has no dependencies
            task.add_node(CompositeTaskNode::new(
                "node-1".to_string(),
                "unit-1".to_string(),
            ));

            // node-2 depends on node-1
            task.add_node(
                CompositeTaskNode::new("node-2".to_string(), "unit-2".to_string())
                    .with_dependencies(vec!["node-1".to_string()]),
            );

            // node-3 depends on node-1 and node-2
            task.add_node(
                CompositeTaskNode::new("node-3".to_string(), "unit-3".to_string())
                    .with_dependencies(vec!["node-1".to_string(), "node-2".to_string()]),
            );

            // Initially only node-1 is executable
            let executable = task.executable_nodes(&[]);
            assert_eq!(executable.len(), 1);
            assert_eq!(executable[0].id, "node-1");

            // After node-1 completes, node-2 becomes executable
            let executable = task.executable_nodes(&["node-1".to_string()]);
            assert_eq!(executable.len(), 1);
            assert_eq!(executable[0].id, "node-2");

            // After node-1 and node-2 complete, node-3 becomes executable
            let executable = task.executable_nodes(&["node-1".to_string(), "node-2".to_string()]);
            assert_eq!(executable.len(), 1);
            assert_eq!(executable[0].id, "node-3");
        }

        #[test]
        fn test_executable_nodes_excludes_already_completed_nodes() {
            let mut task = CompositeTask::new(
                "comp-1".to_string(),
                "Complex Task".to_string(),
                "Build a feature".to_string(),
                "planning-1".to_string(),
                "repo-1".to_string(),
            );

            task.add_node(CompositeTaskNode::new(
                "node-1".to_string(),
                "unit-1".to_string(),
            ));
            task.add_node(CompositeTaskNode::new(
                "node-2".to_string(),
                "unit-2".to_string(),
            ));

            let executable = task.executable_nodes(&["node-1".to_string(), "node-2".to_string()]);
            assert!(executable.is_empty());
        }
    }
}
