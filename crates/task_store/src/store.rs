//! Task store trait and implementations

use async_trait::async_trait;

use crate::{
    CompositeTask, CompositeTaskNode, CompositeTaskStatus, Repository, RepositoryGroup,
    StoreResult, TaskFilter, UnitTask, UnitTaskStatus, Workspace,
};

/// Trait for task storage operations
#[async_trait]
pub trait TaskStore: Send + Sync {
    // ========== UnitTask Operations ==========

    /// Creates a new unit task
    async fn create_unit_task(&self, task: &UnitTask) -> StoreResult<()>;

    /// Gets a unit task by ID
    async fn get_unit_task(&self, id: &str) -> StoreResult<Option<UnitTask>>;

    /// Lists unit tasks with optional filter
    async fn list_unit_tasks(&self, filter: &TaskFilter) -> StoreResult<Vec<UnitTask>>;

    /// Updates a unit task's status
    async fn update_unit_task_status(&self, id: &str, status: UnitTaskStatus) -> StoreResult<()>;

    /// Updates a unit task's PR URL
    async fn update_unit_task_pr_url(&self, id: &str, pr_url: &str) -> StoreResult<()>;

    /// Updates a unit task's branch name
    async fn update_unit_task_branch_name(&self, id: &str, branch_name: &str) -> StoreResult<()>;

    /// Updates a unit task's base commit
    async fn update_unit_task_base_commit(&self, id: &str, commit: &str) -> StoreResult<()>;

    /// Updates a unit task's end commit
    async fn update_unit_task_end_commit(&self, id: &str, commit: &str) -> StoreResult<()>;

    /// Updates a unit task's prompt
    async fn update_unit_task_prompt(&self, id: &str, prompt: &str) -> StoreResult<()>;

    /// Deletes a unit task
    async fn delete_unit_task(&self, id: &str) -> StoreResult<()>;

    // ========== CompositeTask Operations ==========

    /// Creates a new composite task
    async fn create_composite_task(&self, task: &CompositeTask) -> StoreResult<()>;

    /// Gets a composite task by ID
    async fn get_composite_task(&self, id: &str) -> StoreResult<Option<CompositeTask>>;

    /// Lists composite tasks with optional repository group filter
    async fn list_composite_tasks(
        &self,
        repository_group_id: Option<&str>,
    ) -> StoreResult<Vec<CompositeTask>>;

    /// Updates a composite task's status
    async fn update_composite_task_status(
        &self,
        id: &str,
        status: CompositeTaskStatus,
    ) -> StoreResult<()>;

    /// Updates a composite task's plan file path
    async fn update_composite_task_plan_path(&self, id: &str, path: &str) -> StoreResult<()>;

    /// Updates a composite task's plan content
    async fn update_composite_task_plan_content(&self, id: &str, content: &str) -> StoreResult<()>;

    /// Adds a node to a composite task
    async fn add_composite_task_node(
        &self,
        composite_task_id: &str,
        node: &CompositeTaskNode,
    ) -> StoreResult<()>;

    /// Deletes a composite task
    async fn delete_composite_task(&self, id: &str) -> StoreResult<()>;

    // ========== Repository Operations ==========

    /// Creates a new repository
    async fn create_repository(&self, repo: &Repository) -> StoreResult<()>;

    /// Gets a repository by ID
    async fn get_repository(&self, id: &str) -> StoreResult<Option<Repository>>;

    /// Lists all repositories
    async fn list_repositories(&self) -> StoreResult<Vec<Repository>>;

    /// Deletes a repository
    async fn delete_repository(&self, id: &str) -> StoreResult<()>;

    // ========== Workspace Operations ==========

    /// Creates a new workspace
    async fn create_workspace(&self, workspace: &Workspace) -> StoreResult<()>;

    /// Gets a workspace by ID
    async fn get_workspace(&self, id: &str) -> StoreResult<Option<Workspace>>;

    /// Lists all workspaces
    async fn list_workspaces(&self) -> StoreResult<Vec<Workspace>>;

