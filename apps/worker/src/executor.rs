//! Task execution

use std::{collections::HashMap, path::PathBuf, sync::Arc};

use coding_agents::{
    AgentError, AgentType, ContainerRuntime, ExecutionContext, ExecutionResult, NormalizedMessage,
    SandboxConfig,
};
use thiserror::Error;
use tracing::{error, info, warn};

use crate::{config::WorkerConfig, server_client::MainServerClient};

/// Task execution information
#[derive(Debug, Clone)]
pub struct TaskAssignment {
    /// Task ID
    pub task_id: String,
    /// Session ID
    pub session_id: String,
    /// Prompt to execute
    pub prompt: String,
    /// Agent type to use
    pub agent_type: AgentType,
    /// Model to use
    pub model: Option<String>,
    /// Repository information
    pub repository_path: PathBuf,
    /// Branch name
    pub branch_name: String,
    /// Secrets for the task
    pub secrets: HashMap<String, String>,
}

/// Task executor
pub struct TaskExecutor {
    /// Worker configuration
    config: Arc<WorkerConfig>,
    /// Server client for reporting
    client: Arc<MainServerClient>,
    /// Active task sessions
    active_sessions: tokio::sync::RwLock<HashMap<String, TaskSession>>,
}

/// An active task session
struct TaskSession {
    task_id: String,
    session_id: String,
    #[allow(dead_code)]
    cancel_tx: tokio::sync::oneshot::Sender<()>,
}

impl TaskExecutor {
    /// Create a new task executor
    pub fn new(config: Arc<WorkerConfig>, client: Arc<MainServerClient>) -> Self {
        Self {
            config,
            client,
            active_sessions: tokio::sync::RwLock::new(HashMap::new()),
        }
    }

    /// Execute a task
    pub async fn execute(&self, assignment: TaskAssignment) -> Result<(), ExecutorError> {
        let task_id = assignment.task_id.clone();
        let session_id = assignment.session_id.clone();

        info!(
            task_id = %task_id,
            session_id = %session_id,
            agent_type = ?assignment.agent_type,
            "Starting task execution"
        );

        // Create cancel channel
        let (cancel_tx, cancel_rx) = tokio::sync::oneshot::channel();

        // Register session
        {
            let mut sessions = self.active_sessions.write().await;
            sessions.insert(
                task_id.clone(),
                TaskSession {
                    task_id: task_id.clone(),
                    session_id: session_id.clone(),
                    cancel_tx,
                },
            );
        }

        // Send start message
        self.send_log(
            &task_id,
            &session_id,
            NormalizedMessage::Start {
                timestamp: chrono::Utc::now(),
            },
        )
        .await;

        // Execute the task
        let result = self.run_agent(&assignment, cancel_rx).await;

        // Send completion message
        match &result {
            Ok(exec_result) => {
                self.send_log(
                    &task_id,
                    &session_id,
                    NormalizedMessage::Complete {
                        success: exec_result.success,
                        summary: exec_result.summary.clone(),
                        timestamp: chrono::Utc::now(),
                    },
                )
                .await;

                // Report to server
                if let Err(e) = self
                    .client
                    .report_task_complete(
                        &task_id,
                        exec_result.success,
                        exec_result.summary.clone(),
                        Some(exec_result.exit_code),
                    )
                    .await
                {
                    error!(task_id = %task_id, error = %e, "Failed to report task completion");
                }
            }
            Err(e) => {
                self.send_log(
                    &task_id,
                    &session_id,
                    NormalizedMessage::Error {
                        message: e.to_string(),
                        code: None,
                        timestamp: chrono::Utc::now(),
                    },
                )
                .await;

                // Report failure to server
                if let Err(report_err) = self
                    .client
                    .report_task_complete(&task_id, false, Some(e.to_string()), None)
                    .await
                {
                    error!(task_id = %task_id, error = %report_err, "Failed to report task failure");
                }
            }
        }

        // Remove session
        {
            let mut sessions = self.active_sessions.write().await;
            sessions.remove(&task_id);
        }

        result.map(|_| ())
    }

