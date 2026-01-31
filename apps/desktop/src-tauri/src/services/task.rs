use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use crate::{
    database::{
        ai_agent_type_to_string, composite_task_status_to_string, unit_task_status_to_string,
        AgentSessionRow, AgentTaskRow, BaseRemoteRow, CompositeTaskNodeRow, CompositeTaskRow,
        Database, DatabaseResult, UnitTaskRow,
    },
    entities::{
        AgentSession, AgentTask, CompositeTask, CompositeTaskNode, CompositeTaskStatus, UnitTask,
        UnitTaskStatus,
    },
};

/// Service for managing tasks
pub struct TaskService {
    db: Arc<Database>,
}

impl TaskService {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    // ========== UnitTask Operations ==========

    /// Lists all unit tasks, optionally filtered by repository group
    pub async fn list_unit_tasks(
        &self,
        repository_group_id: Option<&str>,
    ) -> DatabaseResult<Vec<UnitTask>> {
        let rows: Vec<UnitTaskRow> = if let Some(group_id) = repository_group_id {
            sqlx::query_as(
                "SELECT id, title, prompt, agent_task_id, branch_name, linked_pr_url, \
                 base_commit, end_commit, status, repository_group_id, created_at, updated_at, \
                 last_execution_failed
                 FROM unit_tasks
                 WHERE repository_group_id = ?
                 ORDER BY created_at DESC",
            )
            .bind(group_id)
            .fetch_all(self.db.pool())
            .await?
        } else {
            sqlx::query_as(
                "SELECT id, title, prompt, agent_task_id, branch_name, linked_pr_url, \
                 base_commit, end_commit, status, repository_group_id, created_at, updated_at, \
                 last_execution_failed
                 FROM unit_tasks
                 ORDER BY created_at DESC",
            )
            .fetch_all(self.db.pool())
            .await?
        };

        let mut tasks = Vec::new();
        for row in rows {
            let auto_fix_ids = self.get_auto_fix_task_ids(&row.id).await?;
            let composite_task_id = self.get_composite_task_id_for_unit_task(&row.id).await?;
            tasks.push(row.into_unit_task(auto_fix_ids, composite_task_id));
        }

        Ok(tasks)
    }

