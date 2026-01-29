use serde::{Deserialize, Serialize};

/// Types of AI coding agents
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum AIAgentType {
    #[default]
    ClaudeCode,
    OpenCode,
    GeminiCli,
    CodexCli,
    Aider,
    Amp,
}

impl AIAgentType {
    /// Returns the display name for the agent type
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::ClaudeCode => "Claude Code",
            Self::OpenCode => "OpenCode",
            Self::GeminiCli => "Gemini CLI",
            Self::CodexCli => "Codex CLI",
            Self::Aider => "Aider",
            Self::Amp => "Amp",
        }
    }

    /// Returns the default model for this agent type
    pub fn default_model(&self) -> &'static str {
        match self {
            Self::ClaudeCode => "claude-sonnet-4-20250514",
            Self::OpenCode => "gpt-4o",
            Self::GeminiCli => "gemini-2.5-pro",
            Self::CodexCli => "gpt-5.2-codex",
            Self::Aider => "claude-sonnet-4-20250514",
            Self::Amp => "claude-sonnet-4-20250514",
        }
    }
}

/// A single AI coding agent session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSession {
    /// Unique identifier
    pub id: String,
    /// Agent type
    pub ai_agent_type: AIAgentType,
    /// Model to use (uses default if not specified)
    pub ai_agent_model: Option<String>,
}

impl AgentSession {
    pub fn new(id: String, ai_agent_type: AIAgentType) -> Self {
        Self {
            id,
            ai_agent_type,
            ai_agent_model: None,
        }
    }

    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.ai_agent_model = Some(model.into());
        self
    }

    /// Returns the effective model, using default if not specified
    pub fn effective_model(&self) -> &str {
        self.ai_agent_model
            .as_deref()
            .unwrap_or_else(|| self.ai_agent_type.default_model())
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
    /// Git repository information
    pub base_remotes: Vec<BaseRemote>,
    /// Session list (default 1, more on retry)
    pub agent_sessions: Vec<AgentSession>,
    /// Agent type (uses default agent if not specified)
    pub ai_agent_type: Option<AIAgentType>,
    /// Model to use
    pub ai_agent_model: Option<String>,
}

impl AgentTask {
    pub fn new(id: String, base_remotes: Vec<BaseRemote>) -> Self {
        Self {
            id,
            base_remotes,
            agent_sessions: Vec::new(),
            ai_agent_type: None,
            ai_agent_model: None,
        }
    }

    pub fn with_agent_type(mut self, agent_type: AIAgentType) -> Self {
        self.ai_agent_type = Some(agent_type);
        self
    }

    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.ai_agent_model = Some(model.into());
        self
    }

    pub fn add_session(&mut self, session: AgentSession) {
        self.agent_sessions.push(session);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod ai_agent_type {
        use super::*;

        #[test]
        fn test_display_name() {
            assert_eq!(AIAgentType::ClaudeCode.display_name(), "Claude Code");
            assert_eq!(AIAgentType::OpenCode.display_name(), "OpenCode");
            assert_eq!(AIAgentType::GeminiCli.display_name(), "Gemini CLI");
            assert_eq!(AIAgentType::CodexCli.display_name(), "Codex CLI");
            assert_eq!(AIAgentType::Aider.display_name(), "Aider");
            assert_eq!(AIAgentType::Amp.display_name(), "Amp");
        }

        #[test]
        fn test_default_model() {
            assert_eq!(
                AIAgentType::ClaudeCode.default_model(),
                "claude-sonnet-4-20250514"
            );
            assert_eq!(AIAgentType::OpenCode.default_model(), "gpt-4o");
            assert_eq!(AIAgentType::GeminiCli.default_model(), "gemini-2.5-pro");
            assert_eq!(AIAgentType::CodexCli.default_model(), "gpt-5.2-codex");
            assert_eq!(
                AIAgentType::Aider.default_model(),
                "claude-sonnet-4-20250514"
            );
            assert_eq!(AIAgentType::Amp.default_model(), "claude-sonnet-4-20250514");
        }

        #[test]
        fn test_serialization_roundtrip() {
            let agent_type = AIAgentType::ClaudeCode;
            let json = serde_json::to_string(&agent_type).unwrap();
            assert_eq!(json, "\"claude_code\"");

            let deserialized: AIAgentType = serde_json::from_str("\"open_code\"").unwrap();
            assert_eq!(deserialized, AIAgentType::OpenCode);

            // Test new agent types serialization
            assert_eq!(
                serde_json::to_string(&AIAgentType::GeminiCli).unwrap(),
                "\"gemini_cli\""
            );
            assert_eq!(
                serde_json::to_string(&AIAgentType::CodexCli).unwrap(),
                "\"codex_cli\""
            );
            assert_eq!(
                serde_json::to_string(&AIAgentType::Aider).unwrap(),
                "\"aider\""
            );
            assert_eq!(serde_json::to_string(&AIAgentType::Amp).unwrap(), "\"amp\"");

            // Test deserialization for new agent types
            let gemini: AIAgentType = serde_json::from_str("\"gemini_cli\"").unwrap();
            assert_eq!(gemini, AIAgentType::GeminiCli);

            let codex: AIAgentType = serde_json::from_str("\"codex_cli\"").unwrap();
            assert_eq!(codex, AIAgentType::CodexCli);

            let aider: AIAgentType = serde_json::from_str("\"aider\"").unwrap();
            assert_eq!(aider, AIAgentType::Aider);

            let amp: AIAgentType = serde_json::from_str("\"amp\"").unwrap();
            assert_eq!(amp, AIAgentType::Amp);
        }
    }

    mod agent_session {
        use super::*;

        #[test]
        fn test_effective_model_uses_custom_when_set() {
            let session = AgentSession::new("session-1".to_string(), AIAgentType::ClaudeCode)
                .with_model("custom-model");

            assert_eq!(session.effective_model(), "custom-model");
        }

        #[test]
        fn test_effective_model_falls_back_to_agent_type_default() {
            let claude_session =
                AgentSession::new("session-1".to_string(), AIAgentType::ClaudeCode);
            let opencode_session =
                AgentSession::new("session-2".to_string(), AIAgentType::OpenCode);

            assert_eq!(claude_session.effective_model(), "claude-sonnet-4-20250514");
            assert_eq!(opencode_session.effective_model(), "gpt-4o");
        }
    }

    mod agent_task {
        use super::*;

        #[test]
        fn test_add_session_accumulates_sessions() {
            let mut task = AgentTask::new("task-1".to_string(), vec![]);

            task.add_session(AgentSession::new(
                "session-1".to_string(),
                AIAgentType::ClaudeCode,
            ));
            task.add_session(AgentSession::new(
                "session-2".to_string(),
                AIAgentType::ClaudeCode,
            ));

            assert_eq!(task.agent_sessions.len(), 2);
        }
    }
}
