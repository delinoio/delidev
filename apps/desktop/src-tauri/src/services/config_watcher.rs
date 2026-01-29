use std::{collections::HashSet, path::PathBuf, sync::Arc, time::Duration};

use notify_debouncer_mini::{new_debouncer, DebouncedEventKind};
use tauri::{AppHandle, Emitter};
use tokio::sync::{mpsc, RwLock};

use super::{AgentExecutionService, DockerService, GitService, NotificationService};
use crate::{
    config::ConfigManager,
    entities::{GlobalConfig, RepositoryConfig},
    services::{RepositoryService, TaskService},
};

/// Event emitted when global config changes
pub const GLOBAL_CONFIG_CHANGED_EVENT: &str = "global-config-changed";
/// Event emitted when repository config changes
pub const REPOSITORY_CONFIG_CHANGED_EVENT: &str = "repository-config-changed";

/// Payload for repository config changed event
#[derive(Clone, serde::Serialize)]
pub struct RepositoryConfigChangedPayload {
    pub repo_path: String,
    pub config: RepositoryConfig,
}

/// Config watcher service that monitors config files for changes
pub struct ConfigWatcherService {
    /// App handle for emitting events
    app_handle: AppHandle,
    /// Config manager for loading configs
    config_manager: Arc<ConfigManager>,
    /// Global config state to update
    global_config: Arc<RwLock<GlobalConfig>>,
    /// Repository paths being watched
    watched_repos: Arc<RwLock<HashSet<PathBuf>>>,
    /// Shutdown signal sender
    shutdown_tx: Option<mpsc::Sender<()>>,
    /// Docker service for reinitializing agent execution service
    docker_service: Arc<RwLock<Option<Arc<DockerService>>>>,
    /// Agent execution service to reinitialize on config change
    agent_execution_service: Arc<RwLock<Option<Arc<AgentExecutionService>>>>,
    /// Git service for creating agent execution service
    git_service: Arc<GitService>,
    /// Task service for creating agent execution service
    task_service: Arc<TaskService>,
    /// Repository service for creating agent execution service
    repository_service: Arc<RepositoryService>,
    /// Notification service for creating agent execution service
    notification_service: Arc<NotificationService>,
}

