use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Log level for execution logs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum LogLevel {
    Debug,
    #[default]
    Info,
    Warn,
    Error,
}

/// Execution status for an agent session
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum ExecutionStatus {
    #[default]
    Pending,
    Running,
    Completed,
    Failed(String),
}

/// Execution context for running an agent in a Docker container
#[derive(Debug, Clone)]
pub struct ExecutionContext {
    /// Task ID
    pub task_id: String,
    /// Agent session ID
    pub session_id: String,
    /// Docker container ID
    pub container_id: Option<String>,
    /// Path to the git worktree
    pub worktree_path: PathBuf,
    /// Working directory inside the container
    pub working_dir: String,
    /// Repository path (original)
    pub repo_path: PathBuf,
}

impl ExecutionContext {
    pub fn new(
        task_id: String,
        session_id: String,
        worktree_path: PathBuf,
        repo_path: PathBuf,
    ) -> Self {
        Self {
            task_id,
            session_id,
            container_id: None,
            worktree_path,
            working_dir: "/workspace".to_string(),
            repo_path,
        }
    }

    pub fn with_container_id(mut self, container_id: String) -> Self {
        self.container_id = Some(container_id);
        self
    }
}

/// A log entry from agent execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionLog {
    /// Unique identifier
    pub id: String,
    /// Agent session ID
    pub session_id: String,
    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Log level
    pub level: LogLevel,
    /// Log message
    pub message: String,
}

impl ExecutionLog {
    pub fn new(id: String, session_id: String, level: LogLevel, message: String) -> Self {
        Self {
            id,
            session_id,
            timestamp: chrono::Utc::now(),
            level,
            message,
        }
    }

    pub fn info(session_id: String, message: String) -> Self {
        Self::new(
            uuid::Uuid::new_v4().to_string(),
            session_id,
            LogLevel::Info,
            message,
        )
    }

    pub fn error(session_id: String, message: String) -> Self {
        Self::new(
            uuid::Uuid::new_v4().to_string(),
            session_id,
            LogLevel::Error,
            message,
        )
    }

    pub fn debug(session_id: String, message: String) -> Self {
        Self::new(
            uuid::Uuid::new_v4().to_string(),
            session_id,
            LogLevel::Debug,
            message,
        )
    }

    pub fn warn(session_id: String, message: String) -> Self {
        Self::new(
            uuid::Uuid::new_v4().to_string(),
            session_id,
            LogLevel::Warn,
            message,
        )
    }
}

// ========== Agent Stream Types ==========

/// A stored agent stream message entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStreamMessageEntry {
    /// Unique identifier
    pub id: String,
    /// Agent session ID
    pub session_id: String,
    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Stream message
    pub message: AgentStreamMessage,
}

impl AgentStreamMessageEntry {
    pub fn new(session_id: String, message: AgentStreamMessage) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            session_id,
            timestamp: chrono::Utc::now(),
            message,
        }
    }
}

/// Unified stream message types for all agents (Claude Code, OpenCode, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AgentStreamMessage {
    /// Claude Code stream-json format
    ClaudeCode(ClaudeStreamMessage),
    /// OpenCode JSON format
    OpenCode(OpenCodeMessage),
}

/// Claude Code stream-json message types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClaudeStreamMessage {
    /// System message (e.g., initialization info)
    System {
        subtype: String,
        #[serde(default)]
        parent_tool_use_id: Option<String>,
        #[serde(flatten)]
        data: Value,
    },
    /// Assistant message with content blocks
    Assistant {
        message: AssistantMessage,
        #[serde(default)]
        parent_tool_use_id: Option<String>,
    },
    /// User message
    User {
        message: UserMessage,
        #[serde(default)]
        parent_tool_use_id: Option<String>,
    },
    /// Result message (final output)
    Result {
        subtype: String,
        #[serde(default)]
        cost_usd: Option<f64>,
        #[serde(default)]
        duration_ms: Option<f64>,
        #[serde(default)]
        duration_api_ms: Option<f64>,
        #[serde(default)]
        is_error: bool,
        #[serde(default)]
        num_turns: Option<u32>,
        #[serde(default)]
        result: Option<String>,
        #[serde(default)]
        session_id: Option<String>,
        #[serde(default)]
        parent_tool_use_id: Option<String>,
    },
}

