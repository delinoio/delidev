use std::sync::Arc;

use tauri::{AppHandle, Emitter};
use tokio::sync::{mpsc, RwLock};

use super::{
    concurrency::PendingTask, AgentExecutionService, ConcurrencyService, ConfigWatcherService,
    DockerService, GitService, LicenseService, NotificationService, RepositoryGroupService,
    RepositoryService, TaskService, VCSProviderService, WorkspaceService, WorktreeCleanupService,
};
use crate::{
    config::ConfigManager,
    database::Database,
    entities::{GlobalConfig, UnitTaskStatus, VCSCredentials},
};

/// Application state shared across all commands
pub struct AppState {
    /// Database connection
    pub db: Arc<Database>,
    /// Configuration manager
    pub config_manager: Arc<ConfigManager>,
    /// Global configuration (cached)
    pub global_config: Arc<RwLock<GlobalConfig>>,
    /// VCS credentials (cached)
    pub credentials: Arc<RwLock<VCSCredentials>>,
    /// Repository service
    pub repository_service: Arc<RepositoryService>,
    /// Workspace service
    pub workspace_service: Arc<WorkspaceService>,
    /// Repository group service
    pub repository_group_service: Arc<RepositoryGroupService>,
    /// Task service
    pub task_service: Arc<TaskService>,
    /// Docker service (optional - may not be available if Docker is not
    /// installed) Wrapped in RwLock to allow dynamic re-initialization
    pub docker_service: Arc<RwLock<Option<Arc<DockerService>>>>,
    /// VCS provider service
    pub vcs_service: Arc<VCSProviderService>,
    /// Git service
    pub git_service: Arc<GitService>,
    /// Agent execution service (optional - requires Docker)
    /// Wrapped in RwLock to allow dynamic re-initialization
    pub agent_execution_service: Arc<RwLock<Option<Arc<AgentExecutionService>>>>,
    /// Notification service
    pub notification_service: Arc<NotificationService>,
    /// License service for Polar.sh integration
    pub license_service: Arc<LicenseService>,
    /// Config watcher service for hot-reloading configs
    pub config_watcher: Arc<RwLock<Option<ConfigWatcherService>>>,
    /// Worktree cleanup service for periodic orphaned worktree cleanup
    pub worktree_cleanup_service: Arc<WorktreeCleanupService>,
    /// Concurrency service for managing agent session limits (premium feature)
    pub concurrency_service: Arc<ConcurrencyService>,
    /// App handle for event emission (used when re-initializing services)
    app_handle: Option<AppHandle>,
}