    /// Deletes a workspace
    async fn delete_workspace(&self, id: &str) -> StoreResult<()>;

    // ========== RepositoryGroup Operations ==========

    /// Creates a new repository group
    async fn create_repository_group(&self, group: &RepositoryGroup) -> StoreResult<()>;

    /// Gets a repository group by ID
    async fn get_repository_group(&self, id: &str) -> StoreResult<Option<RepositoryGroup>>;

    /// Lists repository groups for a workspace
    async fn list_repository_groups(&self, workspace_id: &str)
        -> StoreResult<Vec<RepositoryGroup>>;

    /// Adds a repository to a group
    async fn add_repository_to_group(&self, group_id: &str, repository_id: &str)
        -> StoreResult<()>;

    /// Removes a repository from a group
    async fn remove_repository_from_group(
        &self,
        group_id: &str,
        repository_id: &str,
    ) -> StoreResult<()>;

    /// Deletes a repository group
    async fn delete_repository_group(&self, id: &str) -> StoreResult<()>;
}

/// In-memory implementation for testing
#[derive(Debug, Default)]
pub struct MemoryStore {
    unit_tasks: std::sync::RwLock<std::collections::HashMap<String, UnitTask>>,
    composite_tasks: std::sync::RwLock<std::collections::HashMap<String, CompositeTask>>,
    repositories: std::sync::RwLock<std::collections::HashMap<String, Repository>>,
    workspaces: std::sync::RwLock<std::collections::HashMap<String, Workspace>>,
    repository_groups: std::sync::RwLock<std::collections::HashMap<String, RepositoryGroup>>,
}