    /// Run the AI agent
    async fn run_agent(
        &self,
        assignment: &TaskAssignment,
        mut cancel_rx: tokio::sync::oneshot::Receiver<()>,
    ) -> Result<ExecutionResult, ExecutorError> {
        let task_id = &assignment.task_id;
        let session_id = &assignment.session_id;

        // Create worktree directory
        let worktree_path = self.config.worktree_dir.join(task_id);
        std::fs::create_dir_all(&worktree_path).map_err(ExecutorError::Io)?;

        // Prepare environment with secrets
        let env = secrets::inject_secrets_to_env(assignment.secrets.clone());

        // Build execution context
        let mut context = ExecutionContext::new(worktree_path.clone()).with_env(env);

        if let Some(ref model) = assignment.model {
            context = context.with_model(model);
        }

        if self.config.use_container {
            let runtime = match self.config.container_runtime.as_str() {
                "podman" => ContainerRuntime::Podman,
                _ => ContainerRuntime::Docker,
            };
            let sandbox_config = SandboxConfig::new("node:20-slim")
                .with_runtime(runtime)
                .with_work_dir(worktree_path.clone())
                .with_volume(
                    assignment.repository_path.display().to_string(),
                    "/workspace",
                );
            // Add secrets as environment variables
            let mut sandbox_config = sandbox_config;
            for (key, value) in &assignment.secrets {
                sandbox_config = sandbox_config.with_env(key, value);
            }
            context = context.with_sandbox(sandbox_config);
        }

        // For now, simulate execution since we don't have actual agent implementations
        // In a full implementation, we would:
        // 1. Create git worktree from the repository
        // 2. Spawn the actual agent process
        // 3. Stream output to the server
        // 4. Wait for completion or cancellation

        info!(
            task_id = %task_id,
            prompt = %assignment.prompt,
            "Executing agent (simulated)"
        );

        // Send a text message indicating execution
        self.send_log(
            task_id,
            session_id,
            NormalizedMessage::Text {
                content: format!(
                    "Executing {} with prompt: {}",
                    assignment.agent_type.display_name(),
                    assignment.prompt
                ),
                timestamp: chrono::Utc::now(),
            },
        )
        .await;

        // Simulate execution with a delay
        tokio::select! {
            _ = tokio::time::sleep(std::time::Duration::from_secs(2)) => {
                // Normal completion
            }
            _ = &mut cancel_rx => {
                // Cancelled
                warn!(task_id = %task_id, "Task execution cancelled");
                return Err(ExecutorError::Cancelled);
            }
        }

        // Cleanup worktree
        if let Err(e) = std::fs::remove_dir_all(&worktree_path) {
            warn!(task_id = %task_id, error = %e, "Failed to cleanup worktree");
        }

        Ok(ExecutionResult {
            success: true,
            exit_code: 0,
            summary: Some("Task completed successfully (simulated)".to_string()),
            modified_files: Vec::new(),
            duration_ms: 2000,
        })
    }

    /// Send a log message to the server
    async fn send_log(&self, task_id: &str, session_id: &str, message: NormalizedMessage) {
        if let Err(e) = self
            .client
            .send_execution_log(task_id, session_id, message)
            .await
        {
            warn!(task_id = %task_id, error = %e, "Failed to send execution log");
        }
    }

    /// Stop a running task
    pub async fn stop_task(&self, task_id: &str) -> bool {
        let mut sessions = self.active_sessions.write().await;
        if let Some(session) = sessions.remove(task_id) {
            // Signal cancellation (the sender will drop and cancel_rx will receive)
            drop(session.cancel_tx);
            true
        } else {
            false
        }
    }

    /// Get the number of running tasks
    pub async fn running_task_count(&self) -> usize {
        self.active_sessions.read().await.len()
    }

    /// Check if a task is running
    pub async fn is_task_running(&self, task_id: &str) -> bool {
        self.active_sessions.read().await.contains_key(task_id)
    }
}

/// Executor errors
#[derive(Debug, Error)]
pub enum ExecutorError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Agent error: {0}")]
    Agent(#[from] AgentError),

    #[error("Task was cancelled")]
    Cancelled,

    #[error("Git error: {0}")]
    Git(String),

    #[error("Docker error: {0}")]
    Docker(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_assignment() {
        let assignment = TaskAssignment {
            task_id: "task-1".to_string(),
            session_id: "session-1".to_string(),
            prompt: "Do something".to_string(),
            agent_type: AgentType::ClaudeCode,
            model: None,
            repository_path: PathBuf::from("/tmp/repo"),
            branch_name: "feature/test".to_string(),
            secrets: HashMap::new(),
        };

        assert_eq!(assignment.task_id, "task-1");
        assert_eq!(assignment.agent_type, AgentType::ClaudeCode);
    }
}