impl AppState {
    /// Creates a new application state
    pub async fn new(app_handle: Option<AppHandle>) -> anyhow::Result<Self> {
        // Initialize config manager
        let config_manager = Arc::new(ConfigManager::new()?);

        // Load global configuration
        let global_config = config_manager.load_global_config()?;

        // Load credentials
        let credentials = config_manager.load_credentials()?;

        // Initialize database
        let db_path = config_manager.database_path();
        let db = Arc::new(Database::new(&db_path).await?);

        // Initialize services
        let repository_service = Arc::new(RepositoryService::new(db.clone()));
        let workspace_service = Arc::new(WorkspaceService::new(db.clone()));
        let repository_group_service = Arc::new(RepositoryGroupService::new(db.clone()));
        let task_service = Arc::new(TaskService::new(db.clone()));
        let git_service = Arc::new(GitService::new());

        // Ensure default workspace exists
        if let Err(e) = workspace_service.get_or_create_default().await {
            tracing::warn!("Failed to create default workspace: {}", e);
        }
        // Initialize container service with configured runtime
        let docker_service = match DockerService::with_runtime(
            global_config.container.runtime,
            global_config.container.socket_path.clone(),
        ) {
            Ok(service) => {
                tracing::info!("Container runtime initialized: {}", service.runtime_name());
                Some(Arc::new(service))
            }
            Err(e) => {
                tracing::warn!(
                    "{} service unavailable: {}. Container-related features will be disabled.",
                    global_config.container.runtime.display_name(),
                    e
                );
                None
            }
        };
        let vcs_service = Arc::new(VCSProviderService::new());
        let notification_service = Arc::new(NotificationService::new(app_handle.clone()));
        let license_service = Arc::new(LicenseService::new(config_manager.clone()));

        // Initialize concurrency service early so it can be passed to execution
        // service
        let concurrency_service = Arc::new(ConcurrencyService::new(license_service.clone()));

        // Create global config Arc for sharing
        let global_config = Arc::new(RwLock::new(global_config));

        // Initialize agent execution service
        // - If use_container is false, create service for direct execution
        // - If use_container is true and Docker is available, create service with
        //   Docker
        // - If use_container is true but Docker is unavailable, no service
        let agent_execution_service = if global_config.read().await.container.use_container {
            // Container mode: requires Docker service
            docker_service.as_ref().map(|docker| {
                let mut service = AgentExecutionService::new(
                    docker.clone(),
                    git_service.clone(),
                    task_service.clone(),
                    repository_service.clone(),
                );
                // Set repository group service
                service = service.with_repository_group_service(repository_group_service.clone());
                // Set app handle for event emission if available
                if let Some(handle) = app_handle.clone() {
                    service = service.with_app_handle(handle);
                }
                // Set notification service
                service = service.with_notification_service(notification_service.clone());
                // Set concurrency service for dependent task execution
                service = service
                    .with_concurrency_service(concurrency_service.clone(), global_config.clone());
                Arc::new(service)
            })
        } else {
            // Direct execution mode: no Docker required
            tracing::info!("Container execution disabled, using direct execution mode");
            let mut service = AgentExecutionService::new_direct(
                git_service.clone(),
                task_service.clone(),
                repository_service.clone(),
            );
            // Set repository group service
            service = service.with_repository_group_service(repository_group_service.clone());
            // Set app handle for event emission if available
            if let Some(handle) = app_handle.clone() {
                service = service.with_app_handle(handle);
            }
            // Set notification service
            service = service.with_notification_service(notification_service.clone());
            // Set concurrency service for dependent task execution
            service = service
                .with_concurrency_service(concurrency_service.clone(), global_config.clone());
            Some(Arc::new(service))
        };

        // Wrap services in Arc<RwLock<>> for sharing with config watcher
        let docker_service = Arc::new(RwLock::new(docker_service));
        let agent_execution_service = Arc::new(RwLock::new(agent_execution_service));

        // Initialize config watcher if we have an app handle
        let config_watcher = if let Some(ref handle) = app_handle {
            let mut watcher = ConfigWatcherService::new(
                handle.clone(),
                config_manager.clone(),
                global_config.clone(),
                docker_service.clone(),
                agent_execution_service.clone(),
                git_service.clone(),
                task_service.clone(),
                repository_service.clone(),
                notification_service.clone(),
            );
            if let Err(e) = watcher.start().await {
                tracing::warn!("Failed to start config watcher: {}", e);
                None
            } else {
                Some(watcher)
            }
        } else {
            None
        };

        // Initialize worktree cleanup service
        let worktree_cleanup_service = Arc::new(WorktreeCleanupService::new(
            git_service.clone(),
            task_service.clone(),
            repository_service.clone(),
        ));
        // Start the periodic cleanup (runs every hour by default)
        worktree_cleanup_service.start();

        let app_state = Self {
            db,
            config_manager,
            global_config,
            credentials: Arc::new(RwLock::new(credentials)),
            repository_service,
            workspace_service,
            repository_group_service,
            task_service,
            docker_service,
            vcs_service,
            git_service,
            agent_execution_service,
            notification_service,
            license_service,
            config_watcher: Arc::new(RwLock::new(config_watcher)),
            worktree_cleanup_service,
            concurrency_service,
            app_handle,
        };

        Ok(app_state)
    }

    /// Starts the pending task handler that processes tasks from the pending
    /// queue. This should be called once after the AppState is wrapped in an
    /// Arc.
    pub async fn start_pending_task_handler(self: &Arc<Self>) {
        let (tx, rx) = mpsc::unbounded_channel::<PendingTask>();
        self.concurrency_service
            .set_slot_available_channel(tx)
            .await;

        // Spawn the handler task
        let state = Arc::clone(self);
        tokio::spawn(async move {
            state.handle_pending_tasks(rx).await;
        });
    }

    /// Handles pending tasks from the channel.
    /// This runs in a background task and processes pending tasks as slots
    /// become available.
    async fn handle_pending_tasks(self: Arc<Self>, mut rx: mpsc::UnboundedReceiver<PendingTask>) {
        while let Some(pending_task) = rx.recv().await {
            tracing::info!(
                "Processing pending task from queue: {}",
                pending_task.task_id
            );

            // Execute the pending task
            if let Err(e) = self.execute_pending_task(&pending_task.task_id).await {
                tracing::error!(
                    "Failed to execute pending task {}: {}",
                    pending_task.task_id,
                    e
                );
            }
        }
    }