impl MemoryStore {
    /// Creates a new in-memory store
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl TaskStore for MemoryStore {
    async fn create_unit_task(&self, task: &UnitTask) -> StoreResult<()> {
        let mut tasks = self.unit_tasks.write().unwrap();
        tasks.insert(task.id.clone(), task.clone());
        Ok(())
    }

    async fn get_unit_task(&self, id: &str) -> StoreResult<Option<UnitTask>> {
        let tasks = self.unit_tasks.read().unwrap();
        Ok(tasks.get(id).cloned())
    }

    async fn list_unit_tasks(&self, filter: &TaskFilter) -> StoreResult<Vec<UnitTask>> {
        let tasks = self.unit_tasks.read().unwrap();
        let mut result: Vec<UnitTask> = tasks
            .values()
            .filter(|t| {
                filter
                    .repository_group_id
                    .as_ref()
                    .is_none_or(|id| &t.repository_group_id == id)
                    && filter.status.is_none_or(|s| t.status == s)
            })
            .cloned()
            .collect();
        result.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(result)
    }

    async fn update_unit_task_status(&self, id: &str, status: UnitTaskStatus) -> StoreResult<()> {
        let mut tasks = self.unit_tasks.write().unwrap();
        if let Some(task) = tasks.get_mut(id) {
            task.status = status;
            task.updated_at = chrono::Utc::now();
        }
        Ok(())
    }

    async fn update_unit_task_pr_url(&self, id: &str, pr_url: &str) -> StoreResult<()> {
        let mut tasks = self.unit_tasks.write().unwrap();
        if let Some(task) = tasks.get_mut(id) {
            task.linked_pr_url = Some(pr_url.to_string());
            task.updated_at = chrono::Utc::now();
        }
        Ok(())
    }

    async fn update_unit_task_branch_name(&self, id: &str, branch_name: &str) -> StoreResult<()> {
        let mut tasks = self.unit_tasks.write().unwrap();
        if let Some(task) = tasks.get_mut(id) {
            task.branch_name = Some(branch_name.to_string());
            task.updated_at = chrono::Utc::now();
        }
        Ok(())
    }

    async fn update_unit_task_base_commit(&self, id: &str, commit: &str) -> StoreResult<()> {
        let mut tasks = self.unit_tasks.write().unwrap();
        if let Some(task) = tasks.get_mut(id) {
            task.base_commit = Some(commit.to_string());
            task.updated_at = chrono::Utc::now();
        }
        Ok(())
    }

    async fn update_unit_task_end_commit(&self, id: &str, commit: &str) -> StoreResult<()> {
        let mut tasks = self.unit_tasks.write().unwrap();
        if let Some(task) = tasks.get_mut(id) {
            task.end_commit = Some(commit.to_string());
            task.updated_at = chrono::Utc::now();
        }
        Ok(())
    }

    async fn update_unit_task_prompt(&self, id: &str, prompt: &str) -> StoreResult<()> {
        let mut tasks = self.unit_tasks.write().unwrap();
        if let Some(task) = tasks.get_mut(id) {
            task.prompt = prompt.to_string();
            task.updated_at = chrono::Utc::now();
        }
        Ok(())
    }

    async fn delete_unit_task(&self, id: &str) -> StoreResult<()> {
        let mut tasks = self.unit_tasks.write().unwrap();
        tasks.remove(id);
        Ok(())
    }

    async fn create_composite_task(&self, task: &CompositeTask) -> StoreResult<()> {
        let mut tasks = self.composite_tasks.write().unwrap();
        tasks.insert(task.id.clone(), task.clone());
        Ok(())
    }

    async fn get_composite_task(&self, id: &str) -> StoreResult<Option<CompositeTask>> {
        let tasks = self.composite_tasks.read().unwrap();
        Ok(tasks.get(id).cloned())
    }

    async fn list_composite_tasks(
        &self,
        repository_group_id: Option<&str>,
    ) -> StoreResult<Vec<CompositeTask>> {
        let tasks = self.composite_tasks.read().unwrap();
        let mut result: Vec<CompositeTask> = tasks
            .values()
            .filter(|t| repository_group_id.is_none_or(|id| t.repository_group_id == id))
            .cloned()
            .collect();
        result.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(result)
    }

    async fn update_composite_task_status(
        &self,
        id: &str,
        status: CompositeTaskStatus,
    ) -> StoreResult<()> {
        let mut tasks = self.composite_tasks.write().unwrap();
        if let Some(task) = tasks.get_mut(id) {
            task.status = status;
            task.updated_at = chrono::Utc::now();
        }
        Ok(())
    }

    async fn update_composite_task_plan_path(&self, id: &str, path: &str) -> StoreResult<()> {
        let mut tasks = self.composite_tasks.write().unwrap();
        if let Some(task) = tasks.get_mut(id) {
            task.plan_file_path = Some(path.to_string());
            task.updated_at = chrono::Utc::now();
        }
        Ok(())
    }

    async fn update_composite_task_plan_content(&self, id: &str, content: &str) -> StoreResult<()> {
        let mut tasks = self.composite_tasks.write().unwrap();
        if let Some(task) = tasks.get_mut(id) {
            task.plan_yaml_content = Some(content.to_string());
            task.updated_at = chrono::Utc::now();
        }
        Ok(())
    }

    async fn add_composite_task_node(
        &self,
        composite_task_id: &str,
        node: &CompositeTaskNode,
    ) -> StoreResult<()> {
        let mut tasks = self.composite_tasks.write().unwrap();
        if let Some(task) = tasks.get_mut(composite_task_id) {
            task.nodes.push(node.clone());
            task.updated_at = chrono::Utc::now();
        }
        Ok(())
    }

    async fn delete_composite_task(&self, id: &str) -> StoreResult<()> {
        let mut tasks = self.composite_tasks.write().unwrap();
        tasks.remove(id);
        Ok(())
    }

    async fn create_repository(&self, repo: &Repository) -> StoreResult<()> {
        let mut repos = self.repositories.write().unwrap();
        repos.insert(repo.id.clone(), repo.clone());
        Ok(())
    }

    async fn get_repository(&self, id: &str) -> StoreResult<Option<Repository>> {
        let repos = self.repositories.read().unwrap();
        Ok(repos.get(id).cloned())
    }

    async fn list_repositories(&self) -> StoreResult<Vec<Repository>> {
        let repos = self.repositories.read().unwrap();
        Ok(repos.values().cloned().collect())
    }

    async fn delete_repository(&self, id: &str) -> StoreResult<()> {
        let mut repos = self.repositories.write().unwrap();
        repos.remove(id);
        Ok(())
    }

    async fn create_workspace(&self, workspace: &Workspace) -> StoreResult<()> {
        let mut workspaces = self.workspaces.write().unwrap();
        workspaces.insert(workspace.id.clone(), workspace.clone());
        Ok(())
    }

    async fn get_workspace(&self, id: &str) -> StoreResult<Option<Workspace>> {
        let workspaces = self.workspaces.read().unwrap();
        Ok(workspaces.get(id).cloned())
    }

    async fn list_workspaces(&self) -> StoreResult<Vec<Workspace>> {
        let workspaces = self.workspaces.read().unwrap();
        Ok(workspaces.values().cloned().collect())
    }

    async fn delete_workspace(&self, id: &str) -> StoreResult<()> {
        let mut workspaces = self.workspaces.write().unwrap();
        workspaces.remove(id);
        Ok(())
    }

    async fn create_repository_group(&self, group: &RepositoryGroup) -> StoreResult<()> {
        let mut groups = self.repository_groups.write().unwrap();
        groups.insert(group.id.clone(), group.clone());
        Ok(())
    }

    async fn get_repository_group(&self, id: &str) -> StoreResult<Option<RepositoryGroup>> {
        let groups = self.repository_groups.read().unwrap();
        Ok(groups.get(id).cloned())
    }

    async fn list_repository_groups(
        &self,
        workspace_id: &str,
    ) -> StoreResult<Vec<RepositoryGroup>> {
        let groups = self.repository_groups.read().unwrap();
        Ok(groups
            .values()
            .filter(|g| g.workspace_id == workspace_id)
            .cloned()
            .collect())
    }

    async fn add_repository_to_group(
        &self,
        group_id: &str,
        repository_id: &str,
    ) -> StoreResult<()> {
        let mut groups = self.repository_groups.write().unwrap();
        if let Some(group) = groups.get_mut(group_id) {
            if !group.repository_ids.contains(&repository_id.to_string()) {
                group.repository_ids.push(repository_id.to_string());
                group.updated_at = chrono::Utc::now();
            }
        }
        Ok(())
    }

    async fn remove_repository_from_group(
        &self,
        group_id: &str,
        repository_id: &str,
    ) -> StoreResult<()> {
        let mut groups = self.repository_groups.write().unwrap();
        if let Some(group) = groups.get_mut(group_id) {
            group.repository_ids.retain(|id| id != repository_id);
            group.updated_at = chrono::Utc::now();
        }
        Ok(())
    }

    async fn delete_repository_group(&self, id: &str) -> StoreResult<()> {
        let mut groups = self.repository_groups.write().unwrap();
        groups.remove(id);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_memory_store_unit_tasks() {
        let store = MemoryStore::new();

        let task = UnitTask::new("Test Task", "Do something", "agent-1", "repo-group-1");
        store.create_unit_task(&task).await.unwrap();

        let retrieved = store.get_unit_task(&task.id).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().title, "Test Task");

        store
            .update_unit_task_status(&task.id, UnitTaskStatus::InReview)
            .await
            .unwrap();

        let updated = store.get_unit_task(&task.id).await.unwrap().unwrap();
        assert_eq!(updated.status, UnitTaskStatus::InReview);
    }

    #[tokio::test]
    async fn test_memory_store_filter() {
        let store = MemoryStore::new();

        let task1 = UnitTask::new("Task 1", "Prompt 1", "agent-1", "group-1");
        let task2 = UnitTask::new("Task 2", "Prompt 2", "agent-2", "group-2");

        store.create_unit_task(&task1).await.unwrap();
        store.create_unit_task(&task2).await.unwrap();

        let filter = TaskFilter::new().with_repository_group("group-1");
        let filtered = store.list_unit_tasks(&filter).await.unwrap();

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].title, "Task 1");
    }
}
