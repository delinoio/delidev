use std::{path::PathBuf, sync::Arc};

use tauri::{AppHandle, Emitter};
use thiserror::Error;
use tokio::sync::RwLock;

use super::{
    concurrency::{ConcurrencyError, ConcurrencyService},
    DockerService, GitService, NotificationService, RepositoryService, TaskService,
};
use crate::entities::{
    AIAgentType, AgentSession, AgentStreamMessage, AgentTask, ClaudeStreamMessage,
    ExecutionContext, ExecutionLog, LogLevel, OpenCodeMessage, UnitTask, UnitTaskStatus,
};

/// Execution phase for progress tracking
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionPhase {
    /// Initializing execution
    Starting,
    /// Creating git worktree
    Worktree,
    /// Creating Docker container
    Container,
    /// Running setup commands
    Setup,
    /// Executing Claude Code
    Executing,
    /// Cleaning up resources
    Cleanup,
    /// Execution completed successfully
    Completed,
    /// Execution failed
    Failed,
}

impl ExecutionPhase {
    /// Returns the string representation for event emission
    pub fn as_str(&self) -> &'static str {
        match self {
            ExecutionPhase::Starting => "starting",
            ExecutionPhase::Worktree => "worktree",
            ExecutionPhase::Container => "container",
            ExecutionPhase::Setup => "setup",
            ExecutionPhase::Executing => "executing",
            ExecutionPhase::Cleanup => "cleanup",
            ExecutionPhase::Completed => "completed",
            ExecutionPhase::Failed => "failed",
        }
    }
}

