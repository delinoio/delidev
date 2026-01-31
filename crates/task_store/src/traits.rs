//! Task store trait definitions.

use async_trait::async_trait;
use entities::{
    AgentSession, AgentTask, CompositeTask, CompositeTaskNode, CompositeTaskStatus, Repository,
    RepositoryGroup, TodoItem, TodoItemStatus, TtyInputRequest, TtyInputStatus, UnitTask,
    UnitTaskStatus, User, Workspace,
};
use uuid::Uuid;

use crate::TaskStoreResult;

/// Filter options for listing tasks.
#[derive(Debug, Clone, Default)]
pub struct TaskFilter {
    /// Filter by repository group ID.
    pub repository_group_id: Option<Uuid>,
    /// Filter by unit task status.
    pub unit_status: Option<UnitTaskStatus>,
    /// Filter by composite task status.
    pub composite_status: Option<CompositeTaskStatus>,
    /// Maximum number of results.
    pub limit: Option<u32>,
    /// Offset for pagination.
    pub offset: Option<u32>,
}

/// Filter options for listing todo items.
#[derive(Debug, Clone, Default)]
pub struct TodoFilter {
    /// Filter by repository ID.
    pub repository_id: Option<Uuid>,
    /// Filter by status.
    pub status: Option<TodoItemStatus>,
    /// Maximum number of results.
    pub limit: Option<u32>,
    /// Offset for pagination.
    pub offset: Option<u32>,
}

/// Filter options for listing repositories.
#[derive(Debug, Clone, Default)]
pub struct RepositoryFilter {
    /// Filter by workspace ID.
    pub workspace_id: Option<Uuid>,
    /// Maximum number of results.
    pub limit: Option<u32>,
    /// Offset for pagination.
    pub offset: Option<u32>,
}

/// Filter options for listing workspaces.
#[derive(Debug, Clone, Default)]
pub struct WorkspaceFilter {
    /// Filter by user ID.
    pub user_id: Option<Uuid>,
    /// Maximum number of results.
    pub limit: Option<u32>,
    /// Offset for pagination.
    pub offset: Option<u32>,
}

/// Filter options for listing TTY input requests.
#[derive(Debug, Clone, Default)]
pub struct TtyInputFilter {
    /// Filter by task ID.
    pub task_id: Option<Uuid>,
    /// Filter by session ID.
    pub session_id: Option<Uuid>,
    /// Filter by status.
    pub status: Option<TtyInputStatus>,
    /// Maximum number of results.
    pub limit: Option<u32>,
    /// Offset for pagination.
    pub offset: Option<u32>,
}

/// Trait for task storage operations.
#[async_trait]
pub trait TaskStore: Send + Sync {
    // =========================================================================
    // User operations
    // =========================================================================

    /// Creates a new user.
    async fn create_user(&self, user: User) -> TaskStoreResult<User>;

    /// Gets a user by ID.
    async fn get_user(&self, id: Uuid) -> TaskStoreResult<Option<User>>;

    /// Gets a user by email.
    async fn get_user_by_email(&self, email: &str) -> TaskStoreResult<Option<User>>;

    /// Updates a user.
    async fn update_user(&self, user: User) -> TaskStoreResult<User>;

    /// Deletes a user.
    async fn delete_user(&self, id: Uuid) -> TaskStoreResult<()>;

    // =========================================================================
    // Workspace operations
    // =========================================================================

    /// Creates a new workspace.
    async fn create_workspace(&self, workspace: Workspace) -> TaskStoreResult<Workspace>;

    /// Gets a workspace by ID.
    async fn get_workspace(&self, id: Uuid) -> TaskStoreResult<Option<Workspace>>;

    /// Lists workspaces with optional filters.
    async fn list_workspaces(&self, filter: WorkspaceFilter)
        -> TaskStoreResult<(Vec<Workspace>, u32)>;

    /// Updates a workspace.
    async fn update_workspace(&self, workspace: Workspace) -> TaskStoreResult<Workspace>;

    /// Deletes a workspace.
    async fn delete_workspace(&self, id: Uuid) -> TaskStoreResult<()>;

    // =========================================================================
    // Repository operations
    // =========================================================================

    /// Creates a new repository.
    async fn create_repository(&self, repository: Repository) -> TaskStoreResult<Repository>;

    /// Gets a repository by ID.
    async fn get_repository(&self, id: Uuid) -> TaskStoreResult<Option<Repository>>;

    /// Lists repositories with optional filters.
    async fn list_repositories(
        &self,
        filter: RepositoryFilter,
    ) -> TaskStoreResult<(Vec<Repository>, u32)>;

    /// Updates a repository.
    async fn update_repository(&self, repository: Repository) -> TaskStoreResult<Repository>;

    /// Deletes a repository.
    async fn delete_repository(&self, id: Uuid) -> TaskStoreResult<()>;

    // =========================================================================
    // Repository Group operations
    // =========================================================================

    /// Creates a new repository group.
    async fn create_repository_group(
        &self,
        group: RepositoryGroup,
    ) -> TaskStoreResult<RepositoryGroup>;

    /// Gets a repository group by ID.
    async fn get_repository_group(&self, id: Uuid) -> TaskStoreResult<Option<RepositoryGroup>>;

    /// Lists repository groups by workspace ID.
    async fn list_repository_groups(
        &self,
        workspace_id: Option<Uuid>,
    ) -> TaskStoreResult<Vec<RepositoryGroup>>;

    /// Updates a repository group.
    async fn update_repository_group(
        &self,
        group: RepositoryGroup,
    ) -> TaskStoreResult<RepositoryGroup>;

    /// Deletes a repository group.
    async fn delete_repository_group(&self, id: Uuid) -> TaskStoreResult<()>;