impl ConfigWatcherService {
    /// Creates a new config watcher service
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        app_handle: AppHandle,
        config_manager: Arc<ConfigManager>,
        global_config: Arc<RwLock<GlobalConfig>>,
        docker_service: Arc<RwLock<Option<Arc<DockerService>>>>,
        agent_execution_service: Arc<RwLock<Option<Arc<AgentExecutionService>>>>,
        git_service: Arc<GitService>,
        task_service: Arc<TaskService>,
        repository_service: Arc<RepositoryService>,
        notification_service: Arc<NotificationService>,
    ) -> Self {
        Self {
            app_handle,
            config_manager,
            global_config,
            watched_repos: Arc::new(RwLock::new(HashSet::new())),
            shutdown_tx: None,
            docker_service,
            agent_execution_service,
            git_service,
            task_service,
            repository_service,
            notification_service,
        }
    }

    /// Starts watching config files
    pub async fn start(&mut self) -> anyhow::Result<()> {
        let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);
        self.shutdown_tx = Some(shutdown_tx);

        let global_config_path = self.config_manager.global_config_path();
        let global_config_dir = global_config_path.parent().map(|p| p.to_path_buf());

        let app_handle = self.app_handle.clone();
        let config_manager = self.config_manager.clone();
        let global_config = self.global_config.clone();
        let watched_repos = self.watched_repos.clone();
        let docker_service = self.docker_service.clone();
        let agent_execution_service = self.agent_execution_service.clone();
        let git_service = self.git_service.clone();
        let task_service = self.task_service.clone();
        let repository_service = self.repository_service.clone();
        let notification_service = self.notification_service.clone();

        // Create a channel for receiving file system events
        let (tx, mut rx) = mpsc::channel(100);

        // Clone for the watcher thread
        let tx_clone = tx.clone();

        // Spawn a blocking task for the file watcher
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Failed to create tokio runtime for watcher");

            rt.block_on(async move {
                // Create debounced watcher
                let (debounce_tx, debounce_rx) = std::sync::mpsc::channel();
                let mut debouncer = match new_debouncer(Duration::from_millis(500), debounce_tx) {
                    Ok(d) => d,
                    Err(e) => {
                        tracing::error!("Failed to create file watcher: {}", e);
                        return;
                    }
                };

                // Watch global config directory
                if let Some(ref dir) = global_config_dir {
                    if dir.exists() {
                        if let Err(e) = debouncer
                            .watcher()
                            .watch(dir, notify::RecursiveMode::NonRecursive)
                        {
                            tracing::warn!("Failed to watch global config directory: {}", e);
                        } else {
                            tracing::info!("Watching global config directory: {:?}", dir);
                        }
                    }
                }

                // Process events from debouncer
                loop {
                    match debounce_rx.recv_timeout(Duration::from_millis(100)) {
                        Ok(Ok(events)) => {
                            for event in events {
                                if event.kind == DebouncedEventKind::Any
                                    && tx_clone.send(event.path).await.is_err()
                                {
                                    return;
                                }
                            }
                        }
                        Ok(Err(e)) => {
                            tracing::warn!("File watcher error: {:?}", e);
                        }
                        Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                            // Check if we should exit
                            continue;
                        }
                        Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                            return;
                        }
                    }
                }
            });
        });

        // Spawn task to handle file change events
        let global_config_path_clone = global_config_path.clone();
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    Some(path) = rx.recv() => {
                        Self::handle_file_change(
                            &path,
                            &global_config_path_clone,
                            &app_handle,
                            &config_manager,
                            &global_config,
                            &watched_repos,
                            &docker_service,
                            &agent_execution_service,
                            &git_service,
                            &task_service,
                            &repository_service,
                            &notification_service,
                        ).await;
                    }
                    _ = shutdown_rx.recv() => {
                        tracing::info!("Config watcher shutting down");
                        break;
                    }
                }
            }
        });

        tracing::info!("Config watcher started");
        Ok(())
    }

    /// Handles a file change event
    #[allow(clippy::too_many_arguments)]
    async fn handle_file_change(
        path: &PathBuf,
        global_config_path: &PathBuf,
        app_handle: &AppHandle,
        config_manager: &Arc<ConfigManager>,
        global_config: &Arc<RwLock<GlobalConfig>>,
        watched_repos: &Arc<RwLock<HashSet<PathBuf>>>,
        docker_service: &Arc<RwLock<Option<Arc<DockerService>>>>,
        agent_execution_service: &Arc<RwLock<Option<Arc<AgentExecutionService>>>>,
        git_service: &Arc<GitService>,
        task_service: &Arc<TaskService>,
        repository_service: &Arc<RepositoryService>,
        notification_service: &Arc<NotificationService>,
    ) {
        let file_name = path.file_name().and_then(|n| n.to_str());

        // Check if this is the global config file
        if path == global_config_path
            || file_name == Some("config.toml") && path.parent() == global_config_path.parent()
        {
            tracing::info!("Global config file changed, reloading...");
            match config_manager.load_global_config() {
                Ok(new_config) => {
                    // Check if use_container setting changed
                    let old_use_container = global_config.read().await.container.use_container;
                    let new_use_container = new_config.container.use_container;

                    *global_config.write().await = new_config.clone();

                    // Reinitialize agent execution service if use_container changed
                    if old_use_container != new_use_container {
                        tracing::info!(
                            "use_container changed from {} to {}, reinitializing agent execution \
                             service",
                            old_use_container,
                            new_use_container
                        );
                        Self::reinit_agent_execution_service(
                            new_use_container,
                            app_handle,
                            docker_service,
                            agent_execution_service,
                            git_service,
                            task_service,
                            repository_service,
                            notification_service,
                        )
                        .await;
                    }

                    if let Err(e) = app_handle.emit(GLOBAL_CONFIG_CHANGED_EVENT, new_config) {
                        tracing::warn!("Failed to emit global config changed event: {}", e);
                    } else {
                        tracing::info!("Global config reloaded and event emitted");
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to reload global config: {}", e);
                }
            }
            return;
        }

        // Check if this is a repository config file
        // Repository config is at <repo_path>/.delidev/config.toml
        if file_name == Some("config.toml") {
            if let Some(delidev_dir) = path.parent() {
                if delidev_dir.file_name().and_then(|n| n.to_str()) == Some(".delidev") {
                    if let Some(repo_path) = delidev_dir.parent() {
                        let watched = watched_repos.read().await;
                        if watched.contains(repo_path) {
                            tracing::info!("Repository config changed: {:?}", repo_path);
                            match ConfigManager::load_repository_config(repo_path) {
                                Ok(new_config) => {
                                    let payload = RepositoryConfigChangedPayload {
                                        repo_path: repo_path.to_string_lossy().to_string(),
                                        config: new_config,
                                    };
                                    if let Err(e) =
                                        app_handle.emit(REPOSITORY_CONFIG_CHANGED_EVENT, payload)
                                    {
                                        tracing::warn!(
                                            "Failed to emit repository config changed event: {}",
                                            e
                                        );
                                    } else {
                                        tracing::info!(
                                            "Repository config reloaded and event emitted"
                                        );
                                    }
                                }
                                Err(e) => {
                                    tracing::warn!("Failed to reload repository config: {}", e);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// Reinitializes the agent execution service based on use_container setting
    #[allow(clippy::too_many_arguments)]
    async fn reinit_agent_execution_service(
        use_container: bool,
        app_handle: &AppHandle,
        docker_service: &Arc<RwLock<Option<Arc<DockerService>>>>,
        agent_execution_service: &Arc<RwLock<Option<Arc<AgentExecutionService>>>>,
        git_service: &Arc<GitService>,
        task_service: &Arc<TaskService>,
        repository_service: &Arc<RepositoryService>,
        notification_service: &Arc<NotificationService>,
    ) {
        let new_service = if use_container {
            // Container mode: requires Docker service
            let docker_guard = docker_service.read().await;
            if let Some(docker) = docker_guard.as_ref() {
                tracing::info!(
                    "Reinitializing agent execution service with container mode ({})",
                    docker.runtime_name()
                );
                let mut service = AgentExecutionService::new(
                    docker.clone(),
                    git_service.clone(),
                    task_service.clone(),
                    repository_service.clone(),
                );
                service = service.with_app_handle(app_handle.clone());
                service = service.with_notification_service(notification_service.clone());
                Some(Arc::new(service))
            } else {
                tracing::warn!("Cannot enable container mode: container runtime not available");
                None
            }
        } else {
            // Direct execution mode
            tracing::info!("Reinitializing agent execution service with direct execution mode");
            let mut service = AgentExecutionService::new_direct(
                git_service.clone(),
                task_service.clone(),
                repository_service.clone(),
            );
            service = service.with_app_handle(app_handle.clone());
            service = service.with_notification_service(notification_service.clone());
            Some(Arc::new(service))
        };

        *agent_execution_service.write().await = new_service;
    }

    /// Adds a repository to watch for config changes
    pub async fn watch_repository(&self, repo_path: PathBuf) {
        let mut watched = self.watched_repos.write().await;
        if watched.insert(repo_path.clone()) {
            tracing::info!("Now watching repository config: {:?}", repo_path);
        }
    }

    /// Removes a repository from watch list
    pub async fn unwatch_repository(&self, repo_path: &PathBuf) {
        let mut watched = self.watched_repos.write().await;
        if watched.remove(repo_path) {
            tracing::info!("Stopped watching repository config: {:?}", repo_path);
        }
    }

    /// Stops the config watcher
    pub async fn stop(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(()).await;
        }
    }
}