#[derive(Error, Debug)]
pub enum ExecutionError {
    #[error("Docker not available")]
    DockerUnavailable,
    #[error("Docker error: {0}")]
    Docker(#[from] super::DockerError),
    #[error("Git error: {0}")]
    Git(#[from] super::GitError),
    #[error("Repository not found: {0}")]
    RepositoryNotFound(String),
    #[error("Task not found: {0}")]
    TaskNotFound(String),
    #[error("Database error: {0}")]
    Database(String),
    #[error("Execution timeout after {0} seconds")]
    Timeout(u64),
    #[error("Claude Code execution failed: {0}")]
    ClaudeCodeFailed(String),
    #[error("API key not configured")]
    MissingApiKey,
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub type ExecutionResult<T> = Result<T, ExecutionError>;

/// Event payload for execution log streaming
#[derive(Clone, serde::Serialize)]
pub struct ExecutionLogEvent {
    pub task_id: String,
    pub session_id: String,
    pub log: ExecutionLog,
}

/// Event payload for task status changes
#[derive(Clone, serde::Serialize)]
pub struct TaskStatusEvent {
    pub task_id: String,
    pub old_status: String,
    pub new_status: String,
}

/// Event payload for execution progress
#[derive(Clone, serde::Serialize)]
pub struct ExecutionProgressEvent {
    pub task_id: String,
    pub session_id: String,
    pub phase: String,
    pub message: String,
}

/// Event payload for agent stream messages (Claude Code, OpenCode, etc.)
#[derive(Clone, serde::Serialize)]
pub struct AgentStreamEvent {
    pub task_id: String,
    pub session_id: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub message: AgentStreamMessage,
}

/// Legacy alias for backward compatibility
pub type ClaudeStreamEvent = AgentStreamEvent;

/// Service for orchestrating AI agent execution
/// Supports both containerized (Docker/Podman) and direct execution modes
pub struct AgentExecutionService {
    docker_service: Option<Arc<DockerService>>,
    git_service: Arc<GitService>,
    task_service: Arc<TaskService>,
    repository_service: Arc<RepositoryService>,
    repository_group_service: Option<Arc<super::RepositoryGroupService>>,
    execution_logs: Arc<RwLock<Vec<ExecutionLog>>>,
    app_handle: Option<AppHandle>,
    notification_service: Option<Arc<NotificationService>>,
    /// Whether to use container for execution
    use_container: bool,
    /// Concurrency service for managing session limits (optional, for dependent
    /// task execution)
    concurrency_service: Option<Arc<ConcurrencyService>>,
    /// Global config for concurrency limits
    global_config: Option<Arc<RwLock<crate::entities::GlobalConfig>>>,
}

impl AgentExecutionService {
    /// Creates a new AgentExecutionService with container support
    pub fn new(
        docker_service: Arc<DockerService>,
        git_service: Arc<GitService>,
        task_service: Arc<TaskService>,
        repository_service: Arc<RepositoryService>,
    ) -> Self {
        Self {
            docker_service: Some(docker_service),
            git_service,
            task_service,
            repository_service,
            repository_group_service: None,
            execution_logs: Arc::new(RwLock::new(Vec::new())),
            app_handle: None,
            notification_service: None,
            use_container: true,
            concurrency_service: None,
            global_config: None,
        }
    }

    /// Creates a new AgentExecutionService for direct (non-container) execution
    pub fn new_direct(
        git_service: Arc<GitService>,
        task_service: Arc<TaskService>,
        repository_service: Arc<RepositoryService>,
    ) -> Self {
        Self {
            docker_service: None,
            git_service,
            task_service,
            repository_service,
            repository_group_service: None,
            execution_logs: Arc::new(RwLock::new(Vec::new())),
            app_handle: None,
            notification_service: None,
            use_container: false,
            concurrency_service: None,
            global_config: None,
        }
    }

    /// Sets the repository group service
    pub fn with_repository_group_service(
        mut self,
        repository_group_service: Arc<super::RepositoryGroupService>,
    ) -> Self {
        self.repository_group_service = Some(repository_group_service);
        self
    }

    /// Sets the concurrency service for managing session limits
    pub fn with_concurrency_service(
        mut self,
        concurrency_service: Arc<ConcurrencyService>,
        global_config: Arc<RwLock<crate::entities::GlobalConfig>>,
    ) -> Self {
        self.concurrency_service = Some(concurrency_service);
        self.global_config = Some(global_config);
        self
    }

    /// Sets whether to use container for execution
    pub fn with_use_container(mut self, use_container: bool) -> Self {
        self.use_container = use_container;
        self
    }

    /// Returns whether this service is configured to use containers
    pub fn uses_container(&self) -> bool {
        self.use_container && self.docker_service.is_some()
    }

    /// Gets the primary repository from a repository group.
    /// For single-repo groups, returns the only repository.
    /// For multi-repo groups, returns the first repository.
    async fn get_primary_repository_from_group(
        &self,
        repository_group_id: &str,
    ) -> ExecutionResult<crate::entities::Repository> {
        // If we have a repository group service, use it
        if let Some(rg_service) = &self.repository_group_service {
            let group = rg_service
                .get(repository_group_id)
                .await
                .map_err(|e| ExecutionError::Database(e.to_string()))?
                .ok_or_else(|| {
                    ExecutionError::RepositoryNotFound(format!(
                        "Repository group not found: {}",
                        repository_group_id
                    ))
                })?;

            if group.repository_ids.is_empty() {
                return Err(ExecutionError::RepositoryNotFound(format!(
                    "Repository group {} has no repositories",
                    repository_group_id
                )));
            }

            let repo_id = &group.repository_ids[0];
            self.repository_service
                .get(repo_id)
                .await
                .map_err(|e| ExecutionError::Database(e.to_string()))?
                .ok_or_else(|| ExecutionError::RepositoryNotFound(repo_id.clone()))
        } else {
            // Fallback: try to find a repository via repository_group_members table
            // This path is used when repository_group_service is not set
            Err(ExecutionError::RepositoryNotFound(format!(
                "Repository group service not available to resolve group: {}",
                repository_group_id
            )))
        }
    }

    /// Gets all repositories from a repository group.
    /// Returns a list of repositories in the group.
    #[allow(dead_code)] // Will be used for multi-repo task execution
    async fn get_repositories_from_group(
        &self,
        repository_group_id: &str,
    ) -> ExecutionResult<Vec<crate::entities::Repository>> {
        let rg_service = self.repository_group_service.as_ref().ok_or_else(|| {
            ExecutionError::RepositoryNotFound(format!(
                "Repository group service not available to resolve group: {}",
                repository_group_id
            ))
        })?;

        let group = rg_service
            .get(repository_group_id)
            .await
            .map_err(|e| ExecutionError::Database(e.to_string()))?
            .ok_or_else(|| {
                ExecutionError::RepositoryNotFound(format!(
                    "Repository group not found: {}",
                    repository_group_id
                ))
            })?;

        let mut repos = Vec::new();
        for repo_id in &group.repository_ids {
            let repo = self
                .repository_service
                .get(repo_id)
                .await
                .map_err(|e| ExecutionError::Database(e.to_string()))?
                .ok_or_else(|| ExecutionError::RepositoryNotFound(repo_id.clone()))?;
            repos.push(repo);
        }

        Ok(repos)
    }

    /// Checks if auto-learning is enabled for the given repository path.
    async fn is_auto_learn_enabled(&self, repo_path: &std::path::Path) -> bool {
        // Check user's config settings
        let config_manager = crate::config::ConfigManager::new().ok();
        let global_config = config_manager
            .as_ref()
            .and_then(|cm| cm.load_global_config().ok())
            .unwrap_or_default();
        let repo_config =
            crate::config::ConfigManager::load_repository_config(repo_path).unwrap_or_default();
        repo_config.effective_auto_learn(&global_config)
    }

    /// Sets the Tauri app handle for event emission
    pub fn with_app_handle(mut self, app_handle: AppHandle) -> Self {
        self.app_handle = Some(app_handle);
        self
    }

    /// Sets the notification service
    pub fn with_notification_service(
        mut self,
        notification_service: Arc<NotificationService>,
    ) -> Self {
        self.notification_service = Some(notification_service);
        self
    }

    /// Emits an execution log event to the frontend
    fn emit_log(&self, task_id: &str, session_id: &str, log: &ExecutionLog) {
        if let Some(app) = &self.app_handle {
            let event = ExecutionLogEvent {
                task_id: task_id.to_string(),
                session_id: session_id.to_string(),
                log: log.clone(),
            };
            let _ = app.emit("execution-log", event);
        }
    }

    /// Emits a progress event to the frontend
    fn emit_progress(&self, task_id: &str, session_id: &str, phase: ExecutionPhase, message: &str) {
        if let Some(app) = &self.app_handle {
            let event = ExecutionProgressEvent {
                task_id: task_id.to_string(),
                session_id: session_id.to_string(),
                phase: phase.as_str().to_string(),
                message: message.to_string(),
            };
            let _ = app.emit("execution-progress", event);
        }
    }

    /// Emits a task status change event
    fn emit_status_change(&self, task_id: &str, old_status: &str, new_status: &str) {
        if let Some(app) = &self.app_handle {
            let event = TaskStatusEvent {
                task_id: task_id.to_string(),
                old_status: old_status.to_string(),
                new_status: new_status.to_string(),
            };
            let _ = app.emit("task-status-changed", event);
        }
    }

    /// Executes a UnitTask using the configured execution mode
    /// (container or direct)
    pub async fn execute_unit_task(
        &self,
        task: &UnitTask,
        agent_task: &AgentTask,
    ) -> ExecutionResult<()> {
        if self.uses_container() {
            self.execute_unit_task_in_container(task, agent_task).await
        } else {
            self.execute_unit_task_direct(task, agent_task).await
        }
    }

    /// Executes a UnitTask in a Docker container
    async fn execute_unit_task_in_container(
        &self,
        task: &UnitTask,
        agent_task: &AgentTask,
    ) -> ExecutionResult<()> {
        let docker_service = self
            .docker_service
            .as_ref()
            .ok_or(ExecutionError::DockerUnavailable)?;

        tracing::info!("Starting container execution for task: {}", task.id);

        // 1. Get repository info from repository group
        let repo = self
            .get_primary_repository_from_group(&task.repository_group_id)
            .await?;

        // 2. Create agent session
        let session_id = uuid::Uuid::new_v4().to_string();
        let agent_type = agent_task.ai_agent_type.unwrap_or_default();
        let session = AgentSession::new(session_id.clone(), agent_type);

        // Emit progress: starting
        self.emit_progress(
            &task.id,
            &session_id,
            ExecutionPhase::Starting,
            "Initializing execution...",
        );

        // Clear previous execution failure flag when starting new execution
        if let Err(e) = self
            .task_service
            .update_unit_task_execution_failed(&task.id, false)
            .await
        {
            tracing::warn!(
                "Failed to clear last_execution_failed for task {}: {}",
                task.id,
                e
            );
        }

        // Save session to database
        self.task_service
            .create_agent_session(&agent_task.id, &session)
            .await
            .map_err(|e| ExecutionError::Database(e.to_string()))?;

        // 3. Generate branch name
        let branch_name = task
            .branch_name
            .clone()
            .unwrap_or_else(|| format!("delidev/{}", task.id));

        // 4. Create git worktree
        // Resolve symlinks in /tmp (macOS uses /tmp -> /private/tmp, which Podman
        // doesn't resolve)
        let base_tmp = PathBuf::from("/tmp")
            .canonicalize()
            .unwrap_or_else(|_| PathBuf::from("/tmp"));
        let worktree_path = base_tmp.join(format!("delidev/worktrees/{}", task.id));
        let repo_path = PathBuf::from(&repo.local_path);

        // Emit progress: worktree
        self.emit_progress(
            &task.id,
            &session_id,
            ExecutionPhase::Worktree,
            "Creating git worktree...",
        );
        self.log_info_with_emit(
            &task.id,
            &session_id,
            format!("Creating git worktree at {:?}", worktree_path),
        )
        .await;

        // Capture the base commit hash BEFORE creating the worktree
        // This ensures we have a stable reference point for diffs even if the default
        // branch advances
        let base_commit = match self
            .git_service
            .get_branch_commit_hash(&repo_path, &repo.default_branch)
        {
            Ok(hash) => {
                self.log_info_with_emit(&task.id, &session_id, format!("Base commit: {}", hash))
                    .await;
                Some(hash)
            }
            Err(e) => {
                self.log_error_with_emit(
                    &task.id,
                    &session_id,
                    format!("Failed to get base commit hash: {}", e),
                )
                .await;
                None
            }
        };

        // Store the base commit in the database
        if let Some(ref commit_hash) = base_commit {
            if let Err(e) = self
                .task_service
                .update_unit_task_base_commit(&task.id, commit_hash)
                .await
            {
                self.log_error_with_emit(
                    &task.id,
                    &session_id,
                    format!("Failed to store base commit: {}", e),
                )
                .await;
            }
        }

        // Remove existing worktree if it exists (e.g., from a previous run)
        if worktree_path.exists() {
            self.log_info_with_emit(
                &task.id,
                &session_id,
                "Removing existing worktree from previous run...".to_string(),
            )
            .await;
            if let Err(e) = self.git_service.remove_worktree(&repo_path, &worktree_path) {
                self.log_error_with_emit(
                    &task.id,
                    &session_id,
                    format!("Failed to remove existing worktree: {}", e),
                )
                .await;
                // Continue anyway - try to create the worktree
            }
        }

        if let Err(e) = self.git_service.create_worktree(
            &repo_path,
            &worktree_path,
            &branch_name,
            &repo.default_branch,
        ) {
            self.log_error_with_emit(
                &task.id,
                &session_id,
                format!("Failed to create worktree: {}", e),
            )
            .await;
            return Err(ExecutionError::Git(e));
        }

        // Store the branch name in the database
        if let Err(e) = self
            .task_service
            .update_unit_task_branch_name(&task.id, &branch_name)
            .await
        {
            self.log_error_with_emit(
                &task.id,
                &session_id,
                format!("Failed to store branch name: {}", e),
            )
            .await;
        }

        // Create execution context
        let ctx = ExecutionContext::new(
            task.id.clone(),
            session_id.clone(),
            worktree_path.clone(),
            repo_path.clone(),
        );

        // 5. Build working directory path: /workspace/$repoName
        // Sanitize repo name to prevent path injection and handle filesystem-unsafe
        // characters
        let safe_repo_name = sanitize_repo_name(&repo.name);
        let working_dir = format!("/workspace/{}", safe_repo_name);

        // 6. Build environment variables
        let env_vars = self.build_env_vars()?;

        // 7. Get Claude config path
        let claude_config_path = dirs::home_dir()
            .map(|h| h.join(".claude"))
            .filter(|p| p.exists())
            .map(|p| p.to_string_lossy().to_string());

        // 8. Build or get Docker image
        self.emit_progress(
            &task.id,
            &session_id,
            ExecutionPhase::Setup,
            "Preparing Docker image...",
        );
        self.log_info_with_emit(
            &task.id,
            &session_id,
            "Checking for custom Dockerfile...".to_string(),
        )
        .await;

        // Create callback for streaming build logs
        let task_id_for_build = task.id.clone();
        let session_id_for_build = session_id.clone();
        let logs_for_build = self.execution_logs.clone();
        let app_handle_for_build = self.app_handle.clone();

        let docker_image = match docker_service
            .get_or_build_image(&repo_path, &task.id, move |output| {
                // Strip ANSI escape codes and trim whitespace
                let cleaned = strip_ansi_escapes::strip_str(output).trim().to_string();

                if !cleaned.is_empty() {
                    let log = ExecutionLog::new(
                        uuid::Uuid::new_v4().to_string(),
                        session_id_for_build.clone(),
                        LogLevel::Info,
                        cleaned.clone(),
                    );

                    if let Some(app) = &app_handle_for_build {
                        let event = ExecutionLogEvent {
                            task_id: task_id_for_build.clone(),
                            session_id: session_id_for_build.clone(),
                            log: log.clone(),
                        };
                        let _ = app.emit("execution-log", event);
                    }

                    if let Ok(mut logs_guard) = logs_for_build.try_write() {
                        logs_guard.push(log);
                    }
                }
            })
            .await
        {
            Ok(image) => {
                self.log_info_with_emit(&task.id, &session_id, format!("Using image: {}", image))
                    .await;
                image
            }
            Err(e) => {
                let error_msg = format!("Failed to prepare Docker image: {}", e);
                self.log_error_with_emit(&task.id, &session_id, error_msg.clone())
                    .await;
                return Err(ExecutionError::Docker(e));
            }
        };

        // 9. Create and start Docker container
        let container_name = format!("delidev-{}", task.id);

        // Remove existing container if it exists (e.g., from a previous run)
        let _ = docker_service.stop_container(&container_name).await;
        let _ = docker_service.remove_container(&container_name).await;

        // Emit progress: container
        self.emit_progress(
            &task.id,
            &session_id,
            ExecutionPhase::Container,
            "Creating Docker container...",
        );
        self.log_info_with_emit(
            &task.id,
            &session_id,
            format!("Creating Docker container: {}", container_name),
        )
        .await;

        let container_id = docker_service
            .create_agent_container_with_env(
                &container_name,
                &docker_image,
                &working_dir,
                worktree_path.to_str().unwrap(),
                env_vars,
                claude_config_path.as_deref(),
            )
            .await?;

        let _ctx = ctx.with_container_id(container_id.clone());

        docker_service.start_container(&container_id).await?;

        // Copy Claude config from read-only mount to writable HOME directory
        let _ = docker_service
            .exec_in_container(
                &container_id,
                vec!["cp", "-r", "/tmp/claude-config", "/workspace/.claude"],
            )
            .await;

        // Emit progress: executing
        let agent_name = agent_type.display_name();
        self.emit_progress(
            &task.id,
            &session_id,
            ExecutionPhase::Executing,
            &format!("Running {}...", agent_name),
        );
        self.log_info_with_emit(
            &task.id,
            &session_id,
            format!("Container started, executing {}...", agent_name),
        )
        .await;

        // 8. Build and execute agent command
        // Check auto-learning setting (requires valid license)
        let auto_learn_enabled = self.is_auto_learn_enabled(&repo_path).await;
        let execution_prompt = Self::generate_unit_task_prompt(&task.prompt, auto_learn_enabled);
        let cmd =
            self.build_agent_command(&execution_prompt, session.effective_model(), agent_type);

        let timeout_secs = 600; // 10 minutes
        let session_id_clone = session_id.clone();
        let task_id_clone = task.id.clone();
        let logs = self.execution_logs.clone();
        let app_handle = self.app_handle.clone();
        let agent_name_for_log = agent_name.to_string();
        let task_service = self.task_service.clone();

        // Line buffer for NDJSON parsing (handles partial lines across chunks)
        let line_buffer = std::sync::Mutex::new(String::new());

        let result = docker_service
            .exec_with_callback(&container_id, cmd, timeout_secs, move |output| {
                // Append to buffer and process complete lines
                let mut buffer = line_buffer.lock().unwrap();
                buffer.push_str(output);

                // Process all complete lines
                while let Some(newline_pos) = buffer.find('\n') {
                    let line = buffer[..newline_pos].trim().to_string();
                    *buffer = buffer[newline_pos + 1..].to_string();

                    if line.is_empty() {
                        continue;
                    }

                    // Try to parse as structured stream message (Claude Code or OpenCode)
                    // First try Claude Code format (has "type" field)
                    let parsed_message = if let Ok(claude_msg) =
                        serde_json::from_str::<ClaudeStreamMessage>(&line)
                    {
                        Some(AgentStreamMessage::ClaudeCode(claude_msg))
                    } else if let Ok(opencode_msg) = serde_json::from_str::<OpenCodeMessage>(&line)
                    {
                        Some(AgentStreamMessage::OpenCode(opencode_msg))
                    } else {
                        None
                    };

                    match parsed_message {
                        Some(message) => {
                            // Save stream message to database
                            let task_service_clone = task_service.clone();
                            let session_id_for_save = session_id_clone.clone();
                            let message_clone = message.clone();
                            tokio::spawn(async move {
                                if let Err(e) = task_service_clone
                                    .save_stream_message(&session_id_for_save, &message_clone)
                                    .await
                                {
                                    tracing::error!("Failed to save stream message: {}", e);
                                }
                            });

                            // Emit structured stream event
                            if let Some(app) = &app_handle {
                                let event = AgentStreamEvent {
                                    task_id: task_id_clone.clone(),
                                    session_id: session_id_clone.clone(),
                                    timestamp: chrono::Utc::now(),
                                    message,
                                };
                                let _ = app.emit("claude-stream", event);
                            }
                        }
                        None => {
                            // Fallback: emit as raw log
                            let log = ExecutionLog::new(
                                uuid::Uuid::new_v4().to_string(),
                                session_id_clone.clone(),
                                LogLevel::Info,
                                line.clone(),
                            );

                            if let Some(app) = &app_handle {
                                let event = ExecutionLogEvent {
                                    task_id: task_id_clone.clone(),
                                    session_id: session_id_clone.clone(),
                                    log: log.clone(),
                                };
                                let _ = app.emit("execution-log", event);
                            }

                            if let Ok(mut logs_guard) = logs.try_write() {
                                logs_guard.push(log);
                            }
                        }
                    }
                }
            })
            .await;

        // 9. Handle execution result
        match result {
            Ok(exec_result) => {
                if exec_result.exit_code == Some(0) {
                    self.log_info_with_emit(
                        &task.id,
                        &session_id,
                        format!("{} execution completed successfully", agent_name_for_log),
                    )
                    .await;

                    // Emit progress: completed
                    self.emit_progress(
                        &task.id,
                        &session_id,
                        ExecutionPhase::Completed,
                        "Execution completed successfully",
                    );

                    // Capture the end commit hash after successful execution
                    // This allows accurate diff display showing only this task's changes
                    let end_commit = match self.git_service.get_worktree_head_commit(&worktree_path)
                    {
                        Ok(end_hash) => {
                            self.log_info_with_emit(
                                &task.id,
                                &session_id,
                                format!("End commit: {}", end_hash),
                            )
                            .await;
                            if let Err(e) = self
                                .task_service
                                .update_unit_task_end_commit(&task.id, &end_hash)
                                .await
                            {
                                self.log_error_with_emit(
                                    &task.id,
                                    &session_id,
                                    format!("Failed to store end commit: {}", e),
                                )
                                .await;
                            }
                            Some(end_hash)
                        }
                        Err(e) => {
                            self.log_error_with_emit(
                                &task.id,
                                &session_id,
                                format!("Failed to get end commit hash: {}", e),
                            )
                            .await;
                            None
                        }
                    };

                    // Check if there are any committed changes (diff) between base_commit and
                    // end_commit
                    let has_committed_diff = match (&base_commit, &end_commit) {
                        (Some(base), Some(end)) => {
                            match self.git_service.get_diff_between_commits(
                                &worktree_path,
                                base,
                                end,
                            ) {
                                Ok(diff) => !diff.trim().is_empty(),
                                Err(e) => {
                                    self.log_error_with_emit(
                                        &task.id,
                                        &session_id,
                                        format!("Failed to get diff between commits: {}", e),
                                    )
                                    .await;
                                    // Fallback to get_diff_from_base
                                    match self
                                        .git_service
                                        .get_diff_from_base(&worktree_path, &repo.default_branch)
                                    {
                                        Ok(diff) => !diff.trim().is_empty(),
                                        Err(_) => false,
                                    }
                                }
                            }
                        }
                        _ => {
                            // If we don't have both commits, fallback to get_diff_from_base
                            match self
                                .git_service
                                .get_diff_from_base(&worktree_path, &repo.default_branch)
                            {
                                Ok(diff) => !diff.trim().is_empty(),
                                Err(_) => false,
                            }
                        }
                    };

                    // Also check for uncommitted changes in the worktree
                    let has_uncommitted_diff = match self.git_service.get_diff(&worktree_path) {
                        Ok(diff) => !diff.trim().is_empty(),
                        Err(e) => {
                            self.log_error_with_emit(
                                &task.id,
                                &session_id,
                                format!("Failed to get uncommitted diff: {}", e),
                            )
                            .await;
                            false
                        }
                    };

                    let has_diff = has_committed_diff || has_uncommitted_diff;

                    if has_diff {
                        // Has changes - go to InReview for user to review
                        self.emit_status_change(&task.id, "in_progress", "in_review");

                        self.task_service
                            .update_unit_task_status(&task.id, UnitTaskStatus::InReview)
                            .await
                            .map_err(|e| ExecutionError::Database(e.to_string()))?;

                        // Send notification for status change
                        if let Some(notification_service) = &self.notification_service {
                            notification_service.notify_task_status_change(
                                &task.id,
                                &task.title,
                                UnitTaskStatus::InProgress,
                                UnitTaskStatus::InReview,
                            );
                        }
                    } else {
                        // No changes - skip review and go directly to Done
                        self.log_info_with_emit(
                            &task.id,
                            &session_id,
                            "No changes detected, marking task as done".to_string(),
                        )
                        .await;

                        self.emit_status_change(&task.id, "in_progress", "done");

                        self.task_service
                            .update_unit_task_status(&task.id, UnitTaskStatus::Done)
                            .await
                            .map_err(|e| ExecutionError::Database(e.to_string()))?;

                        // Check if this completes a composite task
                        if let Ok(Some(composite_task_id)) = self
                            .task_service
                            .check_and_complete_composite_task(&task.id)
                            .await
                        {
                            self.emit_status_change(&composite_task_id, "in_progress", "done");
                        }

                        // Send notification for status change
                        if let Some(notification_service) = &self.notification_service {
                            notification_service.notify_task_status_change(
                                &task.id,
                                &task.title,
                                UnitTaskStatus::InProgress,
                                UnitTaskStatus::Done,
                            );
                        }

                        // Cleanup worktree since there are no changes to review
                        if worktree_path.exists() {
                            if let Err(e) =
                                self.git_service.remove_worktree(&repo_path, &worktree_path)
                            {
                                tracing::warn!(
                                    "Failed to cleanup worktree for task {}: {}",
                                    task.id,
                                    e
                                );
                            } else {
                                tracing::info!(
                                    "Cleaned up worktree for task {} (no changes)",
                                    task.id
                                );
                            }
                        }

                        // Cleanup Docker container if applicable
                        if let Some(docker_service) = &self.docker_service {
                            let container_name = format!("delidev-{}", task.id);
                            let _ = docker_service.stop_container(&container_name).await;
                            let _ = docker_service.remove_container(&container_name).await;
                        }
                    }
                } else {
                    let error_msg = format!(
                        "{} exited with code {:?}: {}",
                        agent_name_for_log, exec_result.exit_code, exec_result.stderr
                    );
                    self.log_error_with_emit(&task.id, &session_id, error_msg.clone())
                        .await;

                    // Emit progress: failed
                    self.emit_progress(&task.id, &session_id, ExecutionPhase::Failed, &error_msg);

                    // Mark task as having failed execution
                    if let Err(e) = self
                        .task_service
                        .update_unit_task_execution_failed(&task.id, true)
                        .await
                    {
                        tracing::error!(
                            "Failed to update last_execution_failed for task {}: {}",
                            task.id,
                            e
                        );
                    }

                    // Send notification for execution failure
                    if let Some(notification_service) = &self.notification_service {
                        notification_service.notify_execution_error(
                            &task.id,
                            &task.title,
                            &error_msg,
                        );
                    }

                    // Keep status as InProgress for retry
                    return Err(ExecutionError::ClaudeCodeFailed(error_msg));
                }
            }
            Err(e) => {
                self.log_error_with_emit(&task.id, &session_id, format!("Execution failed: {}", e))
                    .await;

                // Emit progress: failed
                self.emit_progress(
                    &task.id,
                    &session_id,
                    ExecutionPhase::Failed,
                    &format!("Execution failed: {}", e),
                );

                // Mark task as having failed execution
                if let Err(update_err) = self
                    .task_service
                    .update_unit_task_execution_failed(&task.id, true)
                    .await
                {
                    tracing::error!(
                        "Failed to update last_execution_failed for task {}: {}",
                        task.id,
                        update_err
                    );
                }

                // Send notification for execution failure
                if let Some(notification_service) = &self.notification_service {
                    notification_service.notify_execution_error(
                        &task.id,
                        &task.title,
                        &e.to_string(),
                    );
                }

                return Err(e.into());
            }
        }

        // 10. Cleanup
        self.emit_progress(
            &task.id,
            &session_id,
            ExecutionPhase::Cleanup,
            "Cleaning up...",
        );
        self.log_info_with_emit(&task.id, &session_id, "Cleaning up...".to_string())
            .await;

        // Stop and remove container
        let _ = docker_service.stop_container(&container_id).await;
        let _ = docker_service.remove_container(&container_id).await;

        // Note: We don't remove the worktree here because we want to keep the changes
        // for review. The worktree will be cleaned up when the task is
        // approved/rejected.

        // Extract and save token usage from the session
        let model = agent_task.ai_agent_model.clone();
        if let Err(e) = self
            .task_service
            .extract_and_save_session_usage(&session_id, model)
            .await
        {
            tracing::warn!(
                "Failed to extract session usage for task {}: {}",
                task.id,
                e
            );
        }

        tracing::info!("Task {} execution completed", task.id);
        Ok(())
    }

    /// Executes a UnitTask directly on the host without containerization
    async fn execute_unit_task_direct(
        &self,
        task: &UnitTask,
        agent_task: &AgentTask,
    ) -> ExecutionResult<()> {
        tracing::info!("Starting direct execution for task: {}", task.id);

        // 1. Get repository info from repository group
        let repo = self
            .get_primary_repository_from_group(&task.repository_group_id)
            .await?;

        // 2. Create agent session
        let session_id = uuid::Uuid::new_v4().to_string();
        let agent_type = agent_task.ai_agent_type.unwrap_or_default();
        let session = AgentSession::new(session_id.clone(), agent_type);

        // Emit progress: starting
        self.emit_progress(
            &task.id,
            &session_id,
            ExecutionPhase::Starting,
            "Initializing direct execution...",
        );

        // Clear previous execution failure flag when starting new execution
        if let Err(e) = self
            .task_service
            .update_unit_task_execution_failed(&task.id, false)
            .await
        {
            tracing::warn!(
                "Failed to clear last_execution_failed for task {}: {}",
                task.id,
                e
            );
        }

        // Save session to database
        self.task_service
            .create_agent_session(&agent_task.id, &session)
            .await
            .map_err(|e| ExecutionError::Database(e.to_string()))?;

        // 3. Generate branch name
        let branch_name = task
            .branch_name
            .clone()
            .unwrap_or_else(|| format!("delidev/{}", task.id));

        // 4. Create git worktree
        let base_tmp = PathBuf::from("/tmp")
            .canonicalize()
            .unwrap_or_else(|_| PathBuf::from("/tmp"));
        let worktree_path = base_tmp.join(format!("delidev/worktrees/{}", task.id));
        let repo_path = PathBuf::from(&repo.local_path);

        // Emit progress: worktree
        self.emit_progress(
            &task.id,
            &session_id,
            ExecutionPhase::Worktree,
            "Creating git worktree...",
        );
        self.log_info_with_emit(
            &task.id,
            &session_id,
            format!("Creating git worktree at {:?}", worktree_path),
        )
        .await;

        // Capture the base commit hash BEFORE creating the worktree
        let base_commit = match self
            .git_service
            .get_branch_commit_hash(&repo_path, &repo.default_branch)
        {
            Ok(hash) => {
                self.log_info_with_emit(&task.id, &session_id, format!("Base commit: {}", hash))
                    .await;
                Some(hash)
            }
            Err(e) => {
                self.log_error_with_emit(
                    &task.id,
                    &session_id,
                    format!("Failed to get base commit hash: {}", e),
                )
                .await;
                None
            }
        };

        // Store the base commit in the database
        if let Some(ref commit_hash) = base_commit {
            if let Err(e) = self
                .task_service
                .update_unit_task_base_commit(&task.id, commit_hash)
                .await
            {
                self.log_error_with_emit(
                    &task.id,
                    &session_id,
                    format!("Failed to store base commit: {}", e),
                )
                .await;
            }
        }

        // Remove existing worktree if it exists
        if worktree_path.exists() {
            self.log_info_with_emit(
                &task.id,
                &session_id,
                "Removing existing worktree from previous run...".to_string(),
            )
            .await;
            if let Err(e) = self.git_service.remove_worktree(&repo_path, &worktree_path) {
                self.log_error_with_emit(
                    &task.id,
                    &session_id,
                    format!("Failed to remove existing worktree: {}", e),
                )
                .await;
            }
        }

        if let Err(e) = self.git_service.create_worktree(
            &repo_path,
            &worktree_path,
            &branch_name,
            &repo.default_branch,
        ) {
            self.log_error_with_emit(
                &task.id,
                &session_id,
                format!("Failed to create worktree: {}", e),
            )
            .await;
            return Err(ExecutionError::Git(e));
        }

        // Store the branch name in the database
        if let Err(e) = self
            .task_service
            .update_unit_task_branch_name(&task.id, &branch_name)
            .await
        {
            self.log_error_with_emit(
                &task.id,
                &session_id,
                format!("Failed to store branch name: {}", e),
            )
            .await;
        }

        // Emit progress: executing
        let agent_name = agent_type.display_name();
        self.emit_progress(
            &task.id,
            &session_id,
            ExecutionPhase::Executing,
            &format!("Running {} directly...", agent_name),
        );
        self.log_info_with_emit(
            &task.id,
            &session_id,
            format!("Executing {} directly on host...", agent_name),
        )
        .await;

        // Build and execute agent command directly
        // Check auto-learning setting (requires valid license)
        let auto_learn_enabled = self.is_auto_learn_enabled(&repo_path).await;
        let execution_prompt = Self::generate_unit_task_prompt(&task.prompt, auto_learn_enabled);
        let cmd =
            self.build_agent_command(&execution_prompt, session.effective_model(), agent_type);

        let session_id_clone = session_id.clone();
        let task_id_clone = task.id.clone();
        let logs = self.execution_logs.clone();
        let app_handle = self.app_handle.clone();
        let agent_name_for_log = agent_name.to_string();
        let task_service = self.task_service.clone();

        // Execute command directly
        let result = self
            .execute_command_direct(&cmd, &worktree_path, move |output| {
                // Try to parse as structured stream message
                let parsed_message = if let Ok(claude_msg) =
                    serde_json::from_str::<ClaudeStreamMessage>(output)
                {
                    Some(AgentStreamMessage::ClaudeCode(claude_msg))
                } else if let Ok(opencode_msg) = serde_json::from_str::<OpenCodeMessage>(output) {
                    Some(AgentStreamMessage::OpenCode(opencode_msg))
                } else {
                    None
                };

                match parsed_message {
                    Some(message) => {
                        // Save stream message to database
                        let task_service_clone = task_service.clone();
                        let session_id_for_save = session_id_clone.clone();
                        let message_clone = message.clone();
                        tokio::spawn(async move {
                            if let Err(e) = task_service_clone
                                .save_stream_message(&session_id_for_save, &message_clone)
                                .await
                            {
                                tracing::error!("Failed to save stream message: {}", e);
                            }
                        });

                        // Emit structured stream event
                        if let Some(app) = &app_handle {
                            let event = AgentStreamEvent {
                                task_id: task_id_clone.clone(),
                                session_id: session_id_clone.clone(),
                                timestamp: chrono::Utc::now(),
                                message,
                            };
                            let _ = app.emit("claude-stream", event);
                        }
                    }
                    None => {
                        // Fallback: emit as raw log if not empty
                        let trimmed = output.trim();
                        if !trimmed.is_empty() {
                            let log = ExecutionLog::new(
                                uuid::Uuid::new_v4().to_string(),
                                session_id_clone.clone(),
                                LogLevel::Info,
                                trimmed.to_string(),
                            );

                            if let Some(app) = &app_handle {
                                let event = ExecutionLogEvent {
                                    task_id: task_id_clone.clone(),
                                    session_id: session_id_clone.clone(),
                                    log: log.clone(),
                                };
                                let _ = app.emit("execution-log", event);
                            }

                            if let Ok(mut logs_guard) = logs.try_write() {
                                logs_guard.push(log);
                            }
                        }
                    }
                }
            })
            .await;

        // Handle execution result
        match result {
            Ok(exit_code) => {
                if exit_code == 0 {
                    self.log_info_with_emit(
                        &task.id,
                        &session_id,
                        format!("{} execution completed successfully", agent_name_for_log),
                    )
                    .await;

                    self.emit_progress(
                        &task.id,
                        &session_id,
                        ExecutionPhase::Completed,
                        "Execution completed successfully",
                    );

                    // Capture the end commit hash
                    let end_commit = match self.git_service.get_worktree_head_commit(&worktree_path)
                    {
                        Ok(end_hash) => {
                            self.log_info_with_emit(
                                &task.id,
                                &session_id,
                                format!("End commit: {}", end_hash),
                            )
                            .await;
                            if let Err(e) = self
                                .task_service
                                .update_unit_task_end_commit(&task.id, &end_hash)
                                .await
                            {
                                self.log_error_with_emit(
                                    &task.id,
                                    &session_id,
                                    format!("Failed to store end commit: {}", e),
                                )
                                .await;
                            }
                            Some(end_hash)
                        }
                        Err(e) => {
                            self.log_error_with_emit(
                                &task.id,
                                &session_id,
                                format!("Failed to get end commit hash: {}", e),
                            )
                            .await;
                            None
                        }
                    };

                    // Check if there are any committed changes
                    let has_committed_diff = match (&base_commit, &end_commit) {
                        (Some(base), Some(end)) => {
                            match self.git_service.get_diff_between_commits(
                                &worktree_path,
                                base,
                                end,
                            ) {
                                Ok(diff) => !diff.trim().is_empty(),
                                Err(_) => {
                                    match self
                                        .git_service
                                        .get_diff_from_base(&worktree_path, &repo.default_branch)
                                    {
                                        Ok(diff) => !diff.trim().is_empty(),
                                        Err(_) => false,
                                    }
                                }
                            }
                        }
                        _ => match self
                            .git_service
                            .get_diff_from_base(&worktree_path, &repo.default_branch)
                        {
                            Ok(diff) => !diff.trim().is_empty(),
                            Err(_) => false,
                        },
                    };

                    // Also check for uncommitted changes in the worktree
                    let has_uncommitted_diff = match self.git_service.get_diff(&worktree_path) {
                        Ok(diff) => !diff.trim().is_empty(),
                        Err(e) => {
                            self.log_error_with_emit(
                                &task.id,
                                &session_id,
                                format!("Failed to get uncommitted diff: {}", e),
                            )
                            .await;
                            false
                        }
                    };

                    let has_diff = has_committed_diff || has_uncommitted_diff;

                    if has_diff {
                        self.emit_status_change(&task.id, "in_progress", "in_review");
                        self.task_service
                            .update_unit_task_status(&task.id, UnitTaskStatus::InReview)
                            .await
                            .map_err(|e| ExecutionError::Database(e.to_string()))?;

                        if let Some(notification_service) = &self.notification_service {
                            notification_service.notify_task_status_change(
                                &task.id,
                                &task.title,
                                UnitTaskStatus::InProgress,
                                UnitTaskStatus::InReview,
                            );
                        }
                    } else {
                        self.log_info_with_emit(
                            &task.id,
                            &session_id,
                            "No changes detected, marking task as done".to_string(),
                        )
                        .await;

                        self.emit_status_change(&task.id, "in_progress", "done");
                        self.task_service
                            .update_unit_task_status(&task.id, UnitTaskStatus::Done)
                            .await
                            .map_err(|e| ExecutionError::Database(e.to_string()))?;

                        // Check if this completes a composite task
                        if let Ok(Some(composite_task_id)) = self
                            .task_service
                            .check_and_complete_composite_task(&task.id)
                            .await
                        {
                            self.emit_status_change(&composite_task_id, "in_progress", "done");
                        }

                        if let Some(notification_service) = &self.notification_service {
                            notification_service.notify_task_status_change(
                                &task.id,
                                &task.title,
                                UnitTaskStatus::InProgress,
                                UnitTaskStatus::Done,
                            );
                        }

                        // Cleanup worktree since there are no changes to review
                        if worktree_path.exists() {
                            if let Err(e) =
                                self.git_service.remove_worktree(&repo_path, &worktree_path)
                            {
                                tracing::warn!(
                                    "Failed to cleanup worktree for task {}: {}",
                                    task.id,
                                    e
                                );
                            } else {
                                tracing::info!(
                                    "Cleaned up worktree for task {} (no changes)",
                                    task.id
                                );
                            }
                        }
                    }
                } else {
                    let error_msg =
                        format!("{} exited with code {}", agent_name_for_log, exit_code);
                    self.log_error_with_emit(&task.id, &session_id, error_msg.clone())
                        .await;
                    self.emit_progress(&task.id, &session_id, ExecutionPhase::Failed, &error_msg);

                    // Mark task as having failed execution
                    if let Err(e) = self
                        .task_service
                        .update_unit_task_execution_failed(&task.id, true)
                        .await
                    {
                        tracing::error!(
                            "Failed to update last_execution_failed for task {}: {}",
                            task.id,
                            e
                        );
                    }

                    if let Some(notification_service) = &self.notification_service {
                        notification_service.notify_execution_error(
                            &task.id,
                            &task.title,
                            &error_msg,
                        );
                    }

                    return Err(ExecutionError::ClaudeCodeFailed(error_msg));
                }
            }
            Err(e) => {
                let error_msg = format!("Execution failed: {}", e);
                self.log_error_with_emit(&task.id, &session_id, error_msg.clone())
                    .await;
                self.emit_progress(&task.id, &session_id, ExecutionPhase::Failed, &error_msg);

                // Mark task as having failed execution
                if let Err(update_err) = self
                    .task_service
                    .update_unit_task_execution_failed(&task.id, true)
                    .await
                {
                    tracing::error!(
                        "Failed to update last_execution_failed for task {}: {}",
                        task.id,
                        update_err
                    );
                }

                if let Some(notification_service) = &self.notification_service {
                    notification_service.notify_execution_error(
                        &task.id,
                        &task.title,
                        &e.to_string(),
                    );
                }

                return Err(e);
            }
        }

        // Extract and save token usage from the session
        let model = agent_task.ai_agent_model.clone();
        if let Err(e) = self
            .task_service
            .extract_and_save_session_usage(&session_id, model)
            .await
        {
            tracing::warn!(
                "Failed to extract session usage for task {}: {}",
                task.id,
                e
            );
        }

        tracing::info!("Task {} direct execution completed", task.id);
        Ok(())
    }

    /// Executes a command directly on the host
    async fn execute_command_direct<F>(
        &self,
        cmd: &[String],
        working_dir: &PathBuf,
        mut on_line: F,
    ) -> ExecutionResult<i32>
    where
        F: FnMut(&str) + Send,
    {
        use std::process::Stdio;

        use tokio::{
            io::{AsyncBufReadExt, BufReader},
            process::Command,
        };

        if cmd.is_empty() {
            return Err(ExecutionError::ClaudeCodeFailed(
                "Empty command".to_string(),
            ));
        }

        let mut process = Command::new(&cmd[0])
            .args(&cmd[1..])
            .current_dir(working_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        let stdout = process.stdout.take().unwrap();
        let stderr = process.stderr.take().unwrap();

        let mut stdout_reader = BufReader::new(stdout).lines();
        let mut stderr_reader = BufReader::new(stderr).lines();

        // Read stdout and stderr concurrently
        loop {
            tokio::select! {
                line = stdout_reader.next_line() => {
                    match line {
                        Ok(Some(line)) => on_line(&line),
                        Ok(None) => break,
                        Err(e) => {
                            tracing::warn!("Error reading stdout: {}", e);
                            break;
                        }
                    }
                }
                line = stderr_reader.next_line() => {
                    match line {
                        Ok(Some(line)) => on_line(&line),
                        Ok(None) => {},
                        Err(e) => {
                            tracing::warn!("Error reading stderr: {}", e);
                        }
                    }
                }
            }
        }

        // Wait for process to complete
        let status = process.wait().await?;
        Ok(status.code().unwrap_or(-1))
    }

    /// Generates the execution prompt for a unit task.
    /// This wraps the user's prompt with system instructions.
    /// If the prompt contains feedback from review and auto-learning is
    /// enabled, additional learning instructions are added.
    fn generate_unit_task_prompt(user_prompt: &str, auto_learn_enabled: bool) -> String {
        let has_feedback = user_prompt.contains("## Feedback from review");

        let learning_instructions = if has_feedback && auto_learn_enabled {
            r#"

## Auto-Learning from Feedback
The feedback above may represent a recurring pattern or preference that should be documented for future AI agents.

After addressing the feedback:
1. Consider whether this feedback represents a general guideline that should be added to `AGENTS.md` or `CLAUDE.md`.
2. If the feedback points to:
   - Code style preferences → Add to the appropriate `AGENTS.md` file
   - Architecture patterns or conventions → Add to `AGENTS.md`
   - AI-specific instructions or behaviors → Add to `CLAUDE.md`
3. Only add truly generalizable guidelines, not task-specific corrections.
4. Place the new rule in the most appropriate existing section, or create a new section if needed.
5. Keep the added rules concise and actionable."#
        } else {
            ""
        };

        format!(
            r#"You are an AI coding agent. Your task is to complete the following request.

## User Request
{user_prompt}

## Instructions
1. Analyze the request and implement the necessary changes.
2. After completing your work, commit your changes using git.
3. Write clear, descriptive commit messages that explain what was changed and why.
4. Ensure all changes are committed before finishing the task.

## Important
- Always commit your work before finishing the task.
- If you make multiple logical changes, consider making multiple commits.
- Follow the repository's existing code style and conventions.{learning_instructions}"#,
            user_prompt = user_prompt,
            learning_instructions = learning_instructions
        )
    }

    /// Builds the agent command based on agent type
    fn build_agent_command(
        &self,
        prompt: &str,
        model: &str,
        agent_type: AIAgentType,
    ) -> Vec<String> {
        match agent_type {
            AIAgentType::ClaudeCode => vec![
                "npx".to_string(),
                "-y".to_string(),
                "@anthropic-ai/claude-code".to_string(),
                "-p".to_string(),
                "--verbose".to_string(),
                "--output-format".to_string(),
                "stream-json".to_string(),
                "--dangerously-skip-permissions".to_string(),
                "--model".to_string(),
                model.to_string(),
                prompt.to_string(),
            ],
            AIAgentType::OpenCode => vec![
                "npx".to_string(),
                "-y".to_string(),
                "opencode-ai".to_string(),
                "run".to_string(),
                "-c".to_string(),
                prompt.to_string(),
                "--format".to_string(),
                "json".to_string(),
                "--model".to_string(),
                model.to_string(),
            ],
            AIAgentType::GeminiCli => vec![
                "npx".to_string(),
                "-y".to_string(),
                "@google/gemini-cli".to_string(),
                "-p".to_string(),
                prompt.to_string(),
                "-m".to_string(),
                model.to_string(),
                "--yolo".to_string(),
            ],
            AIAgentType::CodexCli => vec![
                "npx".to_string(),
                "-y".to_string(),
                "@openai/codex".to_string(),
                "exec".to_string(),
                "--full-auto".to_string(),
                "-m".to_string(),
                model.to_string(),
                prompt.to_string(),
            ],
            AIAgentType::Aider => vec![
                "aider".to_string(),
                "--yes-always".to_string(),
                "--no-git".to_string(),
                "--model".to_string(),
                model.to_string(),
                "--message".to_string(),
                prompt.to_string(),
            ],
            AIAgentType::Amp => vec![
                "npx".to_string(),
                "-y".to_string(),
                "@sourcegraph/amp".to_string(),
                "-x".to_string(),
                "--dangerously-allow-all".to_string(),
                prompt.to_string(),
            ],
        }
    }

    /// Gets Claude Code OAuth token from system credential store
    /// - macOS: Keychain
    /// - Linux: ~/.claude/.credentials.json
    fn get_claude_oauth_token() -> Option<String> {
        #[cfg(target_os = "macos")]
        {
            let output = std::process::Command::new("security")
                .args([
                    "find-generic-password",
                    "-s",
                    "Claude Code-credentials",
                    "-w",
                ])
                .output()
                .ok()?;

            if output.status.success() {
                let credentials_json = String::from_utf8(output.stdout).ok()?;
                let trimmed = credentials_json.trim();
                if !trimmed.is_empty() {
                    // Parse JSON and extract accessToken from
                    // {"claudeAiOauth":{"accessToken":"..."}}
                    let json: serde_json::Value = serde_json::from_str(trimmed).ok()?;
                    return json
                        .get("claudeAiOauth")
                        .and_then(|v| v.get("accessToken"))
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                }
            }
            None
        }

        #[cfg(target_os = "linux")]
        {
            let home = std::env::var("HOME").ok()?;
            let creds_path = format!("{}/.claude/.credentials.json", home);
            let content = std::fs::read_to_string(&creds_path).ok()?;
            let json: serde_json::Value = serde_json::from_str(&content).ok()?;
            json.get("accessToken")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        }

        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        {
            None
        }
    }

    /// Builds environment variables for the container
    fn build_env_vars(&self) -> ExecutionResult<Vec<String>> {
        let mut vars = Vec::new();

        // ANTHROPIC_API_KEY from environment (priority 1)
        if let Ok(key) = std::env::var("ANTHROPIC_API_KEY") {
            vars.push(format!("ANTHROPIC_API_KEY={}", key));
        } else if let Some(token) = Self::get_claude_oauth_token() {
            // OAuth token from system credential store (priority 2)
            // Must use CLAUDE_CODE_OAUTH_TOKEN for OAuth tokens
            vars.push(format!("CLAUDE_CODE_OAUTH_TOKEN={}", token));
        }

        // OPENAI_API_KEY for OpenCode and Codex CLI
        if let Ok(key) = std::env::var("OPENAI_API_KEY") {
            vars.push(format!("OPENAI_API_KEY={}", key));
        }

        // GEMINI_API_KEY for Gemini CLI
        if let Ok(key) = std::env::var("GEMINI_API_KEY") {
            vars.push(format!("GEMINI_API_KEY={}", key));
        }

        // GOOGLE_API_KEY for Gemini CLI (alternative)
        if let Ok(key) = std::env::var("GOOGLE_API_KEY") {
            vars.push(format!("GOOGLE_API_KEY={}", key));
        }

        // HOME directory (use /workspace for writable access)
        vars.push("HOME=/workspace".to_string());

        // npm cache directory (avoid permission issues)
        vars.push("NPM_CONFIG_CACHE=/tmp/.npm".to_string());

        // Path for executables (base paths, additional paths added separately)
        vars.push("PATH=/usr/local/bin:/usr/bin:/bin".to_string());

        Ok(vars)
    }

    /// Logs an info message and emits event
    async fn log_info_with_emit(&self, task_id: &str, session_id: &str, message: String) {
        tracing::info!("[{}] {}", session_id, message);
        let log = ExecutionLog::info(session_id.to_string(), message);
        self.emit_log(task_id, session_id, &log);
        self.execution_logs.write().await.push(log.clone());

        // Persist to database
        if let Err(e) = self.task_service.save_execution_log(&log).await {
            tracing::error!("Failed to save execution log to database: {}", e);
        }
    }

    /// Logs an error message and emits event
    async fn log_error_with_emit(&self, task_id: &str, session_id: &str, message: String) {
        tracing::error!("[{}] {}", session_id, message);
        let log = ExecutionLog::error(session_id.to_string(), message);
        self.emit_log(task_id, session_id, &log);
        self.execution_logs.write().await.push(log.clone());

        // Persist to database
        if let Err(e) = self.task_service.save_execution_log(&log).await {
            tracing::error!("Failed to save execution log to database: {}", e);
        }
    }

    /// Gets execution logs for a session
    pub async fn get_logs(&self, session_id: &str) -> Vec<ExecutionLog> {
        self.execution_logs
            .read()
            .await
            .iter()
            .filter(|log| log.session_id == session_id)
            .cloned()
            .collect()
    }

    /// Gets all execution logs
    pub async fn get_all_logs(&self) -> Vec<ExecutionLog> {
        self.execution_logs.read().await.clone()
    }

    /// Clears logs for a session
    pub async fn clear_logs(&self, session_id: &str) {
        let mut logs = self.execution_logs.write().await;
        logs.retain(|log| log.session_id != session_id);
    }

    /// Cleans up resources for a completed/rejected task
    pub async fn cleanup_task(&self, task: &UnitTask) -> ExecutionResult<()> {
        // Get repository from repository group
        let repo = self
            .get_primary_repository_from_group(&task.repository_group_id)
            .await?;

        let repo_path = PathBuf::from(&repo.local_path);
        // Resolve symlinks in /tmp (macOS uses /tmp -> /private/tmp, which Podman
        // doesn't resolve)
        let base_tmp = PathBuf::from("/tmp")
            .canonicalize()
            .unwrap_or_else(|_| PathBuf::from("/tmp"));
        let worktree_path = base_tmp.join(format!("delidev/worktrees/{}", task.id));

        // Remove worktree if it exists
        if worktree_path.exists() {
            self.git_service
                .remove_worktree(&repo_path, &worktree_path)?;
        }

        // Remove any leftover container (only if docker service is available)
        if let Some(docker_service) = &self.docker_service {
            let container_name = format!("delidev-{}", task.id);
            // Ignore errors if container doesn't exist
            let _ = docker_service.stop_container(&container_name).await;
            let _ = docker_service.remove_container(&container_name).await;
        }

        Ok(())
    }

    /// Executes an AgentTask in an existing worktree directory.
    /// This is used for tasks that need to run in an already-prepared
    /// environment, such as PR creation where the worktree already exists.
    ///
    /// Unlike `execute_unit_task`, this method:
    /// - Does NOT create a new worktree (expects it to already exist)
    /// - Does NOT update UnitTask status
    /// - Returns the execution result for the caller to handle
    pub async fn execute_agent_task(
        &self,
        agent_task: &AgentTask,
        worktree_path: &PathBuf,
        prompt: &str,
        task_id: &str,
    ) -> ExecutionResult<AgentTaskExecutionResult> {
        tracing::info!(
            "Executing agent task {} in worktree {:?}",
            agent_task.id,
            worktree_path
        );

        // Verify worktree exists
        if !worktree_path.exists() {
            return Err(ExecutionError::ClaudeCodeFailed(format!(
                "Worktree does not exist: {:?}",
                worktree_path
            )));
        }

        // Create agent session
        let session_id = uuid::Uuid::new_v4().to_string();
        let agent_type = agent_task.ai_agent_type.unwrap_or_default();
        let session = AgentSession::new(session_id.clone(), agent_type);

        // Emit progress: starting
        self.emit_progress(
            task_id,
            &session_id,
            ExecutionPhase::Starting,
            "Initializing agent task execution...",
        );

        // Save session to database
        self.task_service
            .create_agent_session(&agent_task.id, &session)
            .await
            .map_err(|e| ExecutionError::Database(e.to_string()))?;

        // Build and execute agent command
        let agent_name = agent_type.display_name();
        self.emit_progress(
            task_id,
            &session_id,
            ExecutionPhase::Executing,
            &format!("Running {}...", agent_name),
        );
        self.log_info_with_emit(
            task_id,
            &session_id,
            format!("Executing {} for agent task...", agent_name),
        )
        .await;

        let cmd = self.build_agent_command(prompt, session.effective_model(), agent_type);

        // Execute based on configured mode
        let result = if self.uses_container() {
            self.execute_agent_task_in_container(
                agent_task,
                worktree_path,
                &cmd,
                task_id,
                &session_id,
            )
            .await
        } else {
            self.execute_agent_task_direct(worktree_path, &cmd, task_id, &session_id)
                .await
        };

        match result {
            Ok(output) => {
                self.log_info_with_emit(
                    task_id,
                    &session_id,
                    format!("{} execution completed successfully", agent_name),
                )
                .await;

                self.emit_progress(
                    task_id,
                    &session_id,
                    ExecutionPhase::Completed,
                    "Agent task completed successfully",
                );

                Ok(AgentTaskExecutionResult {
                    success: true,
                    output,
                    error: None,
                })
            }
            Err(e) => {
                let error_msg = e.to_string();
                self.log_error_with_emit(task_id, &session_id, error_msg.clone())
                    .await;

                self.emit_progress(task_id, &session_id, ExecutionPhase::Failed, &error_msg);

                Ok(AgentTaskExecutionResult {
                    success: false,
                    output: String::new(),
                    error: Some(error_msg),
                })
            }
        }
    }

    /// Executes an agent task command in a Docker container
    async fn execute_agent_task_in_container(
        &self,
        agent_task: &AgentTask,
        worktree_path: &std::path::Path,
        cmd: &[String],
        task_id: &str,
        session_id: &str,
    ) -> ExecutionResult<String> {
        let docker_service = self
            .docker_service
            .as_ref()
            .ok_or(ExecutionError::DockerUnavailable)?;

        // Get repository info from the first base remote
        let repo_path = agent_task
            .base_remotes
            .first()
            .map(|r| PathBuf::from(&r.git_remote_dir_path))
            .ok_or_else(|| {
                ExecutionError::ClaudeCodeFailed("No base remote configured".to_string())
            })?;

        // Build environment variables
        let env_vars = self.build_env_vars()?;

        // Get Claude config path
        let claude_config_path = dirs::home_dir()
            .map(|h| h.join(".claude"))
            .filter(|p| p.exists())
            .map(|p| p.to_string_lossy().to_string());

        // Get or build Docker image
        let docker_image = docker_service
            .get_or_build_image(&repo_path, task_id, |_| {})
            .await?;

        // Create container
        let container_name = format!("delidev-agent-{}", agent_task.id);

        // Remove existing container if it exists
        let _ = docker_service.stop_container(&container_name).await;
        let _ = docker_service.remove_container(&container_name).await;

        // Extract repo name from worktree path for working directory
        let repo_name = worktree_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("workspace");
        let working_dir = format!("/workspace/{}", sanitize_repo_name(repo_name));

        let container_id = docker_service
            .create_agent_container_with_env(
                &container_name,
                &docker_image,
                &working_dir,
                worktree_path.to_str().unwrap(),
                env_vars,
                claude_config_path.as_deref(),
            )
            .await?;

        docker_service.start_container(&container_id).await?;

        // Copy Claude config from read-only mount to writable HOME directory
        let _ = docker_service
            .exec_in_container(
                &container_id,
                vec!["cp", "-r", "/tmp/claude-config", "/workspace/.claude"],
            )
            .await;

        // Execute the command
        let output = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
        let output_clone = output.clone();
        let task_service = self.task_service.clone();
        let session_id_clone = session_id.to_string();
        let task_id_clone = task_id.to_string();
        let app_handle = self.app_handle.clone();
        let logs = self.execution_logs.clone();

        // Line buffer for NDJSON parsing
        let line_buffer = std::sync::Mutex::new(String::new());

        let timeout_secs = 600; // 10 minutes
        let result = docker_service
            .exec_with_callback(&container_id, cmd.to_vec(), timeout_secs, move |chunk| {
                // Append to buffer and process complete lines
                let mut buffer = line_buffer.lock().unwrap();
                buffer.push_str(chunk);

                while let Some(newline_pos) = buffer.find('\n') {
                    let line = buffer[..newline_pos].trim().to_string();
                    *buffer = buffer[newline_pos + 1..].to_string();

                    if line.is_empty() {
                        continue;
                    }

                    // Collect output
                    if let Ok(mut out) = output_clone.lock() {
                        out.push_str(&line);
                        out.push('\n');
                    }

                    // Try to parse as structured stream message
                    let parsed_message = if let Ok(claude_msg) =
                        serde_json::from_str::<ClaudeStreamMessage>(&line)
                    {
                        Some(AgentStreamMessage::ClaudeCode(claude_msg))
                    } else if let Ok(opencode_msg) = serde_json::from_str::<OpenCodeMessage>(&line)
                    {
                        Some(AgentStreamMessage::OpenCode(opencode_msg))
                    } else {
                        None
                    };

                    match parsed_message {
                        Some(message) => {
                            // Save stream message to database
                            let task_service_clone = task_service.clone();
                            let session_id_for_save = session_id_clone.clone();
                            let message_clone = message.clone();
                            tokio::spawn(async move {
                                if let Err(e) = task_service_clone
                                    .save_stream_message(&session_id_for_save, &message_clone)
                                    .await
                                {
                                    tracing::error!("Failed to save stream message: {}", e);
                                }
                            });

                            // Emit structured stream event
                            if let Some(app) = &app_handle {
                                let event = AgentStreamEvent {
                                    task_id: task_id_clone.clone(),
                                    session_id: session_id_clone.clone(),
                                    timestamp: chrono::Utc::now(),
                                    message,
                                };
                                let _ = app.emit("claude-stream", event);
                            }
                        }
                        None => {
                            // Fallback: emit as raw log
                            let log = ExecutionLog::new(
                                uuid::Uuid::new_v4().to_string(),
                                session_id_clone.clone(),
                                LogLevel::Info,
                                line,
                            );

                            if let Some(app) = &app_handle {
                                let event = ExecutionLogEvent {
                                    task_id: task_id_clone.clone(),
                                    session_id: session_id_clone.clone(),
                                    log: log.clone(),
                                };
                                let _ = app.emit("execution-log", event);
                            }

                            if let Ok(mut logs_guard) = logs.try_write() {
                                logs_guard.push(log);
                            }
                        }
                    }
                }
            })
            .await;

        // Cleanup container
        let _ = docker_service.stop_container(&container_id).await;
        let _ = docker_service.remove_container(&container_id).await;

        match result {
            Ok(exec_result) => {
                if exec_result.exit_code == Some(0) {
                    let collected_output = output.lock().unwrap().clone();
                    Ok(collected_output)
                } else {
                    Err(ExecutionError::ClaudeCodeFailed(format!(
                        "Agent exited with code {:?}: {}",
                        exec_result.exit_code, exec_result.stderr
                    )))
                }
            }
            Err(e) => Err(e.into()),
        }
    }

    /// Executes an agent task command directly on the host
    async fn execute_agent_task_direct(
        &self,
        worktree_path: &PathBuf,
        cmd: &[String],
        task_id: &str,
        session_id: &str,
    ) -> ExecutionResult<String> {
        let output = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
        let output_clone = output.clone();
        let task_service = self.task_service.clone();
        let session_id_clone = session_id.to_string();
        let task_id_clone = task_id.to_string();
        let app_handle = self.app_handle.clone();
        let logs = self.execution_logs.clone();

        let exit_code = self
            .execute_command_direct(cmd, worktree_path, move |line| {
                // Collect output
                if let Ok(mut out) = output_clone.lock() {
                    out.push_str(line);
                    out.push('\n');
                }

                // Try to parse as structured stream message
                let parsed_message =
                    if let Ok(claude_msg) = serde_json::from_str::<ClaudeStreamMessage>(line) {
                        Some(AgentStreamMessage::ClaudeCode(claude_msg))
                    } else if let Ok(opencode_msg) = serde_json::from_str::<OpenCodeMessage>(line) {
                        Some(AgentStreamMessage::OpenCode(opencode_msg))
                    } else {
                        None
                    };

                match parsed_message {
                    Some(message) => {
                        // Save stream message to database
                        let task_service_clone = task_service.clone();
                        let session_id_for_save = session_id_clone.clone();
                        let message_clone = message.clone();
                        tokio::spawn(async move {
                            if let Err(e) = task_service_clone
                                .save_stream_message(&session_id_for_save, &message_clone)
                                .await
                            {
                                tracing::error!("Failed to save stream message: {}", e);
                            }
                        });

                        // Emit structured stream event
                        if let Some(app) = &app_handle {
                            let event = AgentStreamEvent {
                                task_id: task_id_clone.clone(),
                                session_id: session_id_clone.clone(),
                                timestamp: chrono::Utc::now(),
                                message,
                            };
                            let _ = app.emit("claude-stream", event);
                        }
                    }
                    None => {
                        // Fallback: emit as raw log if not empty
                        let trimmed = line.trim();
                        if !trimmed.is_empty() {
                            let log = ExecutionLog::new(
                                uuid::Uuid::new_v4().to_string(),
                                session_id_clone.clone(),
                                LogLevel::Info,
                                trimmed.to_string(),
                            );

                            if let Some(app) = &app_handle {
                                let event = ExecutionLogEvent {
                                    task_id: task_id_clone.clone(),
                                    session_id: session_id_clone.clone(),
                                    log: log.clone(),
                                };
                                let _ = app.emit("execution-log", event);
                            }

                            if let Ok(mut logs_guard) = logs.try_write() {
                                logs_guard.push(log);
                            }
                        }
                    }
                }
            })
            .await?;

        if exit_code == 0 {
            let collected_output = output.lock().unwrap().clone();
            Ok(collected_output)
        } else {
            Err(ExecutionError::ClaudeCodeFailed(format!(
                "Agent exited with code {}",
                exit_code
            )))
        }
    }

    /// Triggers execution of dependent tasks when a UnitTask in a CompositeTask
    /// completes. This method finds all nodes whose dependencies are now
    /// satisfied and starts their execution in parallel.
    ///
    /// This should be called after a UnitTask transitions to Done status.
    pub async fn trigger_dependent_tasks(self: &Arc<Self>, completed_task_id: &str) {
        Self::spawn_dependent_tasks(
            self.clone(),
            self.task_service.clone(),
            completed_task_id.to_string(),
        );
    }

    /// Spawns execution of dependent tasks whose dependencies are now
    /// satisfied.
    ///
    /// This is a static method that takes owned values to avoid Send issues
    /// with recursive async calls. When a dependent task completes, it
    /// needs to trigger its own dependents, creating a recursive call
    /// pattern. Using `&Arc<Self>` in an async block that gets spawned
    /// requires the future to be Send, but the borrow checker can't prove
    /// the reference lives long enough across the spawn boundary. By taking
    /// owned `Arc<Self>` and `Arc<TaskService>`, we avoid these
    /// lifetime/Send issues entirely.
    ///
    /// This method spawns a tokio task that:
    /// 1. Queries for tasks whose dependencies are now satisfied
    /// 2. For each ready task, spawns another task to execute it
    /// 3. After execution, recursively triggers dependents if the task
    ///    completed
    fn spawn_dependent_tasks(
        exec_service: Arc<Self>,
        task_service: Arc<TaskService>,
        completed_task_id: String,
    ) {
        tokio::spawn(async move {
            // Get the list of ready dependent tasks
            let ready_task_ids = match task_service
                .get_ready_dependent_tasks(&completed_task_id)
                .await
            {
                Ok(ids) => ids,
                Err(e) => {
                    tracing::error!("Failed to get ready dependent tasks: {}", e);
                    return;
                }
            };

            if ready_task_ids.is_empty() {
                tracing::debug!(
                    "No dependent tasks ready to execute after task {} completed",
                    completed_task_id
                );
                return;
            }

            tracing::info!(
                "Triggering {} dependent tasks after task {} completed",
                ready_task_ids.len(),
                completed_task_id
            );

            // Start execution of each ready task
            for unit_task_id in ready_task_ids {
                // Get the UnitTask
                let unit_task = match task_service.get_unit_task(&unit_task_id).await {
                    Ok(Some(task)) => task,
                    Ok(None) => {
                        tracing::error!("UnitTask not found: {}", unit_task_id);
                        continue;
                    }
                    Err(e) => {
                        tracing::error!("Failed to get UnitTask {}: {}", unit_task_id, e);
                        continue;
                    }
                };

                // Get the AgentTask
                let agent_task = match task_service.get_agent_task(&unit_task.agent_task_id).await {
                    Ok(Some(task)) => task,
                    Ok(None) => {
                        tracing::error!("AgentTask not found: {}", unit_task.agent_task_id);
                        continue;
                    }
                    Err(e) => {
                        tracing::error!(
                            "Failed to get AgentTask {}: {}",
                            unit_task.agent_task_id,
                            e
                        );
                        continue;
                    }
                };

                // Check concurrency limits if service is available
                let task_guard = if let (Some(concurrency_service), Some(global_config)) = (
                    exec_service.concurrency_service.as_ref(),
                    exec_service.global_config.as_ref(),
                ) {
                    let config = global_config.read().await;
                    match concurrency_service
                        .try_start_task(&config.concurrency, &unit_task.id)
                        .await
                    {
                        Ok(guard) => Some(guard),
                        Err(ConcurrencyError::LimitReached { current, limit }) => {
                            // Add to pending queue instead of executing
                            concurrency_service.add_pending_task(&unit_task.id).await;
                            tracing::info!(
                                "Dependent task {} queued for execution (concurrency limit: {}/{})",
                                unit_task.id,
                                current,
                                limit
                            );
                            continue;
                        }
                    }
                } else {
                    // No concurrency service, proceed without guard
                    None
                };

                // Spawn execution in background
                let exec_service_inner = exec_service.clone();
                let task_service_inner = task_service.clone();
                let unit_task_clone = unit_task.clone();
                let agent_task_clone = agent_task.clone();

                tokio::spawn(async move {
                    // TaskGuard is moved into this closure - it will auto-unregister on drop
                    let _guard = task_guard;

                    tracing::info!("Starting dependent task: {}", unit_task_clone.id);
                    let exec_result = exec_service_inner
                        .execute_unit_task(&unit_task_clone, &agent_task_clone)
                        .await;

                    match exec_result {
                        Ok(_) => {
                            // After successful execution, check if task completed with Done status
                            // and trigger any tasks that depend on this one.
                            if let Ok(Some(updated_task)) =
                                task_service_inner.get_unit_task(&unit_task_clone.id).await
                            {
                                if updated_task.status == UnitTaskStatus::Done {
                                    // Recursively trigger cascading dependent tasks
                                    Self::spawn_dependent_tasks(
                                        exec_service_inner.clone(),
                                        task_service_inner.clone(),
                                        unit_task_clone.id.clone(),
                                    );
                                }
                            }
                        }
                        Err(e) => {
                            tracing::error!(
                                "Dependent task {} execution failed: {}",
                                unit_task_clone.id,
                                e
                            );
                        }
                    }
                    // TaskGuard drops here if present, automatically
                    // unregistering the task
                });
            }
        });
    }
}

/// Result of executing an AgentTask
#[derive(Debug, Clone)]
pub struct AgentTaskExecutionResult {
    /// Whether the execution was successful
    pub success: bool,
    /// Output from the agent execution
    pub output: String,
    /// Error message if execution failed
    pub error: Option<String>,
}

/// Sanitizes a repository name for safe use in filesystem paths.
/// Replaces any character that is not alphanumeric, hyphen, underscore, or dot
/// with an underscore.
fn sanitize_repo_name(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_repo_name() {
        assert_eq!(sanitize_repo_name("my-repo"), "my-repo");
        assert_eq!(sanitize_repo_name("my_repo"), "my_repo");
        assert_eq!(sanitize_repo_name("my.repo"), "my.repo");
        assert_eq!(sanitize_repo_name("my/repo"), "my_repo");
        assert_eq!(sanitize_repo_name("my repo"), "my_repo");
        assert_eq!(sanitize_repo_name("my@repo#test"), "my_repo_test");
    }

    #[test]
    fn test_generate_unit_task_prompt() {
        let prompt = AgentExecutionService::generate_unit_task_prompt(
            "Add user authentication to the app",
            true,
        );

        // Verify the prompt contains the user request
        assert!(prompt.contains("Add user authentication to the app"));

        // Verify the prompt contains commit instructions
        assert!(prompt.contains("commit your changes using git"));
        assert!(prompt.contains("Ensure all changes are committed before finishing the task"));
        assert!(prompt.contains("Always commit your work before finishing the task"));

        // Verify the prompt does NOT contain learning instructions for regular prompts
        assert!(!prompt.contains("Auto-Learning from Feedback"));
    }

    #[test]
    fn test_generate_unit_task_prompt_with_feedback() {
        let prompt_with_feedback = r#"Add user authentication to the app

---

## Feedback from review

Please use bcrypt instead of SHA256 for password hashing."#;

        let prompt = AgentExecutionService::generate_unit_task_prompt(prompt_with_feedback, true);

        // Verify the prompt contains the user request and feedback
        assert!(prompt.contains("Add user authentication to the app"));
        assert!(prompt.contains("Feedback from review"));
        assert!(prompt.contains("bcrypt instead of SHA256"));

        // Verify the prompt contains learning instructions
        assert!(prompt.contains("Auto-Learning from Feedback"));
        assert!(prompt.contains("AGENTS.md"));
        assert!(prompt.contains("CLAUDE.md"));
        assert!(prompt.contains("generalizable guidelines"));
    }

    #[test]
    fn test_generate_unit_task_prompt_with_feedback_auto_learn_disabled() {
        let prompt_with_feedback = r#"Add user authentication to the app

---

## Feedback from review

Please use bcrypt instead of SHA256 for password hashing."#;

        // When auto_learn_enabled is false, learning instructions should NOT be added
        let prompt = AgentExecutionService::generate_unit_task_prompt(prompt_with_feedback, false);

        // Verify the prompt contains the user request and feedback
        assert!(prompt.contains("Add user authentication to the app"));
        assert!(prompt.contains("Feedback from review"));
        assert!(prompt.contains("bcrypt instead of SHA256"));

        // Verify the prompt does NOT contain learning instructions when disabled
        assert!(!prompt.contains("Auto-Learning from Feedback"));
    }
}
