//! Normalized output types for AI coding agents

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Normalized message type for all agents
///
/// This enum provides a unified message format that all AI coding agents
/// can output, regardless of their native output format.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum NormalizedMessage {
    /// Agent is starting
    Start { timestamp: DateTime<Utc> },

    /// Text output from agent (assistant message)
    Text {
        content: String,
        timestamp: DateTime<Utc>,
    },

    /// Code output from agent
    Code {
        language: Option<String>,
        content: String,
        timestamp: DateTime<Utc>,
    },

    /// Thinking/reasoning output (for models that support it)
    Thinking {
        content: String,
        timestamp: DateTime<Utc>,
    },

    /// Tool usage started
    ToolUse {
        tool_name: String,
        input: serde_json::Value,
        timestamp: DateTime<Utc>,
    },

    /// Tool result received
    ToolResult {
        tool_name: String,
        output: serde_json::Value,
        success: bool,
        timestamp: DateTime<Utc>,
    },

    /// Agent asking user a question
    UserQuestion {
        question: String,
        options: Option<Vec<QuestionOption>>,
        timestamp: DateTime<Utc>,
    },

    /// User response to a question
    UserResponse {
        response: String,
        timestamp: DateTime<Utc>,
    },

    /// Progress update
    Progress {
        phase: String,
        message: String,
        percentage: Option<u8>,
        timestamp: DateTime<Utc>,
    },

    /// Agent completed
    Complete {
        success: bool,
        summary: Option<String>,
        timestamp: DateTime<Utc>,
    },

    /// Error occurred
    Error {
        message: String,
        code: Option<String>,
        timestamp: DateTime<Utc>,
    },

    /// Raw/unknown message (for passthrough)
    Raw {
        content: String,
        timestamp: DateTime<Utc>,
    },
}

/// An option for a user question
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestionOption {
    /// Display label for the option
    pub label: String,
    /// Value to send back if selected
    pub value: String,
    /// Optional description
    pub description: Option<String>,
}

impl NormalizedMessage {
    /// Creates a start message
    pub fn start() -> Self {
        Self::Start {
            timestamp: Utc::now(),
        }
    }

    /// Creates a text message
    pub fn text(content: impl Into<String>) -> Self {
        Self::Text {
            content: content.into(),
            timestamp: Utc::now(),
        }
    }

    /// Creates a code message
    pub fn code(content: impl Into<String>, language: Option<String>) -> Self {
        Self::Code {
            language,
            content: content.into(),
            timestamp: Utc::now(),
        }
    }

    /// Creates a thinking message
    pub fn thinking(content: impl Into<String>) -> Self {
        Self::Thinking {
            content: content.into(),
            timestamp: Utc::now(),
        }
    }

    /// Creates a tool use message
    pub fn tool_use(tool_name: impl Into<String>, input: serde_json::Value) -> Self {
        Self::ToolUse {
            tool_name: tool_name.into(),
            input,
            timestamp: Utc::now(),
        }
    }

    /// Creates a tool result message
    pub fn tool_result(
        tool_name: impl Into<String>,
        output: serde_json::Value,
        success: bool,
    ) -> Self {
        Self::ToolResult {
            tool_name: tool_name.into(),
            output,
            success,
            timestamp: Utc::now(),
        }
    }

    /// Creates a user question message
    pub fn user_question(
        question: impl Into<String>,
        options: Option<Vec<QuestionOption>>,
    ) -> Self {
        Self::UserQuestion {
            question: question.into(),
            options,
            timestamp: Utc::now(),
        }
    }

    /// Creates a progress message
    pub fn progress(
        phase: impl Into<String>,
        message: impl Into<String>,
        percentage: Option<u8>,
    ) -> Self {
        Self::Progress {
            phase: phase.into(),
            message: message.into(),
            percentage,
            timestamp: Utc::now(),
        }
    }

    /// Creates a complete message
    pub fn complete(success: bool, summary: Option<String>) -> Self {
        Self::Complete {
            success,
            summary,
            timestamp: Utc::now(),
        }
    }

    /// Creates an error message
    pub fn error(message: impl Into<String>, code: Option<String>) -> Self {
        Self::Error {
            message: message.into(),
            code,
            timestamp: Utc::now(),
        }
    }

    /// Creates a raw message
    pub fn raw(content: impl Into<String>) -> Self {
        Self::Raw {
            content: content.into(),
            timestamp: Utc::now(),
        }
    }

    /// Returns the timestamp of this message
    pub fn timestamp(&self) -> DateTime<Utc> {
        match self {
            Self::Start { timestamp } => *timestamp,
            Self::Text { timestamp, .. } => *timestamp,
            Self::Code { timestamp, .. } => *timestamp,
            Self::Thinking { timestamp, .. } => *timestamp,
            Self::ToolUse { timestamp, .. } => *timestamp,
            Self::ToolResult { timestamp, .. } => *timestamp,
            Self::UserQuestion { timestamp, .. } => *timestamp,
            Self::UserResponse { timestamp, .. } => *timestamp,
            Self::Progress { timestamp, .. } => *timestamp,
            Self::Complete { timestamp, .. } => *timestamp,
            Self::Error { timestamp, .. } => *timestamp,
            Self::Raw { timestamp, .. } => *timestamp,
        }
    }

    /// Returns true if this is a terminal message (complete or error)
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Complete { .. } | Self::Error { .. })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_creation() {
        let text = NormalizedMessage::text("Hello, world!");
        assert!(
            matches!(text, NormalizedMessage::Text { content, .. } if content == "Hello, world!")
        );

        let error = NormalizedMessage::error("Something went wrong", Some("E001".to_string()));
        assert!(
            matches!(error, NormalizedMessage::Error { message, code, .. }
            if message == "Something went wrong" && code == Some("E001".to_string()))
        );
    }

    #[test]
    fn test_is_terminal() {
        assert!(!NormalizedMessage::text("test").is_terminal());
        assert!(NormalizedMessage::complete(true, None).is_terminal());
        assert!(NormalizedMessage::error("error", None).is_terminal());
    }

    #[test]
    fn test_serialization() {
        let msg = NormalizedMessage::tool_use("Bash", serde_json::json!({"command": "ls"}));
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"type\":\"tool_use\""));
        assert!(json.contains("\"tool_name\":\"Bash\""));
    }
}
