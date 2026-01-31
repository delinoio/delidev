//! In-memory task store implementation for testing.

use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use entities::{
    AgentSession, AgentTask, CompositeTask, CompositeTaskNode, Repository, RepositoryGroup,
    TodoItem, TtyInputRequest, UnitTask, User, Workspace,
};
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::{
    RepositoryFilter, TaskFilter, TaskStore, TaskStoreError, TaskStoreResult, TodoFilter,
    TtyInputFilter, WorkspaceFilter,
};

/// In-memory task store for testing purposes.
#[derive(Debug, Default)]
pub struct MemoryTaskStore {
    users: Arc<RwLock<HashMap<Uuid, User>>>,
    workspaces: Arc<RwLock<HashMap<Uuid, Workspace>>>,
    repositories: Arc<RwLock<HashMap<Uuid, Repository>>>,
    repository_groups: Arc<RwLock<HashMap<Uuid, RepositoryGroup>>>,
    agent_tasks: Arc<RwLock<HashMap<Uuid, AgentTask>>>,
    agent_sessions: Arc<RwLock<HashMap<Uuid, AgentSession>>>,
    unit_tasks: Arc<RwLock<HashMap<Uuid, UnitTask>>>,
    composite_tasks: Arc<RwLock<HashMap<Uuid, CompositeTask>>>,
    composite_task_nodes: Arc<RwLock<HashMap<Uuid, CompositeTaskNode>>>,
    todo_items: Arc<RwLock<HashMap<Uuid, TodoItem>>>,
    tty_input_requests: Arc<RwLock<HashMap<Uuid, TtyInputRequest>>>,
}

