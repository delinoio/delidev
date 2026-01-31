//! AI coding agent trait and types

use std::{collections::HashMap, path::PathBuf};

use async_trait::async_trait;
use futures::Stream;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{AgentType, NormalizedMessage, SandboxConfig};

/// Errors that can occur during agent operations
#[derive(Error, Debug)]
pub enum AgentError {
    #[error("Agent not found: {0}")]
    NotFound(String),

    #[error("Agent execution failed: {0}")]
    ExecutionFailed(String),

    #[error("Agent timed out after {0} seconds")]
    Timeout(u64),

    #[error("Agent configuration error: {0}")]
    Configuration(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Docker error: {0}")]
    Docker(String),

    #[error("Stream error: {0}")]
    Stream(String),
}

/// Execution context for an agent
#[derive(Debug, Clone)]
pub struct ExecutionContext {
    /// Working directory for the agent
    pub work_dir: PathBuf,

    /// Environment variables to set
    pub env: HashMap<String, String>,

    /// Sandbox configuration (if running in container)
    pub sandbox: Option<SandboxConfig>,

    /// Timeout in seconds (0 for no timeout)
    pub timeout_seconds: u64,

    /// Model to use (overrides agent default)
    pub model: Option<String>,

    /// Whether to run in non-interactive mode
    pub non_interactive: bool,
}

impl Default for ExecutionContext {
    fn default() -> Self {
        Self {
            work_dir: PathBuf::from("."),
            env: HashMap::new(),
            sandbox: None,
            timeout_seconds: 0,
            model: None,
            non_interactive: true,
        }
    }
}

impl ExecutionContext {
    /// Creates a new execution context with the given working directory
    pub fn new(work_dir: PathBuf) -> Self {
        Self {
            work_dir,
            ..Default::default()
        }
    }

    /// Sets the environment variables
    pub fn with_env(mut self, env: HashMap<String, String>) -> Self {
        self.env = env;
        self
    }

    /// Sets the sandbox configuration
    pub fn with_sandbox(mut self, sandbox: SandboxConfig) -> Self {
        self.sandbox = Some(sandbox);
        self
    }

    /// Sets the timeout
    pub fn with_timeout(mut self, timeout_seconds: u64) -> Self {
        self.timeout_seconds = timeout_seconds;
        self
    }

    /// Sets the model
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }
}

/// Result of an agent execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    /// Whether the execution was successful
    pub success: bool,

    /// Exit code from the agent process
    pub exit_code: i32,

    /// Summary of what was done
    pub summary: Option<String>,

    /// Files that were modified
    pub modified_files: Vec<String>,

    /// Execution duration in milliseconds
    pub duration_ms: u64,
}

/// Trait for AI coding agents
#[async_trait]
pub trait CodingAgent: Send + Sync {
    /// Returns the agent type
    fn agent_type(&self) -> AgentType;

    /// Execute the agent with the given prompt
    async fn execute(
        &self,
        context: ExecutionContext,
        prompt: &str,
    ) -> Result<ExecutionResult, AgentError>;

    /// Stream execution output
    fn output_stream(&self) -> Box<dyn Stream<Item = NormalizedMessage> + Send + Unpin>;

    /// Stop execution
    async fn stop(&self) -> Result<(), AgentError>;

    /// Check if the agent is currently running
    fn is_running(&self) -> bool;
}

/// A single AI coding agent session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSession {
    /// Unique identifier
    pub id: String,
    /// Agent type
    pub agent_type: AgentType,
    /// Model to use (uses default if not specified)
    pub model: Option<String>,
    /// Created timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl AgentSession {
    /// Creates a new agent session
    pub fn new(agent_type: AgentType) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            agent_type,
            model: None,
            created_at: chrono::Utc::now(),
        }
    }

    /// Sets the model
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Returns the effective model, using default if not specified
    pub fn effective_model(&self) -> &str {
        self.model
            .as_deref()
            .unwrap_or_else(|| self.agent_type.default_model())
    }
}

/// Git repository information for a task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseRemote {
    /// Git repository path
    pub git_remote_dir_path: String,
    /// Branch name
    pub git_branch_name: String,
}

/// A collection of AgentSessions. The retryable unit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTask {
    /// Unique identifier
    pub id: String,
    /// Prompt for the task
    pub prompt: String,
    /// Git repository information
    pub base_remotes: Vec<BaseRemote>,
    /// Session list (default 1, more on retry)
    pub sessions: Vec<AgentSession>,
    /// Agent type (uses default agent if not specified)
    pub agent_type: Option<AgentType>,
    /// Model to use
    pub model: Option<String>,
    /// Created timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl AgentTask {
    /// Creates a new agent task
    pub fn new(prompt: impl Into<String>, base_remotes: Vec<BaseRemote>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            prompt: prompt.into(),
            base_remotes,
            sessions: Vec::new(),
            agent_type: None,
            model: None,
            created_at: chrono::Utc::now(),
        }
    }

    /// Sets the agent type
    pub fn with_agent_type(mut self, agent_type: AgentType) -> Self {
        self.agent_type = Some(agent_type);
        self
    }

    /// Sets the model
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Adds a session to the task
    pub fn add_session(&mut self, session: AgentSession) {
        self.sessions.push(session);
    }

    /// Returns the effective agent type
    pub fn effective_agent_type(&self) -> AgentType {
        self.agent_type.unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_session_effective_model() {
        let session = AgentSession::new(AgentType::ClaudeCode);
        assert_eq!(session.effective_model(), "claude-sonnet-4-20250514");

        let session_with_model =
            AgentSession::new(AgentType::ClaudeCode).with_model("custom-model");
        assert_eq!(session_with_model.effective_model(), "custom-model");
    }

    #[test]
    fn test_execution_context_builder() {
        let ctx = ExecutionContext::new(PathBuf::from("/tmp/test"))
            .with_timeout(300)
            .with_model("claude-opus-4");

        assert_eq!(ctx.work_dir, PathBuf::from("/tmp/test"));
        assert_eq!(ctx.timeout_seconds, 300);
        assert_eq!(ctx.model, Some("claude-opus-4".to_string()));
    }
}