    /// Gets a unit task by ID
    pub async fn get_unit_task(&self, id: &str) -> DatabaseResult<Option<UnitTask>> {
        let row: Option<UnitTaskRow> = sqlx::query_as(
            "SELECT id, title, prompt, agent_task_id, branch_name, linked_pr_url, base_commit, \
             end_commit, status, repository_group_id, created_at, updated_at, \
             last_execution_failed
             FROM unit_tasks
             WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(self.db.pool())
        .await?;

        match row {
            Some(r) => {
                let auto_fix_ids = self.get_auto_fix_task_ids(&r.id).await?;
                let composite_task_id = self.get_composite_task_id_for_unit_task(&r.id).await?;
                Ok(Some(r.into_unit_task(auto_fix_ids, composite_task_id)))
            }
            None => Ok(None),
        }
    }

    /// Creates a new unit task
    pub async fn create_unit_task(&self, task: &UnitTask) -> DatabaseResult<()> {
        let status = unit_task_status_to_string(task.status);

        sqlx::query(
            "INSERT INTO unit_tasks (id, title, prompt, agent_task_id, branch_name, \
             linked_pr_url, base_commit, end_commit, status, repository_group_id, created_at, \
             updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&task.id)
        .bind(&task.title)
        .bind(&task.prompt)
        .bind(&task.agent_task_id)
        .bind(&task.branch_name)
        .bind(&task.linked_pr_url)
        .bind(&task.base_commit)
        .bind(&task.end_commit)
        .bind(status)
        .bind(&task.repository_group_id)
        .bind(task.created_at.to_rfc3339())
        .bind(task.updated_at.to_rfc3339())
        .execute(self.db.pool())
        .await?;

        Ok(())
    }

    /// Updates unit task status
    pub async fn update_unit_task_status(
        &self,
        id: &str,
        status: UnitTaskStatus,
    ) -> DatabaseResult<()> {
        let status_str = unit_task_status_to_string(status);
        let now = chrono::Utc::now().to_rfc3339();

        sqlx::query("UPDATE unit_tasks SET status = ?, updated_at = ? WHERE id = ?")
            .bind(status_str)
            .bind(&now)
            .bind(id)
            .execute(self.db.pool())
            .await?;

        Ok(())
    }

    /// Updates unit task PR URL
    pub async fn update_unit_task_pr_url(&self, id: &str, pr_url: &str) -> DatabaseResult<()> {
        let now = chrono::Utc::now().to_rfc3339();

        sqlx::query("UPDATE unit_tasks SET linked_pr_url = ?, updated_at = ? WHERE id = ?")
            .bind(pr_url)
            .bind(&now)
            .bind(id)
            .execute(self.db.pool())
            .await?;

        Ok(())
    }

    /// Updates unit task base commit hash
    pub async fn update_unit_task_base_commit(
        &self,
        id: &str,
        base_commit: &str,
    ) -> DatabaseResult<()> {
        let now = chrono::Utc::now().to_rfc3339();

        sqlx::query("UPDATE unit_tasks SET base_commit = ?, updated_at = ? WHERE id = ?")
            .bind(base_commit)
            .bind(&now)
            .bind(id)
            .execute(self.db.pool())
            .await?;

        Ok(())
    }

    /// Updates unit task end commit hash
    pub async fn update_unit_task_end_commit(
        &self,
        id: &str,
        end_commit: &str,
    ) -> DatabaseResult<()> {
        let now = chrono::Utc::now().to_rfc3339();

        sqlx::query("UPDATE unit_tasks SET end_commit = ?, updated_at = ? WHERE id = ?")
            .bind(end_commit)
            .bind(&now)
            .bind(id)
            .execute(self.db.pool())
            .await?;

        Ok(())
    }

    /// Updates unit task branch name
    pub async fn update_unit_task_branch_name(
        &self,
        id: &str,
        branch_name: &str,
    ) -> DatabaseResult<()> {
        let now = chrono::Utc::now().to_rfc3339();

        sqlx::query("UPDATE unit_tasks SET branch_name = ?, updated_at = ? WHERE id = ?")
            .bind(branch_name)
            .bind(&now)
            .bind(id)
            .execute(self.db.pool())
            .await?;

        Ok(())
    }

    /// Updates unit task prompt
    pub async fn update_unit_task_prompt(&self, id: &str, prompt: &str) -> DatabaseResult<()> {
        let now = chrono::Utc::now().to_rfc3339();

        sqlx::query("UPDATE unit_tasks SET prompt = ?, updated_at = ? WHERE id = ?")
            .bind(prompt)
            .bind(&now)
            .bind(id)
            .execute(self.db.pool())
            .await?;

        Ok(())
    }

    /// Updates the last_execution_failed flag for a unit task
    pub async fn update_unit_task_execution_failed(
        &self,
        id: &str,
        failed: bool,
    ) -> DatabaseResult<()> {
        let now = chrono::Utc::now().to_rfc3339();
        let failed_int: i32 = if failed { 1 } else { 0 };

        sqlx::query("UPDATE unit_tasks SET last_execution_failed = ?, updated_at = ? WHERE id = ?")
            .bind(failed_int)
            .bind(&now)
            .bind(id)
            .execute(self.db.pool())
            .await?;

        Ok(())
    }

    /// Checks if a unit task is referenced by any composite task
    pub async fn is_unit_task_in_composite(&self, id: &str) -> DatabaseResult<bool> {
        let count: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM composite_task_nodes WHERE unit_task_id = ?")
                .bind(id)
                .fetch_one(self.db.pool())
                .await?;

        Ok(count.0 > 0)
    }

    /// Gets the composite task ID that a unit task belongs to (if any)
    pub async fn get_composite_task_id_for_unit_task(
        &self,
        unit_task_id: &str,
    ) -> DatabaseResult<Option<String>> {
        let result: Option<(String,)> = sqlx::query_as(
            "SELECT composite_task_id FROM composite_task_nodes WHERE unit_task_id = ? LIMIT 1",
        )
        .bind(unit_task_id)
        .fetch_optional(self.db.pool())
        .await?;

        Ok(result.map(|(id,)| id))
    }

    /// Checks if all unit tasks in a composite task are complete (status =
    /// Done) Returns true if all unit tasks have Done status, false
    /// otherwise
    ///
    /// Uses a single aggregate query to avoid N+1 query performance issues.
    pub async fn are_all_composite_task_nodes_complete(
        &self,
        composite_task_id: &str,
    ) -> DatabaseResult<bool> {
        // Use a single aggregate query to check completion status
        // LEFT JOIN ensures nodes pointing to missing unit_tasks are treated as
        // incomplete
        let (total_nodes, done_nodes): (i64, i64) = sqlx::query_as(
            r#"
            SELECT
                COUNT(*) AS total_nodes,
                COUNT(CASE WHEN unit_tasks.status = ? THEN 1 END) AS done_nodes
            FROM composite_task_nodes
            LEFT JOIN unit_tasks
                ON composite_task_nodes.unit_task_id = unit_tasks.id
            WHERE composite_task_nodes.composite_task_id = ?
            "#,
        )
        .bind(unit_task_status_to_string(UnitTaskStatus::Done))
        .bind(composite_task_id)
        .fetch_one(self.db.pool())
        .await?;

        if total_nodes == 0 {
            // No nodes means nothing to complete
            return Ok(false);
        }

        Ok(done_nodes == total_nodes)
    }

    /// Checks if a unit task is part of a composite task and if all nodes are
    /// complete. If so, updates the composite task status to Done.
    /// Returns the composite task ID if the status was updated, None otherwise.
    ///
    /// This method includes a check to ensure the composite task is in
    /// `InProgress` status before updating to `Done`. This prevents race
    /// conditions where multiple unit tasks completing simultaneously could
    /// attempt concurrent updates, and ensures we don't incorrectly
    /// transition tasks that are in Planning, PendingApproval, or other states.
    pub async fn check_and_complete_composite_task(
        &self,
        unit_task_id: &str,
    ) -> DatabaseResult<Option<String>> {
        // Check if this unit task is part of a composite task
        let composite_task_id = match self
            .get_composite_task_id_for_unit_task(unit_task_id)
            .await?
        {
            Some(id) => id,
            None => return Ok(None),
        };

        // Get the current composite task status to prevent race conditions
        // Only proceed if the composite task is in InProgress status
        let composite_task = match self.get_composite_task(&composite_task_id).await? {
            Some(task) => task,
            None => return Ok(None),
        };

        if composite_task.status != CompositeTaskStatus::InProgress {
            tracing::debug!(
                "Composite task {} is not in InProgress status (current: {:?}), skipping \
                 auto-completion",
                composite_task_id,
                composite_task.status
            );
            return Ok(None);
        }

        // Check if all nodes are complete
        if !self
            .are_all_composite_task_nodes_complete(&composite_task_id)
            .await?
        {
            return Ok(None);
        }

        // All nodes are complete, update composite task status to Done
        tracing::info!(
            "All nodes complete for composite task {}, updating status to Done",
            composite_task_id
        );

        self.update_composite_task_status(&composite_task_id, CompositeTaskStatus::Done)
            .await?;

        Ok(Some(composite_task_id))
    }

    /// Gets the list of UnitTask IDs that are ready to execute in a composite
    /// task. A node is ready if:
    /// - All its dependencies are complete (Done status)
    /// - Its own status is not InProgress or Done (i.e., hasn't started or
    ///   completed)
    ///
    /// Returns an empty vector if:
    /// - The unit task is not part of a composite task
    /// - The composite task is not in InProgress status
    /// - No dependent nodes are ready
    pub async fn get_ready_dependent_tasks(
        &self,
        unit_task_id: &str,
    ) -> DatabaseResult<Vec<String>> {
        // Check if this unit task is part of a composite task
        let composite_task_id = match self
            .get_composite_task_id_for_unit_task(unit_task_id)
            .await?
        {
            Some(id) => id,
            None => return Ok(Vec::new()),
        };

        // Get the composite task and verify it's in InProgress status
        let composite_task = match self.get_composite_task(&composite_task_id).await? {
            Some(task) => task,
            None => return Ok(Vec::new()),
        };

        if composite_task.status != CompositeTaskStatus::InProgress {
            return Ok(Vec::new());
        }

        // Get all nodes with their unit task statuses
        // Build a map of node_id -> (unit_task_id, status)
        let mut node_statuses: std::collections::HashMap<String, (String, UnitTaskStatus)> =
            std::collections::HashMap::new();

        for node in &composite_task.nodes {
            if let Some(unit_task) = self.get_unit_task(&node.unit_task_id).await? {
                node_statuses.insert(
                    node.id.clone(),
                    (node.unit_task_id.clone(), unit_task.status),
                );
            }
        }

        // Find completed node IDs
        let completed_node_ids: Vec<String> = node_statuses
            .iter()
            .filter(|(_, (_, status))| *status == UnitTaskStatus::Done)
            .map(|(id, _)| id.clone())
            .collect();

        // Find executable nodes (dependencies satisfied, not yet started/completed)
        let executable_nodes = composite_task.executable_nodes(&completed_node_ids);

        // Filter to nodes that are not in progress or done
        let ready_unit_task_ids: Vec<String> = executable_nodes
            .into_iter()
            .filter_map(|node| {
                node_statuses
                    .get(&node.id)
                    .and_then(|(unit_task_id, status)| {
                        if *status != UnitTaskStatus::InProgress && *status != UnitTaskStatus::Done
                        {
                            Some(unit_task_id.clone())
                        } else {
                            None
                        }
                    })
            })
            .collect();

        tracing::debug!(
            "Found {} ready dependent tasks for composite task {}",
            ready_unit_task_ids.len(),
            composite_task_id
        );

        Ok(ready_unit_task_ids)
    }

    /// Deletes a unit task by ID
    /// Returns an error if the task is referenced by a composite task
    pub async fn delete_unit_task(&self, id: &str) -> DatabaseResult<()> {
        // Note: unit_task_auto_fixes has ON DELETE CASCADE, so it will be cleaned up
        // automatically
        sqlx::query("DELETE FROM unit_tasks WHERE id = ?")
            .bind(id)
            .execute(self.db.pool())
            .await?;

        Ok(())
    }

    /// Gets auto-fix task IDs for a unit task
    async fn get_auto_fix_task_ids(&self, unit_task_id: &str) -> DatabaseResult<Vec<String>> {
        let ids: Vec<(String,)> =
            sqlx::query_as("SELECT agent_task_id FROM unit_task_auto_fixes WHERE unit_task_id = ?")
                .bind(unit_task_id)
                .fetch_all(self.db.pool())
                .await?;

        Ok(ids.into_iter().map(|(id,)| id).collect())
    }

    /// Adds an auto-fix task to a unit task
    pub async fn add_auto_fix_task(
        &self,
        unit_task_id: &str,
        agent_task_id: &str,
    ) -> DatabaseResult<()> {
        sqlx::query("INSERT INTO unit_task_auto_fixes (unit_task_id, agent_task_id) VALUES (?, ?)")
            .bind(unit_task_id)
            .bind(agent_task_id)
            .execute(self.db.pool())
            .await?;

        Ok(())
    }

    // ========== CompositeTask Operations ==========

    /// Lists all composite tasks
    pub async fn list_composite_tasks(
        &self,
        repository_group_id: Option<&str>,
    ) -> DatabaseResult<Vec<CompositeTask>> {
        let rows: Vec<CompositeTaskRow> = if let Some(group_id) = repository_group_id {
            sqlx::query_as(
                "SELECT id, title, prompt, planning_task_id, status, repository_group_id, \
                 plan_file_path, plan_yaml_content, execution_agent_type, created_at, updated_at
                 FROM composite_tasks
                 WHERE repository_group_id = ?
                 ORDER BY created_at DESC",
            )
            .bind(group_id)
            .fetch_all(self.db.pool())
            .await?
        } else {
            sqlx::query_as(
                "SELECT id, title, prompt, planning_task_id, status, repository_group_id, \
                 plan_file_path, plan_yaml_content, execution_agent_type, created_at, updated_at
                 FROM composite_tasks
                 ORDER BY created_at DESC",
            )
            .fetch_all(self.db.pool())
            .await?
        };

        let mut tasks = Vec::new();
        for row in rows {
            let nodes = self.get_composite_task_nodes(&row.id).await?;
            tasks.push(row.into_composite_task(nodes));
        }

        Ok(tasks)
    }

    /// Gets a composite task by ID
    pub async fn get_composite_task(&self, id: &str) -> DatabaseResult<Option<CompositeTask>> {
        let row: Option<CompositeTaskRow> = sqlx::query_as(
            "SELECT id, title, prompt, planning_task_id, status, repository_group_id, \
             plan_file_path, plan_yaml_content, execution_agent_type, created_at, updated_at
             FROM composite_tasks
             WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(self.db.pool())
        .await?;

        match row {
            Some(r) => {
                let nodes = self.get_composite_task_nodes(&r.id).await?;
                Ok(Some(r.into_composite_task(nodes)))
            }
            None => Ok(None),
        }
    }

    /// Creates a new composite task
    pub async fn create_composite_task(&self, task: &CompositeTask) -> DatabaseResult<()> {
        let status = composite_task_status_to_string(task.status);
        let execution_agent_type = task.execution_agent_type.map(ai_agent_type_to_string);

        sqlx::query(
            "INSERT INTO composite_tasks (id, title, prompt, planning_task_id, status, \
             repository_group_id, plan_file_path, plan_yaml_content, execution_agent_type, \
             created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&task.id)
        .bind(&task.title)
        .bind(&task.prompt)
        .bind(&task.planning_task_id)
        .bind(status)
        .bind(&task.repository_group_id)
        .bind(&task.plan_file_path)
        .bind(&task.plan_yaml_content)
        .bind(execution_agent_type)
        .bind(task.created_at.to_rfc3339())
        .bind(task.updated_at.to_rfc3339())
        .execute(self.db.pool())
        .await?;

        // Insert nodes
        for node in &task.nodes {
            self.create_composite_task_node(&task.id, node).await?;
        }

        Ok(())
    }

    /// Updates composite task status
    pub async fn update_composite_task_status(
        &self,
        id: &str,
        status: CompositeTaskStatus,
    ) -> DatabaseResult<()> {
        let status_str = composite_task_status_to_string(status);
        let now = chrono::Utc::now().to_rfc3339();

        sqlx::query("UPDATE composite_tasks SET status = ?, updated_at = ? WHERE id = ?")
            .bind(status_str)
            .bind(&now)
            .bind(id)
            .execute(self.db.pool())
            .await?;

        Ok(())
    }

    /// Updates composite task plan file path
    pub async fn update_composite_task_plan_path(
        &self,
        id: &str,
        plan_path: &str,
    ) -> DatabaseResult<()> {
        let now = chrono::Utc::now().to_rfc3339();

        sqlx::query("UPDATE composite_tasks SET plan_file_path = ?, updated_at = ? WHERE id = ?")
            .bind(plan_path)
            .bind(&now)
            .bind(id)
            .execute(self.db.pool())
            .await?;

        Ok(())
    }

    /// Gets nodes for a composite task
    async fn get_composite_task_nodes(
        &self,
        composite_task_id: &str,
    ) -> DatabaseResult<Vec<CompositeTaskNode>> {
        let rows: Vec<CompositeTaskNodeRow> = sqlx::query_as(
            "SELECT id, composite_task_id, unit_task_id FROM composite_task_nodes WHERE \
             composite_task_id = ?",
        )
        .bind(composite_task_id)
        .fetch_all(self.db.pool())
        .await?;

        let mut nodes = Vec::new();
        for row in rows {
            let deps = self
                .get_node_dependencies(composite_task_id, &row.id)
                .await?;
            nodes.push(CompositeTaskNode {
                id: row.id,
                unit_task_id: row.unit_task_id,
                depends_on: deps,
            });
        }

        Ok(nodes)
    }

    /// Gets dependencies for a node
    async fn get_node_dependencies(
        &self,
        composite_task_id: &str,
        node_id: &str,
    ) -> DatabaseResult<Vec<String>> {
        let deps: Vec<(String,)> = sqlx::query_as(
            "SELECT depends_on_id FROM composite_task_node_deps WHERE composite_task_id = ? AND \
             node_id = ?",
        )
        .bind(composite_task_id)
        .bind(node_id)
        .fetch_all(self.db.pool())
        .await?;

        Ok(deps.into_iter().map(|(id,)| id).collect())
    }

    /// Creates a composite task node
    async fn create_composite_task_node(
        &self,
        composite_task_id: &str,
        node: &CompositeTaskNode,
    ) -> DatabaseResult<()> {
        sqlx::query(
            "INSERT INTO composite_task_nodes (id, composite_task_id, unit_task_id) VALUES (?, ?, \
             ?)",
        )
        .bind(&node.id)
        .bind(composite_task_id)
        .bind(&node.unit_task_id)
        .execute(self.db.pool())
        .await?;

        // Insert dependencies
        for dep in &node.depends_on {
            sqlx::query(
                "INSERT INTO composite_task_node_deps (composite_task_id, node_id, depends_on_id) \
                 VALUES (?, ?, ?)",
            )
            .bind(composite_task_id)
            .bind(&node.id)
            .bind(dep)
            .execute(self.db.pool())
            .await?;
        }

        Ok(())
    }

    /// Adds a node to an existing composite task (public interface)
    pub async fn add_composite_task_node(
        &self,
        composite_task_id: &str,
        node: &CompositeTaskNode,
    ) -> DatabaseResult<()> {
        self.create_composite_task_node(composite_task_id, node)
            .await
    }

    // ========== AgentTask Operations ==========

    /// Creates an agent task
    pub async fn create_agent_task(&self, task: &AgentTask) -> DatabaseResult<()> {
        let ai_agent_type = task.ai_agent_type.map(ai_agent_type_to_string);
        let now = chrono::Utc::now().to_rfc3339();

        sqlx::query(
            "INSERT INTO agent_tasks (id, ai_agent_type, ai_agent_model, created_at)
             VALUES (?, ?, ?, ?)",
        )
        .bind(&task.id)
        .bind(ai_agent_type)
        .bind(&task.ai_agent_model)
        .bind(&now)
        .execute(self.db.pool())
        .await?;

        // Insert base remotes
        for remote in &task.base_remotes {
            sqlx::query(
                "INSERT INTO agent_task_remotes (agent_task_id, git_remote_dir_path, \
                 git_branch_name) VALUES (?, ?, ?)",
            )
            .bind(&task.id)
            .bind(&remote.git_remote_dir_path)
            .bind(&remote.git_branch_name)
            .execute(self.db.pool())
            .await?;
        }

        // Insert sessions
        for session in &task.agent_sessions {
            self.create_agent_session(&task.id, session).await?;
        }

        Ok(())
    }

    /// Gets an agent task by ID
    pub async fn get_agent_task(&self, id: &str) -> DatabaseResult<Option<AgentTask>> {
        let row: Option<AgentTaskRow> = sqlx::query_as(
            "SELECT id, ai_agent_type, ai_agent_model, created_at FROM agent_tasks WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(self.db.pool())
        .await?;

        match row {
            Some(r) => {
                // Get base remotes
                let remote_rows: Vec<BaseRemoteRow> = sqlx::query_as(
                    "SELECT id, agent_task_id, git_remote_dir_path, git_branch_name FROM \
                     agent_task_remotes WHERE agent_task_id = ?",
                )
                .bind(id)
                .fetch_all(self.db.pool())
                .await?;

                // Get sessions
                let session_rows: Vec<AgentSessionRow> = sqlx::query_as(
                    "SELECT id, agent_task_id, ai_agent_type, ai_agent_model, created_at FROM \
                     agent_sessions WHERE agent_task_id = ?",
                )
                .bind(id)
                .fetch_all(self.db.pool())
                .await?;

                // Parse ai_agent_type
                let ai_agent_type = r.ai_agent_type.as_deref().map(|s| match s {
                    "claude_code" => crate::entities::AIAgentType::ClaudeCode,
                    "open_code" => crate::entities::AIAgentType::OpenCode,
                    _ => crate::entities::AIAgentType::ClaudeCode,
                });

                Ok(Some(AgentTask {
                    id: r.id,
                    base_remotes: remote_rows.into_iter().map(|r| r.into()).collect(),
                    agent_sessions: session_rows.into_iter().map(|r| r.into()).collect(),
                    ai_agent_type,
                    ai_agent_model: r.ai_agent_model,
                }))
            }
            None => Ok(None),
        }
    }

    /// Creates an agent session
    pub async fn create_agent_session(
        &self,
        agent_task_id: &str,
        session: &AgentSession,
    ) -> DatabaseResult<()> {
        let ai_agent_type = ai_agent_type_to_string(session.ai_agent_type);
        let now = chrono::Utc::now().to_rfc3339();

        sqlx::query(
            "INSERT INTO agent_sessions (id, agent_task_id, ai_agent_type, ai_agent_model, \
             created_at)
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(&session.id)
        .bind(agent_task_id)
        .bind(ai_agent_type)
        .bind(&session.ai_agent_model)
        .bind(&now)
        .execute(self.db.pool())
        .await?;

        Ok(())
    }

    /// Gets unit tasks by status
    pub async fn get_unit_tasks_by_status(
        &self,
        status: UnitTaskStatus,
    ) -> DatabaseResult<Vec<UnitTask>> {
        let status_str = unit_task_status_to_string(status);

        let rows: Vec<UnitTaskRow> = sqlx::query_as(
            "SELECT id, title, prompt, agent_task_id, branch_name, linked_pr_url, base_commit, \
             end_commit, status, repository_group_id, created_at, updated_at, \
             last_execution_failed
             FROM unit_tasks
             WHERE status = ?
             ORDER BY created_at DESC",
        )
        .bind(status_str)
        .fetch_all(self.db.pool())
        .await?;

        let mut tasks = Vec::new();
        for row in rows {
            let auto_fix_ids = self.get_auto_fix_task_ids(&row.id).await?;
            let composite_task_id = self.get_composite_task_id_for_unit_task(&row.id).await?;
            tasks.push(row.into_unit_task(auto_fix_ids, composite_task_id));
        }

        Ok(tasks)
    }

    /// Gets composite tasks by status
    pub async fn get_composite_tasks_by_status(
        &self,
        status: CompositeTaskStatus,
    ) -> DatabaseResult<Vec<CompositeTask>> {
        let status_str = composite_task_status_to_string(status);

        let rows: Vec<CompositeTaskRow> = sqlx::query_as(
            "SELECT id, title, prompt, planning_task_id, status, repository_group_id, \
             plan_file_path, plan_yaml_content, execution_agent_type, created_at, updated_at
             FROM composite_tasks
             WHERE status = ?
             ORDER BY created_at DESC",
        )
        .bind(status_str)
        .fetch_all(self.db.pool())
        .await?;

        let mut tasks = Vec::new();
        for row in rows {
            let nodes = self.get_composite_task_nodes(&row.id).await?;
            tasks.push(row.into_composite_task(nodes));
        }

        Ok(tasks)
    }

    /// Updates composite task plan YAML content
    pub async fn update_composite_task_plan_content(
        &self,
        id: &str,
        plan_content: &str,
    ) -> DatabaseResult<()> {
        let now = chrono::Utc::now().to_rfc3339();

        sqlx::query(
            "UPDATE composite_tasks SET plan_yaml_content = ?, updated_at = ? WHERE id = ?",
        )
        .bind(plan_content)
        .bind(&now)
        .bind(id)
        .execute(self.db.pool())
        .await?;

        Ok(())
    }

    /// Gets all unit task IDs belonging to a composite task
    pub async fn get_composite_task_unit_task_ids(
        &self,
        composite_task_id: &str,
    ) -> DatabaseResult<Vec<String>> {
        let rows: Vec<(String,)> = sqlx::query_as(
            "SELECT unit_task_id FROM composite_task_nodes WHERE composite_task_id = ?",
        )
        .bind(composite_task_id)
        .fetch_all(self.db.pool())
        .await?;

        Ok(rows.into_iter().map(|(id,)| id).collect())
    }

    /// Deletes a composite task by ID
    /// This only deletes the composite task and its nodes from the database.
    /// Unit tasks must be deleted separately by the caller to ensure proper
    /// resource cleanup. Note: composite_task_nodes and
    /// composite_task_node_deps are deleted via ON DELETE CASCADE.
    pub async fn delete_composite_task(&self, id: &str) -> DatabaseResult<()> {
        sqlx::query("DELETE FROM composite_tasks WHERE id = ?")
            .bind(id)
            .execute(self.db.pool())
            .await?;

        Ok(())
    }

    // ========== Agent Stream Message Operations ==========

    /// Saves an agent stream message to the database
    pub async fn save_stream_message(
        &self,
        session_id: &str,
        message: &crate::entities::AgentStreamMessage,
    ) -> DatabaseResult<()> {
        let id = uuid::Uuid::new_v4().to_string();
        let timestamp = chrono::Utc::now().to_rfc3339();
        let message_json = serde_json::to_string(message)
            .map_err(|e| crate::database::DatabaseError::Migration(e.to_string()))?;

        sqlx::query(
            "INSERT INTO agent_stream_messages (id, session_id, timestamp, message_json)
             VALUES (?, ?, ?, ?)",
        )
        .bind(&id)
        .bind(session_id)
        .bind(&timestamp)
        .bind(&message_json)
        .execute(self.db.pool())
        .await?;

        Ok(())
    }

    /// Gets all agent stream messages for a session
    pub async fn get_stream_messages(
        &self,
        session_id: &str,
    ) -> DatabaseResult<Vec<crate::entities::AgentStreamMessageEntry>> {
        let rows: Vec<(String, String, String, String)> = sqlx::query_as(
            "SELECT id, session_id, timestamp, message_json
             FROM agent_stream_messages
             WHERE session_id = ?
             ORDER BY timestamp ASC",
        )
        .bind(session_id)
        .fetch_all(self.db.pool())
        .await?;

        let mut messages = Vec::new();
        for (id, session_id, timestamp_str, message_json) in rows {
            let timestamp = chrono::DateTime::parse_from_rfc3339(&timestamp_str)
                .map_err(|e| crate::database::DatabaseError::Migration(e.to_string()))?
                .with_timezone(&chrono::Utc);

            let message: crate::entities::AgentStreamMessage = serde_json::from_str(&message_json)
                .map_err(|e| crate::database::DatabaseError::Migration(e.to_string()))?;

            messages.push(crate::entities::AgentStreamMessageEntry {
                id,
                session_id,
                timestamp,
                message,
            });
        }

        Ok(messages)
    }

    /// Saves an execution log to the database
    pub async fn save_execution_log(
        &self,
        log: &crate::entities::ExecutionLog,
    ) -> DatabaseResult<()> {
        let timestamp = log.timestamp.to_rfc3339();
        let level = match log.level {
            crate::entities::LogLevel::Debug => "debug",
            crate::entities::LogLevel::Info => "info",
            crate::entities::LogLevel::Warn => "warn",
            crate::entities::LogLevel::Error => "error",
        };

        sqlx::query(
            "INSERT INTO execution_logs (id, session_id, timestamp, level, message)
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(&log.id)
        .bind(&log.session_id)
        .bind(&timestamp)
        .bind(level)
        .bind(&log.message)
        .execute(self.db.pool())
        .await?;

        Ok(())
    }

    /// Gets all execution logs for a session
    pub async fn get_execution_logs(
        &self,
        session_id: &str,
    ) -> DatabaseResult<Vec<crate::entities::ExecutionLog>> {
        let rows: Vec<(String, String, String, String, String)> = sqlx::query_as(
            "SELECT id, session_id, timestamp, level, message
             FROM execution_logs
             WHERE session_id = ?
             ORDER BY timestamp ASC",
        )
        .bind(session_id)
        .fetch_all(self.db.pool())
        .await?;

        let mut logs = Vec::new();
        for (id, session_id, timestamp_str, level_str, message) in rows {
            let timestamp = chrono::DateTime::parse_from_rfc3339(&timestamp_str)
                .map_err(|e| crate::database::DatabaseError::Migration(e.to_string()))?
                .with_timezone(&chrono::Utc);

            let level = match level_str.as_str() {
                "debug" => crate::entities::LogLevel::Debug,
                "info" => crate::entities::LogLevel::Info,
                "warn" => crate::entities::LogLevel::Warn,
                "error" => crate::entities::LogLevel::Error,
                _ => crate::entities::LogLevel::Info,
            };

            logs.push(crate::entities::ExecutionLog {
                id,
                session_id,
                timestamp,
                level,
                message,
            });
        }

        Ok(logs)
    }

    // ========== Session Usage Operations ==========

    /// Extracts token usage from stream messages and saves session usage
    /// This should be called after a session completes to record the final
    /// token usage
    pub async fn extract_and_save_session_usage(
        &self,
        session_id: &str,
        model: Option<String>,
    ) -> DatabaseResult<Option<crate::entities::SessionUsage>> {
        // Get all stream messages for this session
        let messages = self.get_stream_messages(session_id).await?;

        // Find the Result message which contains the token usage
        for entry in messages.iter().rev() {
            // Check for Claude Code Result message
            if let crate::entities::AgentStreamMessage::ClaudeCode(
                crate::entities::ClaudeStreamMessage::Result {
                    cost_usd,
                    total_cost_usd,
                    total_input_tokens,
                    total_output_tokens,
                    usage,
                    ..
                },
            ) = &entry.message
            {
                // Get token counts from either the usage object (preferred) or legacy fields
                let input_tokens = usage
                    .as_ref()
                    .and_then(|u| u.input_tokens)
                    .or(*total_input_tokens)
                    .unwrap_or(0);
                let output_tokens = usage
                    .as_ref()
                    .and_then(|u| u.output_tokens)
                    .or(*total_output_tokens)
                    .unwrap_or(0);

                // Get cost from total_cost_usd (preferred) or cost_usd (legacy)
                let cost = total_cost_usd.or(*cost_usd);

                // Only save if we have any token data or cost
                if input_tokens > 0 || output_tokens > 0 || cost.is_some() {
                    let session_usage = crate::entities::SessionUsage::new(
                        session_id.to_string(),
                        input_tokens,
                        output_tokens,
                        cost,
                        model,
                    );

                    self.save_session_usage(&session_usage).await?;
                    tracing::info!(
                        "Saved session usage for {}: {} input tokens, {} output tokens, cost: {:?}",
                        session_id,
                        input_tokens,
                        output_tokens,
                        cost
                    );
                    return Ok(Some(session_usage));
                }
            }
        }

        tracing::debug!(
            "No token usage found in stream messages for session {}",
            session_id
        );
        Ok(None)
    }

    /// Saves session usage data to the database
    pub async fn save_session_usage(
        &self,
        usage: &crate::entities::SessionUsage,
    ) -> DatabaseResult<()> {
        let created_at = usage.created_at.to_rfc3339();

        sqlx::query(
            "INSERT INTO session_usage (id, session_id, input_tokens, output_tokens, \
             total_tokens, cost_usd, model, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&usage.id)
        .bind(&usage.session_id)
        .bind(usage.input_tokens as i64)
        .bind(usage.output_tokens as i64)
        .bind(usage.total_tokens as i64)
        .bind(usage.cost_usd)
        .bind(&usage.model)
        .bind(&created_at)
        .execute(self.db.pool())
        .await?;

        Ok(())
    }

    /// Gets session usage for a specific session
    #[allow(clippy::type_complexity)]
    pub async fn get_session_usage(
        &self,
        session_id: &str,
    ) -> DatabaseResult<Option<crate::entities::SessionUsage>> {
        let row: Option<(
            String,
            String,
            i64,
            i64,
            i64,
            Option<f64>,
            Option<String>,
            String,
        )> = sqlx::query_as(
            "SELECT id, session_id, input_tokens, output_tokens, total_tokens, cost_usd, model, \
             created_at
                 FROM session_usage
                 WHERE session_id = ?",
        )
        .bind(session_id)
        .fetch_optional(self.db.pool())
        .await?;

        match row {
            Some((
                id,
                session_id,
                input_tokens,
                output_tokens,
                total_tokens,
                cost_usd,
                model,
                created_at_str,
            )) => {
                let created_at = chrono::DateTime::parse_from_rfc3339(&created_at_str)
                    .map_err(|e| crate::database::DatabaseError::Migration(e.to_string()))?
                    .with_timezone(&chrono::Utc);

                Ok(Some(crate::entities::SessionUsage {
                    id,
                    session_id,
                    input_tokens: input_tokens as u64,
                    output_tokens: output_tokens as u64,
                    total_tokens: total_tokens as u64,
                    cost_usd,
                    model,
                    created_at,
                }))
            }
            None => Ok(None),
        }
    }

    /// Gets aggregated usage summary for an agent task (all sessions combined)
    pub async fn get_agent_task_usage_summary(
        &self,
        agent_task_id: &str,
    ) -> DatabaseResult<crate::entities::TaskUsageSummary> {
        // Use a single query with JOIN to get aggregated usage for all sessions of this
        // agent task
        let result: (i64, i64, i64, Option<f64>, i64) = sqlx::query_as(
            "SELECT
                COALESCE(SUM(su.input_tokens), 0) as total_input,
                COALESCE(SUM(su.output_tokens), 0) as total_output,
                COALESCE(SUM(su.total_tokens), 0) as total,
                SUM(su.cost_usd) as total_cost,
                COUNT(su.id) as session_count
             FROM agent_sessions s
             LEFT JOIN session_usage su ON s.id = su.session_id
             WHERE s.agent_task_id = ?",
        )
        .bind(agent_task_id)
        .fetch_one(self.db.pool())
        .await
        .unwrap_or((0, 0, 0, None, 0));

        Ok(crate::entities::TaskUsageSummary {
            total_input_tokens: result.0 as u64,
            total_output_tokens: result.1 as u64,
            total_tokens: result.2 as u64,
            total_cost_usd: result.3.unwrap_or(0.0),
            session_count: result.4 as u64,
        })
    }

    /// Gets usage summary for a unit task (includes auto-fix tasks)
    pub async fn get_unit_task_usage_summary(
        &self,
        unit_task_id: &str,
    ) -> DatabaseResult<crate::entities::TaskUsageSummary> {
        // Get the unit task to find the agent_task_id
        let unit_task = self.get_unit_task(unit_task_id).await?;
        let unit_task = unit_task.ok_or_else(|| {
            crate::database::DatabaseError::NotFound(format!(
                "Unit task {} not found",
                unit_task_id
            ))
        })?;

        // Get usage for the main agent task
        let mut summary = self
            .get_agent_task_usage_summary(&unit_task.agent_task_id)
            .await?;

        // Add usage from auto-fix tasks
        for auto_fix_id in &unit_task.auto_fix_task_ids {
            let auto_fix_summary = self.get_agent_task_usage_summary(auto_fix_id).await?;
            summary.total_input_tokens += auto_fix_summary.total_input_tokens;
            summary.total_output_tokens += auto_fix_summary.total_output_tokens;
            summary.total_tokens += auto_fix_summary.total_tokens;
            summary.total_cost_usd += auto_fix_summary.total_cost_usd;
            summary.session_count += auto_fix_summary.session_count;
        }

        Ok(summary)
    }

    /// Gets usage summary for a composite task (all unit tasks combined)
    pub async fn get_composite_task_usage_summary(
        &self,
        composite_task_id: &str,
    ) -> DatabaseResult<crate::entities::TaskUsageSummary> {
        // Get the composite task to find the planning task and all unit tasks
        let composite_task = self.get_composite_task(composite_task_id).await?;
        let composite_task = composite_task.ok_or_else(|| {
            crate::database::DatabaseError::NotFound(format!(
                "Composite task {} not found",
                composite_task_id
            ))
        })?;

        // Start with the planning task usage
        let mut summary = self
            .get_agent_task_usage_summary(&composite_task.planning_task_id)
            .await?;

        // Add usage from all unit tasks in the composite task
        for node in &composite_task.nodes {
            let unit_task_summary = self.get_unit_task_usage_summary(&node.unit_task_id).await?;
            summary.total_input_tokens += unit_task_summary.total_input_tokens;
            summary.total_output_tokens += unit_task_summary.total_output_tokens;
            summary.total_tokens += unit_task_summary.total_tokens;
            summary.total_cost_usd += unit_task_summary.total_cost_usd;
            summary.session_count += unit_task_summary.session_count;
        }

        Ok(summary)
    }

    /// Gets the set of blocked unit task IDs.
    /// A unit task is blocked if:
    /// - It belongs to a composite task that is in InProgress status
    /// - The node has dependencies that are not yet completed (Done status)
    pub async fn get_blocked_unit_task_ids(&self) -> DatabaseResult<HashSet<String>> {
        let mut blocked_ids = HashSet::new();

        // Get all composite tasks that are InProgress
        let composite_tasks = self
            .get_composite_tasks_by_status(CompositeTaskStatus::InProgress)
            .await?;

        for composite_task in composite_tasks {
            // Build a map of node_id -> (unit_task_id, status)
            let mut node_statuses: HashMap<String, (String, UnitTaskStatus)> = HashMap::new();

            for node in &composite_task.nodes {
                if let Some(unit_task) = self.get_unit_task(&node.unit_task_id).await? {
                    node_statuses.insert(
                        node.id.clone(),
                        (node.unit_task_id.clone(), unit_task.status),
                    );
                } else {
                    tracing::warn!(
                        composite_task_id = %composite_task.id,
                        node_id = %node.id,
                        unit_task_id = %node.unit_task_id,
                        "Unit task not found for composite task node - possible data integrity issue"
                    );
                }
            }

            // Find completed node IDs using HashSet for O(1) lookups
            let completed_node_ids: HashSet<String> = node_statuses
                .iter()
                .filter(|(_, (_, status))| *status == UnitTaskStatus::Done)
                .map(|(id, _)| id.clone())
                .collect();

            // A node is blocked if it has dependencies that are not all in
            // completed_node_ids
            for node in &composite_task.nodes {
                if !node.depends_on.is_empty() {
                    let all_deps_complete = node
                        .depends_on
                        .iter()
                        .all(|dep| completed_node_ids.contains(dep));

                    if !all_deps_complete {
                        // This node is blocked - add its unit_task_id to blocked set
                        blocked_ids.insert(node.unit_task_id.clone());
                    }
                }
            }
        }

        Ok(blocked_ids)
    }
}
