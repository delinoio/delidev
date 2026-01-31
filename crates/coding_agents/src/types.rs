//! Platform-agnostic types for AI coding agents
//!
//! These types are available on all platforms, including mobile.

use serde::{Deserialize, Serialize};

/// Types of AI coding agents
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AgentType {
    /// Claude Code - Anthropic's terminal-based agentic coding tool
    #[default]
    ClaudeCode,
    /// OpenCode - Open-source Claude Code alternative supporting any model
    OpenCode,
    /// Gemini CLI - Google's open-source AI agent for terminal
    GeminiCli,
    /// Codex CLI - OpenAI's interactive terminal-based coding assistant
    CodexCli,
    /// Aider - Open-source CLI tool for multi-file changes via natural language
    Aider,
    /// Amp - Sourcegraph's agentic coding CLI tool
    Amp,
}

impl AgentType {
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

    /// Returns the command to execute for this agent type
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_type_display_name() {
        assert_eq!(AgentType::ClaudeCode.display_name(), "Claude Code");
        assert_eq!(AgentType::OpenCode.display_name(), "OpenCode");
        assert_eq!(AgentType::GeminiCli.display_name(), "Gemini CLI");
        assert_eq!(AgentType::CodexCli.display_name(), "Codex CLI");
        assert_eq!(AgentType::Aider.display_name(), "Aider");
        assert_eq!(AgentType::Amp.display_name(), "Amp");
    }

    #[test]
    fn test_agent_type_serialization() {
        let agent = AgentType::ClaudeCode;
        let json = serde_json::to_string(&agent).unwrap();
        assert_eq!(json, "\"claude_code\"");

        let deserialized: AgentType = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, agent);
    }
}