/// OpenCode JSON message format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenCodeMessage {
    /// Event type (e.g., "message", "tool_use", "result")
    #[serde(default)]
    pub event: Option<String>,
    /// Content of the message
    #[serde(default)]
    pub content: Option<String>,
    /// Tool name if this is a tool use
    #[serde(default)]
    pub tool: Option<String>,
    /// Whether this is an error
    #[serde(default)]
    pub error: Option<bool>,
    /// Additional arbitrary data
    #[serde(flatten)]
    pub data: Value,
}

/// Assistant message structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssistantMessage {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(rename = "type", default)]
    pub message_type: Option<String>,
    #[serde(default)]
    pub role: Option<String>,
    #[serde(default)]
    pub content: Vec<ContentBlock>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub stop_reason: Option<String>,
    #[serde(default)]
    pub stop_sequence: Option<String>,
}

/// User message structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserMessage {
    #[serde(default)]
    pub role: Option<String>,
    #[serde(default)]
    pub content: Vec<ContentBlock>,
}

/// Content block in a message
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlock {
    /// Text content
    Text { text: String },
    /// Tool use request
    ToolUse {
        id: String,
        name: String,
        input: Value,
    },
    /// Tool result
    ToolResult {
        tool_use_id: String,
        #[serde(default)]
        content: Option<String>,
        #[serde(default)]
        is_error: Option<bool>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    mod execution_status {
        use super::*;

        #[test]
        fn test_failed_variant_includes_error_message() {
            let status = ExecutionStatus::Failed("Something went wrong".to_string());
            let json = serde_json::to_string(&status).unwrap();
            assert!(json.contains("failed"));
            assert!(json.contains("Something went wrong"));
        }
    }

    mod execution_log {
        use super::*;

        #[test]
        fn test_helper_methods_set_correct_log_level() {
            let info = ExecutionLog::info("session-1".to_string(), "Info".to_string());
            let error = ExecutionLog::error("session-1".to_string(), "Error".to_string());
            let debug = ExecutionLog::debug("session-1".to_string(), "Debug".to_string());
            let warn = ExecutionLog::warn("session-1".to_string(), "Warn".to_string());

            assert_eq!(info.level, LogLevel::Info);
            assert_eq!(error.level, LogLevel::Error);
            assert_eq!(debug.level, LogLevel::Debug);
            assert_eq!(warn.level, LogLevel::Warn);
        }

        #[test]
        fn test_helper_methods_generate_unique_ids() {
            let log1 = ExecutionLog::info("session-1".to_string(), "Test 1".to_string());
            let log2 = ExecutionLog::info("session-1".to_string(), "Test 2".to_string());

            assert!(!log1.id.is_empty());
            assert!(!log2.id.is_empty());
            assert_ne!(log1.id, log2.id);
        }
    }

    mod content_block {
        use super::*;

        #[test]
        fn test_all_variants_serialize_with_type_tag() {
            let text_block = ContentBlock::Text {
                text: "Hello".to_string(),
            };
            let tool_use_block = ContentBlock::ToolUse {
                id: "tool-1".to_string(),
                name: "read_file".to_string(),
                input: serde_json::json!({}),
            };
            let tool_result_block = ContentBlock::ToolResult {
                tool_use_id: "tool-1".to_string(),
                content: Some("Result".to_string()),
                is_error: Some(false),
            };

            let text_json = serde_json::to_string(&text_block).unwrap();
            let tool_use_json = serde_json::to_string(&tool_use_block).unwrap();
            let tool_result_json = serde_json::to_string(&tool_result_block).unwrap();

            assert!(text_json.contains("\"type\":\"text\""));
            assert!(tool_use_json.contains("\"type\":\"tool_use\""));
            assert!(tool_result_json.contains("\"type\":\"tool_result\""));
        }
    }
}