    // =========================================================================
    // Agent Task operations
    // =========================================================================

    /// Creates a new agent task.
    async fn create_agent_task(&self, task: AgentTask) -> TaskStoreResult<AgentTask>;

    /// Gets an agent task by ID.
    async fn get_agent_task(&self, id: Uuid) -> TaskStoreResult<Option<AgentTask>>;

    /// Updates an agent task.
    async fn update_agent_task(&self, task: AgentTask) -> TaskStoreResult<AgentTask>;

    /// Deletes an agent task.
    async fn delete_agent_task(&self, id: Uuid) -> TaskStoreResult<()>;

    // =========================================================================
    // Agent Session operations
    // =========================================================================

    /// Creates a new agent session.
    async fn create_agent_session(&self, session: AgentSession) -> TaskStoreResult<AgentSession>;

    /// Gets an agent session by ID.
    async fn get_agent_session(&self, id: Uuid) -> TaskStoreResult<Option<AgentSession>>;

    /// Lists agent sessions by agent task ID.
    async fn list_agent_sessions(&self, agent_task_id: Uuid)
        -> TaskStoreResult<Vec<AgentSession>>;

    /// Updates an agent session.
    async fn update_agent_session(&self, session: AgentSession) -> TaskStoreResult<AgentSession>;

    /// Deletes an agent session.
    async fn delete_agent_session(&self, id: Uuid) -> TaskStoreResult<()>;

    // =========================================================================
    // Unit Task operations
    // =========================================================================

    /// Creates a new unit task.
    async fn create_unit_task(&self, task: UnitTask) -> TaskStoreResult<UnitTask>;

    /// Gets a unit task by ID.
    async fn get_unit_task(&self, id: Uuid) -> TaskStoreResult<Option<UnitTask>>;

    /// Lists unit tasks with optional filters.
    async fn list_unit_tasks(&self, filter: TaskFilter) -> TaskStoreResult<(Vec<UnitTask>, u32)>;

    /// Updates a unit task.
    async fn update_unit_task(&self, task: UnitTask) -> TaskStoreResult<UnitTask>;

    /// Deletes a unit task.
    async fn delete_unit_task(&self, id: Uuid) -> TaskStoreResult<()>;

    // =========================================================================
    // Composite Task operations
    // =========================================================================

    /// Creates a new composite task.
    async fn create_composite_task(&self, task: CompositeTask) -> TaskStoreResult<CompositeTask>;

    /// Gets a composite task by ID.
    async fn get_composite_task(&self, id: Uuid) -> TaskStoreResult<Option<CompositeTask>>;

    /// Lists composite tasks with optional filters.
    async fn list_composite_tasks(
        &self,
        filter: TaskFilter,
    ) -> TaskStoreResult<(Vec<CompositeTask>, u32)>;

    /// Updates a composite task.
    async fn update_composite_task(&self, task: CompositeTask) -> TaskStoreResult<CompositeTask>;

    /// Deletes a composite task.
    async fn delete_composite_task(&self, id: Uuid) -> TaskStoreResult<()>;

    // =========================================================================
    // Composite Task Node operations
    // =========================================================================

    /// Creates a new composite task node.
    async fn create_composite_task_node(
        &self,
        node: CompositeTaskNode,
    ) -> TaskStoreResult<CompositeTaskNode>;

    /// Gets a composite task node by ID.
    async fn get_composite_task_node(
        &self,
        id: Uuid,
    ) -> TaskStoreResult<Option<CompositeTaskNode>>;

    /// Lists composite task nodes by composite task ID.
    async fn list_composite_task_nodes(
        &self,
        composite_task_id: Uuid,
    ) -> TaskStoreResult<Vec<CompositeTaskNode>>;

    /// Updates a composite task node.
    async fn update_composite_task_node(
        &self,
        node: CompositeTaskNode,
    ) -> TaskStoreResult<CompositeTaskNode>;

    /// Deletes a composite task node.
    async fn delete_composite_task_node(&self, id: Uuid) -> TaskStoreResult<()>;

    // =========================================================================
    // Todo Item operations
    // =========================================================================

    /// Creates a new todo item.
    async fn create_todo_item(&self, item: TodoItem) -> TaskStoreResult<TodoItem>;

    /// Gets a todo item by ID.
    async fn get_todo_item(&self, id: Uuid) -> TaskStoreResult<Option<TodoItem>>;

    /// Lists todo items with optional filters.
    async fn list_todo_items(&self, filter: TodoFilter) -> TaskStoreResult<(Vec<TodoItem>, u32)>;

    /// Updates a todo item.
    async fn update_todo_item(&self, item: TodoItem) -> TaskStoreResult<TodoItem>;

    /// Deletes a todo item.
    async fn delete_todo_item(&self, id: Uuid) -> TaskStoreResult<()>;

    // =========================================================================
    // TTY Input Request operations
    // =========================================================================

    /// Creates a new TTY input request.
    async fn create_tty_input_request(
        &self,
        request: TtyInputRequest,
    ) -> TaskStoreResult<TtyInputRequest>;

    /// Gets a TTY input request by ID.
    async fn get_tty_input_request(&self, id: Uuid) -> TaskStoreResult<Option<TtyInputRequest>>;

    /// Lists TTY input requests with optional filters.
    async fn list_tty_input_requests(
        &self,
        filter: TtyInputFilter,
    ) -> TaskStoreResult<Vec<TtyInputRequest>>;

    /// Updates a TTY input request.
    async fn update_tty_input_request(
        &self,
        request: TtyInputRequest,
    ) -> TaskStoreResult<TtyInputRequest>;

    /// Deletes a TTY input request.
    async fn delete_tty_input_request(&self, id: Uuid) -> TaskStoreResult<()>;
}