impl MemoryTaskStore {
    /// Creates a new in-memory task store.
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl TaskStore for MemoryTaskStore {
    // =========================================================================
    // User operations
    // =========================================================================

    async fn create_user(&self, user: User) -> TaskStoreResult<User> {
        let mut users = self.users.write().await;
        if users.contains_key(&user.id) {
            return Err(TaskStoreError::already_exists("User", user.id.to_string()));
        }
        users.insert(user.id, user.clone());
        Ok(user)
    }

    async fn get_user(&self, id: Uuid) -> TaskStoreResult<Option<User>> {
        let users = self.users.read().await;
        Ok(users.get(&id).cloned())
    }

    async fn get_user_by_email(&self, email: &str) -> TaskStoreResult<Option<User>> {
        let users = self.users.read().await;
        Ok(users.values().find(|u| u.email == email).cloned())
    }

    async fn update_user(&self, user: User) -> TaskStoreResult<User> {
        let mut users = self.users.write().await;
        if !users.contains_key(&user.id) {
            return Err(TaskStoreError::not_found("User", user.id.to_string()));
        }
        users.insert(user.id, user.clone());
        Ok(user)
    }

    async fn delete_user(&self, id: Uuid) -> TaskStoreResult<()> {
        let mut users = self.users.write().await;
        if users.remove(&id).is_none() {
            return Err(TaskStoreError::not_found("User", id.to_string()));
        }
        Ok(())
    }

    // =========================================================================
    // Workspace operations
    // =========================================================================

    async fn create_workspace(&self, workspace: Workspace) -> TaskStoreResult<Workspace> {
        let mut workspaces = self.workspaces.write().await;
        if workspaces.contains_key(&workspace.id) {
            return Err(TaskStoreError::already_exists(
                "Workspace",
                workspace.id.to_string(),
            ));
        }
        workspaces.insert(workspace.id, workspace.clone());
        Ok(workspace)
    }

    async fn get_workspace(&self, id: Uuid) -> TaskStoreResult<Option<Workspace>> {
        let workspaces = self.workspaces.read().await;
        Ok(workspaces.get(&id).cloned())
    }

    async fn list_workspaces(
        &self,
        filter: WorkspaceFilter,
    ) -> TaskStoreResult<(Vec<Workspace>, u32)> {
        let workspaces = self.workspaces.read().await;
        let mut result: Vec<Workspace> = workspaces
            .values()
            .filter(|w| {
                if let Some(user_id) = filter.user_id {
                    w.user_id == Some(user_id)
                } else {
                    true
                }
            })
            .cloned()
            .collect();

        let total = result.len() as u32;

        if let Some(offset) = filter.offset {
            result = result.into_iter().skip(offset as usize).collect();
        }
        if let Some(limit) = filter.limit {
            result = result.into_iter().take(limit as usize).collect();
        }

        Ok((result, total))
    }

    async fn update_workspace(&self, workspace: Workspace) -> TaskStoreResult<Workspace> {
        let mut workspaces = self.workspaces.write().await;
        if !workspaces.contains_key(&workspace.id) {
            return Err(TaskStoreError::not_found(
                "Workspace",
                workspace.id.to_string(),
            ));
        }
        workspaces.insert(workspace.id, workspace.clone());
        Ok(workspace)
    }

    async fn delete_workspace(&self, id: Uuid) -> TaskStoreResult<()> {
        let mut workspaces = self.workspaces.write().await;
        if workspaces.remove(&id).is_none() {
            return Err(TaskStoreError::not_found("Workspace", id.to_string()));
        }
        Ok(())
    }

    // =========================================================================
    // Repository operations
    // =========================================================================

    async fn create_repository(&self, repository: Repository) -> TaskStoreResult<Repository> {
        let mut repositories = self.repositories.write().await;
        if repositories.contains_key(&repository.id) {
            return Err(TaskStoreError::already_exists(
                "Repository",
                repository.id.to_string(),
            ));
        }
        repositories.insert(repository.id, repository.clone());
        Ok(repository)
    }

    async fn get_repository(&self, id: Uuid) -> TaskStoreResult<Option<Repository>> {
        let repositories = self.repositories.read().await;
        Ok(repositories.get(&id).cloned())
    }

    async fn list_repositories(
        &self,
        filter: RepositoryFilter,
    ) -> TaskStoreResult<(Vec<Repository>, u32)> {
        let repositories = self.repositories.read().await;
        let mut result: Vec<Repository> = repositories
            .values()
            .filter(|r| {
                if let Some(workspace_id) = filter.workspace_id {
                    r.workspace_id == workspace_id
                } else {
                    true
                }
            })
            .cloned()
            .collect();

        let total = result.len() as u32;

        if let Some(offset) = filter.offset {
            result = result.into_iter().skip(offset as usize).collect();
        }
        if let Some(limit) = filter.limit {
            result = result.into_iter().take(limit as usize).collect();
        }

        Ok((result, total))
    }

    async fn update_repository(&self, repository: Repository) -> TaskStoreResult<Repository> {
        let mut repositories = self.repositories.write().await;
        if !repositories.contains_key(&repository.id) {
            return Err(TaskStoreError::not_found(
                "Repository",
                repository.id.to_string(),
            ));
        }
        repositories.insert(repository.id, repository.clone());
        Ok(repository)
    }

    async fn delete_repository(&self, id: Uuid) -> TaskStoreResult<()> {
        let mut repositories = self.repositories.write().await;
        if repositories.remove(&id).is_none() {
            return Err(TaskStoreError::not_found("Repository", id.to_string()));
        }
        Ok(())
    }

    // =========================================================================
    // Repository Group operations
    // =========================================================================

    async fn create_repository_group(
        &self,
        group: RepositoryGroup,
    ) -> TaskStoreResult<RepositoryGroup> {
        let mut groups = self.repository_groups.write().await;
        if groups.contains_key(&group.id) {
            return Err(TaskStoreError::already_exists(
                "RepositoryGroup",
                group.id.to_string(),
            ));
        }
        groups.insert(group.id, group.clone());
        Ok(group)
    }

    async fn get_repository_group(&self, id: Uuid) -> TaskStoreResult<Option<RepositoryGroup>> {
        let groups = self.repository_groups.read().await;
        Ok(groups.get(&id).cloned())
    }

    async fn list_repository_groups(
        &self,
        workspace_id: Option<Uuid>,
    ) -> TaskStoreResult<Vec<RepositoryGroup>> {
        let groups = self.repository_groups.read().await;
        Ok(groups
            .values()
            .filter(|g| {
                if let Some(ws_id) = workspace_id {
                    g.workspace_id == ws_id
                } else {
                    true
                }
            })
            .cloned()
            .collect())
    }

    async fn update_repository_group(
        &self,
        group: RepositoryGroup,
    ) -> TaskStoreResult<RepositoryGroup> {
        let mut groups = self.repository_groups.write().await;
        if !groups.contains_key(&group.id) {
            return Err(TaskStoreError::not_found(
                "RepositoryGroup",
                group.id.to_string(),
            ));
        }
        groups.insert(group.id, group.clone());
        Ok(group)
    }

    async fn delete_repository_group(&self, id: Uuid) -> TaskStoreResult<()> {
        let mut groups = self.repository_groups.write().await;
        if groups.remove(&id).is_none() {
            return Err(TaskStoreError::not_found("RepositoryGroup", id.to_string()));
        }
        Ok(())
    }

    // =========================================================================
    // Agent Task operations
    // =========================================================================

    async fn create_agent_task(&self, task: AgentTask) -> TaskStoreResult<AgentTask> {
        let mut tasks = self.agent_tasks.write().await;
        if tasks.contains_key(&task.id) {
            return Err(TaskStoreError::already_exists(
                "AgentTask",
                task.id.to_string(),
            ));
        }
        tasks.insert(task.id, task.clone());
        Ok(task)
    }

    async fn get_agent_task(&self, id: Uuid) -> TaskStoreResult<Option<AgentTask>> {
        let tasks = self.agent_tasks.read().await;
        Ok(tasks.get(&id).cloned())
    }

    async fn update_agent_task(&self, task: AgentTask) -> TaskStoreResult<AgentTask> {
        let mut tasks = self.agent_tasks.write().await;
        if !tasks.contains_key(&task.id) {
            return Err(TaskStoreError::not_found("AgentTask", task.id.to_string()));
        }
        tasks.insert(task.id, task.clone());
        Ok(task)
    }

    async fn delete_agent_task(&self, id: Uuid) -> TaskStoreResult<()> {
        let mut tasks = self.agent_tasks.write().await;
        if tasks.remove(&id).is_none() {
            return Err(TaskStoreError::not_found("AgentTask", id.to_string()));
        }
        Ok(())
    }

    // =========================================================================
    // Agent Session operations
    // =========================================================================

    async fn create_agent_session(&self, session: AgentSession) -> TaskStoreResult<AgentSession> {
        let mut sessions = self.agent_sessions.write().await;
        if sessions.contains_key(&session.id) {
            return Err(TaskStoreError::already_exists(
                "AgentSession",
                session.id.to_string(),
            ));
        }
        sessions.insert(session.id, session.clone());
        Ok(session)
    }

    async fn get_agent_session(&self, id: Uuid) -> TaskStoreResult<Option<AgentSession>> {
        let sessions = self.agent_sessions.read().await;
        Ok(sessions.get(&id).cloned())
    }

    async fn list_agent_sessions(&self, agent_task_id: Uuid) -> TaskStoreResult<Vec<AgentSession>> {
        let sessions = self.agent_sessions.read().await;
        Ok(sessions
            .values()
            .filter(|s| s.agent_task_id == agent_task_id)
            .cloned()
            .collect())
    }

    async fn update_agent_session(&self, session: AgentSession) -> TaskStoreResult<AgentSession> {
        let mut sessions = self.agent_sessions.write().await;
        if !sessions.contains_key(&session.id) {
            return Err(TaskStoreError::not_found(
                "AgentSession",
                session.id.to_string(),
            ));
        }
        sessions.insert(session.id, session.clone());
        Ok(session)
    }

    async fn delete_agent_session(&self, id: Uuid) -> TaskStoreResult<()> {
        let mut sessions = self.agent_sessions.write().await;
        if sessions.remove(&id).is_none() {
            return Err(TaskStoreError::not_found("AgentSession", id.to_string()));
        }
        Ok(())
    }

    // =========================================================================
    // Unit Task operations
    // =========================================================================

    async fn create_unit_task(&self, task: UnitTask) -> TaskStoreResult<UnitTask> {
        let mut tasks = self.unit_tasks.write().await;
        if tasks.contains_key(&task.id) {
            return Err(TaskStoreError::already_exists(
                "UnitTask",
                task.id.to_string(),
            ));
        }
        tasks.insert(task.id, task.clone());
        Ok(task)
    }

    async fn get_unit_task(&self, id: Uuid) -> TaskStoreResult<Option<UnitTask>> {
        let tasks = self.unit_tasks.read().await;
        Ok(tasks.get(&id).cloned())
    }

    async fn list_unit_tasks(&self, filter: TaskFilter) -> TaskStoreResult<(Vec<UnitTask>, u32)> {
        let tasks = self.unit_tasks.read().await;
        let mut result: Vec<UnitTask> = tasks
            .values()
            .filter(|t| {
                let mut matches = true;
                if let Some(group_id) = filter.repository_group_id {
                    matches = matches && t.repository_group_id == group_id;
                }
                if let Some(status) = filter.unit_status {
                    matches = matches && t.status == status;
                }
                matches
            })
            .cloned()
            .collect();

        let total = result.len() as u32;

        if let Some(offset) = filter.offset {
            result = result.into_iter().skip(offset as usize).collect();
        }
        if let Some(limit) = filter.limit {
            result = result.into_iter().take(limit as usize).collect();
        }

        Ok((result, total))
    }

    async fn update_unit_task(&self, task: UnitTask) -> TaskStoreResult<UnitTask> {
        let mut tasks = self.unit_tasks.write().await;
        if !tasks.contains_key(&task.id) {
            return Err(TaskStoreError::not_found("UnitTask", task.id.to_string()));
        }
        tasks.insert(task.id, task.clone());
        Ok(task)
    }

    async fn delete_unit_task(&self, id: Uuid) -> TaskStoreResult<()> {
        let mut tasks = self.unit_tasks.write().await;
        if tasks.remove(&id).is_none() {
            return Err(TaskStoreError::not_found("UnitTask", id.to_string()));
        }
        Ok(())
    }

    // =========================================================================
    // Composite Task operations
    // =========================================================================

    async fn create_composite_task(&self, task: CompositeTask) -> TaskStoreResult<CompositeTask> {
        let mut tasks = self.composite_tasks.write().await;
        if tasks.contains_key(&task.id) {
            return Err(TaskStoreError::already_exists(
                "CompositeTask",
                task.id.to_string(),
            ));
        }
        tasks.insert(task.id, task.clone());
        Ok(task)
    }

    async fn get_composite_task(&self, id: Uuid) -> TaskStoreResult<Option<CompositeTask>> {
        let tasks = self.composite_tasks.read().await;
        Ok(tasks.get(&id).cloned())
    }

    async fn list_composite_tasks(
        &self,
        filter: TaskFilter,
    ) -> TaskStoreResult<(Vec<CompositeTask>, u32)> {
        let tasks = self.composite_tasks.read().await;
        let mut result: Vec<CompositeTask> = tasks
            .values()
            .filter(|t| {
                let mut matches = true;
                if let Some(group_id) = filter.repository_group_id {
                    matches = matches && t.repository_group_id == group_id;
                }
                if let Some(status) = filter.composite_status {
                    matches = matches && t.status == status;
                }
                matches
            })
            .cloned()
            .collect();

        let total = result.len() as u32;

        if let Some(offset) = filter.offset {
            result = result.into_iter().skip(offset as usize).collect();
        }
        if let Some(limit) = filter.limit {
            result = result.into_iter().take(limit as usize).collect();
        }

        Ok((result, total))
    }

    async fn update_composite_task(&self, task: CompositeTask) -> TaskStoreResult<CompositeTask> {
        let mut tasks = self.composite_tasks.write().await;
        if !tasks.contains_key(&task.id) {
            return Err(TaskStoreError::not_found(
                "CompositeTask",
                task.id.to_string(),
            ));
        }
        tasks.insert(task.id, task.clone());
        Ok(task)
    }

    async fn delete_composite_task(&self, id: Uuid) -> TaskStoreResult<()> {
        let mut tasks = self.composite_tasks.write().await;
        if tasks.remove(&id).is_none() {
            return Err(TaskStoreError::not_found("CompositeTask", id.to_string()));
        }
        Ok(())
    }

    // =========================================================================
    // Composite Task Node operations
    // =========================================================================

    async fn create_composite_task_node(
        &self,
        node: CompositeTaskNode,
    ) -> TaskStoreResult<CompositeTaskNode> {
        let mut nodes = self.composite_task_nodes.write().await;
        if nodes.contains_key(&node.id) {
            return Err(TaskStoreError::already_exists(
                "CompositeTaskNode",
                node.id.to_string(),
            ));
        }
        nodes.insert(node.id, node.clone());
        Ok(node)
    }

    async fn get_composite_task_node(
        &self,
        id: Uuid,
    ) -> TaskStoreResult<Option<CompositeTaskNode>> {
        let nodes = self.composite_task_nodes.read().await;
        Ok(nodes.get(&id).cloned())
    }

    async fn list_composite_task_nodes(
        &self,
        composite_task_id: Uuid,
    ) -> TaskStoreResult<Vec<CompositeTaskNode>> {
        let nodes = self.composite_task_nodes.read().await;
        Ok(nodes
            .values()
            .filter(|n| n.composite_task_id == composite_task_id)
            .cloned()
            .collect())
    }

    async fn update_composite_task_node(
        &self,
        node: CompositeTaskNode,
    ) -> TaskStoreResult<CompositeTaskNode> {
        let mut nodes = self.composite_task_nodes.write().await;
        if !nodes.contains_key(&node.id) {
            return Err(TaskStoreError::not_found(
                "CompositeTaskNode",
                node.id.to_string(),
            ));
        }
        nodes.insert(node.id, node.clone());
        Ok(node)
    }

    async fn delete_composite_task_node(&self, id: Uuid) -> TaskStoreResult<()> {
        let mut nodes = self.composite_task_nodes.write().await;
        if nodes.remove(&id).is_none() {
            return Err(TaskStoreError::not_found(
                "CompositeTaskNode",
                id.to_string(),
            ));
        }
        Ok(())
    }

    // =========================================================================
    // Todo Item operations
    // =========================================================================

    async fn create_todo_item(&self, item: TodoItem) -> TaskStoreResult<TodoItem> {
        let mut items = self.todo_items.write().await;
        if items.contains_key(&item.id) {
            return Err(TaskStoreError::already_exists(
                "TodoItem",
                item.id.to_string(),
            ));
        }
        items.insert(item.id, item.clone());
        Ok(item)
    }

    async fn get_todo_item(&self, id: Uuid) -> TaskStoreResult<Option<TodoItem>> {
        let items = self.todo_items.read().await;
        Ok(items.get(&id).cloned())
    }

    async fn list_todo_items(&self, filter: TodoFilter) -> TaskStoreResult<(Vec<TodoItem>, u32)> {
        let items = self.todo_items.read().await;
        let mut result: Vec<TodoItem> = items
            .values()
            .filter(|i| {
                let mut matches = true;
                if let Some(repo_id) = filter.repository_id {
                    matches = matches && i.repository_id == repo_id;
                }
                if let Some(status) = filter.status {
                    matches = matches && i.status == status;
                }
                matches
            })
            .cloned()
            .collect();

        let total = result.len() as u32;

        if let Some(offset) = filter.offset {
            result = result.into_iter().skip(offset as usize).collect();
        }
        if let Some(limit) = filter.limit {
            result = result.into_iter().take(limit as usize).collect();
        }

        Ok((result, total))
    }

    async fn update_todo_item(&self, item: TodoItem) -> TaskStoreResult<TodoItem> {
        let mut items = self.todo_items.write().await;
        if !items.contains_key(&item.id) {
            return Err(TaskStoreError::not_found("TodoItem", item.id.to_string()));
        }
        items.insert(item.id, item.clone());
        Ok(item)
    }

    async fn delete_todo_item(&self, id: Uuid) -> TaskStoreResult<()> {
        let mut items = self.todo_items.write().await;
        if items.remove(&id).is_none() {
            return Err(TaskStoreError::not_found("TodoItem", id.to_string()));
        }
        Ok(())
    }

    // =========================================================================
    // TTY Input Request operations
    // =========================================================================

    async fn create_tty_input_request(
        &self,
        request: TtyInputRequest,
    ) -> TaskStoreResult<TtyInputRequest> {
        let mut requests = self.tty_input_requests.write().await;
        if requests.contains_key(&request.id) {
            return Err(TaskStoreError::already_exists(
                "TtyInputRequest",
                request.id.to_string(),
            ));
        }
        requests.insert(request.id, request.clone());
        Ok(request)
    }

    async fn get_tty_input_request(&self, id: Uuid) -> TaskStoreResult<Option<TtyInputRequest>> {
        let requests = self.tty_input_requests.read().await;
        Ok(requests.get(&id).cloned())
    }

    async fn list_tty_input_requests(
        &self,
        filter: TtyInputFilter,
    ) -> TaskStoreResult<Vec<TtyInputRequest>> {
        let requests = self.tty_input_requests.read().await;
        let mut result: Vec<TtyInputRequest> = requests
            .values()
            .filter(|r| {
                let mut matches = true;
                if let Some(task_id) = filter.task_id {
                    matches = matches && r.task_id == task_id;
                }
                if let Some(session_id) = filter.session_id {
                    matches = matches && r.session_id == session_id;
                }
                if let Some(status) = filter.status {
                    matches = matches && r.status == status;
                }
                matches
            })
            .cloned()
            .collect();

        if let Some(offset) = filter.offset {
            result = result.into_iter().skip(offset as usize).collect();
        }
        if let Some(limit) = filter.limit {
            result = result.into_iter().take(limit as usize).collect();
        }

        Ok(result)
    }

    async fn update_tty_input_request(
        &self,
        request: TtyInputRequest,
    ) -> TaskStoreResult<TtyInputRequest> {
        let mut requests = self.tty_input_requests.write().await;
        if !requests.contains_key(&request.id) {
            return Err(TaskStoreError::not_found(
                "TtyInputRequest",
                request.id.to_string(),
            ));
        }
        requests.insert(request.id, request.clone());
        Ok(request)
    }

    async fn delete_tty_input_request(&self, id: Uuid) -> TaskStoreResult<()> {
        let mut requests = self.tty_input_requests.write().await;
        if requests.remove(&id).is_none() {
            return Err(TaskStoreError::not_found("TtyInputRequest", id.to_string()));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use entities::VcsProviderType;

    use super::*;

    #[tokio::test]
    async fn test_workspace_crud() {
        let store = MemoryTaskStore::new();

        // Create
        let workspace = Workspace::new("Test Workspace");
        let created = store.create_workspace(workspace.clone()).await.unwrap();
        assert_eq!(created.name, "Test Workspace");

        // Get
        let fetched = store.get_workspace(created.id).await.unwrap().unwrap();
        assert_eq!(fetched.name, "Test Workspace");

        // List
        let (workspaces, count) = store
            .list_workspaces(WorkspaceFilter::default())
            .await
            .unwrap();
        assert_eq!(count, 1);
        assert_eq!(workspaces.len(), 1);

        // Delete
        store.delete_workspace(created.id).await.unwrap();
        assert!(store.get_workspace(created.id).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_repository_crud() {
        let store = MemoryTaskStore::new();

        // Create workspace first
        let workspace = Workspace::new("Test Workspace");
        let workspace = store.create_workspace(workspace).await.unwrap();

        // Create repository
        let repo = Repository::new(
            workspace.id,
            "test-repo",
            "https://github.com/test/test-repo",
            VcsProviderType::Github,
        );
        let created = store.create_repository(repo).await.unwrap();
        assert_eq!(created.name, "test-repo");

        // Get
        let fetched = store.get_repository(created.id).await.unwrap().unwrap();
        assert_eq!(fetched.remote_url, "https://github.com/test/test-repo");

        // List by workspace
        let filter = RepositoryFilter {
            workspace_id: Some(workspace.id),
            ..Default::default()
        };
        let (repos, count) = store.list_repositories(filter).await.unwrap();
        assert_eq!(count, 1);
        assert_eq!(repos.len(), 1);

        // Delete
        store.delete_repository(created.id).await.unwrap();
        assert!(store.get_repository(created.id).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_unit_task_crud() {
        let store = MemoryTaskStore::new();

        // Setup
        let workspace = Workspace::new("Test Workspace");
        let workspace = store.create_workspace(workspace).await.unwrap();
        let group = RepositoryGroup::new(workspace.id);
        let group = store.create_repository_group(group).await.unwrap();
        let agent_task = AgentTask::new();
        let agent_task = store.create_agent_task(agent_task).await.unwrap();

        // Create unit task
        let task = UnitTask::new(group.id, agent_task.id, "Fix the bug");
        let created = store.create_unit_task(task).await.unwrap();
        assert_eq!(created.prompt, "Fix the bug");

        // Get
        let fetched = store.get_unit_task(created.id).await.unwrap().unwrap();
        assert_eq!(fetched.prompt, "Fix the bug");

        // List
        let (tasks, count) = store.list_unit_tasks(TaskFilter::default()).await.unwrap();
        assert_eq!(count, 1);
        assert_eq!(tasks.len(), 1);

        // Delete
        store.delete_unit_task(created.id).await.unwrap();
        assert!(store.get_unit_task(created.id).await.unwrap().is_none());
    }
}