    /// Executes a pending task by its ID.
    async fn execute_pending_task(&self, task_id: &str) -> anyhow::Result<()> {
        // Get the UnitTask
        let unit_task = self
            .task_service
            .get_unit_task(task_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("UnitTask not found: {}", task_id))?;

        // Check if the task is still in a valid state to execute.
        // Only InProgress tasks should be executed - tasks that are already
        // InReview, Done, Approved, etc. should not be re-executed.
        if unit_task.status != UnitTaskStatus::InProgress {
            tracing::info!(
                "Pending task {} is no longer in progress (status: {:?}), skipping execution",
                task_id,
                unit_task.status
            );
            return Ok(());
        }

        // Get the execution service
        let exec_service = {
            let guard = self.agent_execution_service.read().await;
            guard
                .as_ref()
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("Execution service not available"))?
        };

        // Get the AgentTask
        let agent_task = self
            .task_service
            .get_agent_task(&unit_task.agent_task_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("AgentTask not found: {}", unit_task.agent_task_id))?;

        // Try to acquire a concurrency slot
        let task_guard = {
            let global_config = self.global_config.read().await;
            match self
                .concurrency_service
                .try_start_task(&global_config.concurrency, task_id)
                .await
            {
                Ok(guard) => guard,
                Err(e) => {
                    // Still can't start, re-queue the task
                    tracing::warn!(
                        "Still cannot start pending task {} due to concurrency limit: {}. \
                         Re-queuing.",
                        task_id,
                        e
                    );
                    self.concurrency_service.add_pending_task(task_id).await;
                    return Ok(());
                }
            }
        };

        // Execute the task in a background task
        let exec_service_clone = exec_service.clone();
        let unit_task_clone = unit_task.clone();
        let agent_task_clone = agent_task.clone();
        let task_service = self.task_service.clone();

        tokio::spawn(async move {
            // TaskGuard is moved into this closure - it will auto-unregister on drop
            let _guard = task_guard;

            if let Err(e) = exec_service_clone
                .execute_unit_task(&unit_task_clone, &agent_task_clone)
                .await
            {
                tracing::error!(
                    "Pending task {} execution failed: {}",
                    unit_task_clone.id,
                    e
                );
            }

            // After execution completes, check if task is Done and trigger
            // dependent tasks
            if let Ok(Some(updated_task)) = task_service.get_unit_task(&unit_task_clone.id).await {
                if updated_task.status == UnitTaskStatus::Done {
                    exec_service_clone
                        .trigger_dependent_tasks(&unit_task_clone.id)
                        .await;
                }
            }
        });

