//! Agent-related entity definitions.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Type of version control system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum VcsType {
    /// Git version control system.
    #[default]
    Git,
}

/// Type of VCS provider.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VcsProviderType {
    /// GitHub
    Github,
    /// GitLab
    Gitlab,
    /// Bitbucket
    Bitbucket,
}

/// Type of AI coding agent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AiAgentType {
    /// Claude Code - Anthropic's terminal-based agentic coding tool.
    #[default]
    ClaudeCode,
    /// OpenCode - Open-source Claude Code alternative.
    OpenCode,
    /// Gemini CLI - Google's open-source AI agent.
    GeminiCli,
    /// Codex CLI - OpenAI's terminal-based coding assistant.
    CodexCli,
    /// Aider - Open-source CLI for multi-file changes.
    Aider,
    /// Amp - Sourcegraph's agentic coding CLI.
    Amp,
}

impl AiAgentType {
    /// Returns the command used to invoke this agent.
    pub fn command(&self) -> &'static str {
        match self {
            Self::ClaudeCode => "claude",
            Self::OpenCode => "opencode",
            Self::GeminiCli => "gemini",
            Self::CodexCli => "codex",
            Self::Aider => "aider",
            Self::Amp => "amp",
        }
    }
}

/// A single AI coding agent session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSession {
    /// Unique identifier.
    pub id: Uuid,
    /// Associated AgentTask ID.
    pub agent_task_id: Uuid,
    /// Type of AI agent.
    pub ai_agent_type: AiAgentType,
    /// Optional model override.
    pub ai_agent_model: Option<String>,
    /// When the session started.
    pub started_at: Option<DateTime<Utc>>,
    /// When the session completed.
    pub completed_at: Option<DateTime<Utc>>,
    /// Output log from the agent.
    pub output_log: Option<String>,
    /// When this record was created.
    pub created_at: DateTime<Utc>,
}

impl AgentSession {
    /// Creates a new agent session.
    pub fn new(agent_task_id: Uuid, ai_agent_type: AiAgentType) -> Self {
        Self {
            id: Uuid::new_v4(),
            agent_task_id,
            ai_agent_type,
            ai_agent_model: None,
            started_at: None,
            completed_at: None,
            output_log: None,
            created_at: Utc::now(),
        }
    }

    /// Sets the model for this session.
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.ai_agent_model = Some(model.into());
        self
    }
}

/// Base remote information for an agent task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseRemote {
    /// Path to the git remote directory.
    pub git_remote_dir_path: String,
    /// Git branch name.
    pub git_branch_name: String,
}

/// A collection of AgentSessions. The retryable unit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTask {
    /// Unique identifier.
    pub id: Uuid,
    /// Git repository information.
    pub base_remotes: Vec<BaseRemote>,
    /// List of agent sessions.
    pub agent_sessions: Vec<AgentSession>,
    /// Optional default agent type.
    pub ai_agent_type: Option<AiAgentType>,
    /// Optional default model.
    pub ai_agent_model: Option<String>,
    /// When this record was created.
    pub created_at: DateTime<Utc>,
}

impl AgentTask {
    /// Creates a new agent task.
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4(),
            base_remotes: Vec::new(),
            agent_sessions: Vec::new(),
            ai_agent_type: None,
            ai_agent_model: None,
            created_at: Utc::now(),
        }
    }

    /// Adds a base remote to this task.
    pub fn add_base_remote(&mut self, dir_path: impl Into<String>, branch_name: impl Into<String>) {
        self.base_remotes.push(BaseRemote {
            git_remote_dir_path: dir_path.into(),
            git_branch_name: branch_name.into(),
        });
    }

    /// Adds an agent session to this task.
    pub fn add_session(&mut self, session: AgentSession) {
        self.agent_sessions.push(session);
    }
}

impl Default for AgentTask {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ai_agent_type_command() {
        assert_eq!(AiAgentType::ClaudeCode.command(), "claude");
        assert_eq!(AiAgentType::OpenCode.command(), "opencode");
        assert_eq!(AiAgentType::GeminiCli.command(), "gemini");
        assert_eq!(AiAgentType::CodexCli.command(), "codex");
        assert_eq!(AiAgentType::Aider.command(), "aider");
        assert_eq!(AiAgentType::Amp.command(), "amp");
    }

    #[test]
    fn test_agent_session_creation() {
        let task_id = Uuid::new_v4();
        let session = AgentSession::new(task_id, AiAgentType::ClaudeCode)
            .with_model("claude-sonnet-4-20250514");

        assert_eq!(session.agent_task_id, task_id);
        assert_eq!(session.ai_agent_type, AiAgentType::ClaudeCode);
        assert_eq!(
            session.ai_agent_model,
            Some("claude-sonnet-4-20250514".to_string())
        );
    }

    #[test]
    fn test_agent_task_creation() {
        let mut task = AgentTask::new();
        task.add_base_remote("/path/to/repo", "main");

        assert_eq!(task.base_remotes.len(), 1);
        assert_eq!(task.base_remotes[0].git_remote_dir_path, "/path/to/repo");
        assert_eq!(task.base_remotes[0].git_branch_name, "main");
    }
}