        Ok(())
    }

    /// Tries to initialize or re-initialize the Docker service
    /// Returns true if Docker is now available
    pub async fn try_init_docker_service(&self) -> bool {
        // First check if we already have a working docker service
        {
            let docker_guard = self.docker_service.read().await;
            if let Some(docker) = docker_guard.as_ref() {
                if docker.is_available().await {
                    return true;
                }
            }
        }

        // Try to create a new docker service
        let global_config = self.global_config.read().await;
        match DockerService::with_runtime(
            global_config.container.runtime,
            global_config.container.socket_path.clone(),
        ) {
            Ok(service) => {
                let service = Arc::new(service);
                // Check if it's actually available
                if !service.is_available().await {
                    return false;
                }

                tracing::info!("Container runtime initialized: {}", service.runtime_name());

                // Create agent execution service
                let mut agent_service = AgentExecutionService::new(
                    service.clone(),
                    self.git_service.clone(),
                    self.task_service.clone(),
                    self.repository_service.clone(),
                );
                if let Some(handle) = self.app_handle.clone() {
                    agent_service = agent_service.with_app_handle(handle);
                }
                agent_service =
                    agent_service.with_notification_service(self.notification_service.clone());

                // Update both services
                *self.docker_service.write().await = Some(service);
                *self.agent_execution_service.write().await = Some(Arc::new(agent_service));

                true
            }
            Err(e) => {
                tracing::debug!("Failed to initialize container runtime: {}", e);
                false
            }
        }
    }

    /// Reloads global configuration from file
    pub async fn reload_global_config(&self) -> anyhow::Result<()> {
        let config = self.config_manager.load_global_config()?;
        *self.global_config.write().await = config;
        Ok(())
    }

    /// Saves and updates global configuration
    pub async fn update_global_config(&self, config: GlobalConfig) -> anyhow::Result<()> {
        self.config_manager.save_global_config(&config)?;
        *self.global_config.write().await = config;
        Ok(())
    }

    /// Reloads credentials from file
    pub async fn reload_credentials(&self) -> anyhow::Result<()> {
        let creds = self.config_manager.load_credentials()?;
        *self.credentials.write().await = creds;
        Ok(())
    }

    /// Saves and updates credentials
    pub async fn update_credentials(&self, creds: VCSCredentials) -> anyhow::Result<()> {
        self.config_manager.save_credentials(&creds)?;
        *self.credentials.write().await = creds;
        Ok(())
    }

    /// Watches a repository for config changes
    pub async fn watch_repository_config(&self, repo_path: std::path::PathBuf) {
        if let Some(watcher) = self.config_watcher.read().await.as_ref() {
            watcher.watch_repository(repo_path).await;
        }
    }

    /// Stops watching a repository for config changes
    pub async fn unwatch_repository_config(&self, repo_path: &std::path::PathBuf) {
        if let Some(watcher) = self.config_watcher.read().await.as_ref() {
            watcher.unwatch_repository(repo_path).await;
        }
    }

    /// Emits a task status change event to the frontend.
    ///
    /// This is used for status transitions of both `UnitTask` and
    /// `CompositeTask` instances. The `task_id` must uniquely identify the
    /// task whose status changed, regardless of whether it is a unit or
    /// composite task.
    ///
    /// The `old_status` and `new_status` values are expected to be snake_case
    /// strings representing the task's status, such as `"pending_approval"` or
    /// `"in_progress"`. Callers should ensure these values follow this format
    /// so that the frontend can reliably interpret and display them.
    pub fn emit_task_status_change(&self, task_id: &str, old_status: &str, new_status: &str) {
        if let Some(app) = &self.app_handle {
            #[derive(Clone, serde::Serialize)]
            struct TaskStatusEvent {
                task_id: String,
                old_status: String,
                new_status: String,
            }

            let event = TaskStatusEvent {
                task_id: task_id.to_string(),
                old_status: old_status.to_string(),
                new_status: new_status.to_string(),
            };
            if let Err(e) = app.emit("task-status-changed", event) {
                tracing::warn!("Failed to emit task-status-changed event: {}", e);
            }
        }
    }

    /// Reinitializes the agent execution service based on current config.
    /// This should be called when use_container setting changes.
    pub async fn reinit_agent_execution_service(&self) {
        let global_config_guard = self.global_config.read().await;
        let use_container = global_config_guard.container.use_container;
        drop(global_config_guard);

        let new_service = if use_container {
            // Container mode: requires Docker service
            let docker_guard = self.docker_service.read().await;
            if let Some(docker) = docker_guard.as_ref() {
                tracing::info!(
                    "Reinitializing agent execution service with container mode ({})",
                    docker.runtime_name()
                );
                let mut service = AgentExecutionService::new(
                    docker.clone(),
                    self.git_service.clone(),
                    self.task_service.clone(),
                    self.repository_service.clone(),
                );
                service =
                    service.with_repository_group_service(self.repository_group_service.clone());
                if let Some(handle) = self.app_handle.clone() {
                    service = service.with_app_handle(handle);
                }
                service = service.with_notification_service(self.notification_service.clone());
                service = service.with_concurrency_service(
                    self.concurrency_service.clone(),
                    self.global_config.clone(),
                );
                Some(Arc::new(service))
            } else {
                tracing::warn!("Cannot enable container mode: container runtime not available");
                None
            }
        } else {
            // Direct execution mode
            tracing::info!("Reinitializing agent execution service with direct execution mode");
            let mut service = AgentExecutionService::new_direct(
                self.git_service.clone(),
                self.task_service.clone(),
                self.repository_service.clone(),
            );
            service = service.with_repository_group_service(self.repository_group_service.clone());
            if let Some(handle) = self.app_handle.clone() {
                service = service.with_app_handle(handle);
            }
            service = service.with_notification_service(self.notification_service.clone());
            service = service.with_concurrency_service(
                self.concurrency_service.clone(),
                self.global_config.clone(),
            );
            Some(Arc::new(service))
        };

        *self.agent_execution_service.write().await = new_service;
    }
}
