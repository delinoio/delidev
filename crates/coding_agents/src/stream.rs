//! Stream message parsing and normalization for various AI coding agents

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{NormalizedMessage, QuestionOption};

/// Claude Code stream message types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClaudeStreamMessage {
    /// System message
    System {
        subtype: String,
        #[serde(default)]
        message: Option<String>,
    },

    /// Assistant message (text output)
    Assistant {
        #[serde(default)]
        message: AssistantMessage,
    },

    /// User message
    User {
        #[serde(default)]
        message: UserMessage,
    },

    /// Result message
    Result {
        #[serde(default)]
        subtype: Option<String>,
        #[serde(default)]
        result: Option<String>,
        #[serde(default)]
        is_error: bool,
        #[serde(default)]
        duration_ms: Option<u64>,
        #[serde(default)]
        duration_api_ms: Option<u64>,
        #[serde(default)]
        num_turns: Option<u32>,
    },
}

/// Assistant message content
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AssistantMessage {
    #[serde(default)]
    pub content: Vec<ContentBlock>,
}

/// Content block in an assistant message
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlock {
    /// Text content
    Text {
        text: String,
    },

    /// Thinking content
    Thinking {
        thinking: String,
    },

    /// Tool use
    ToolUse {
        id: String,
        name: String,
        input: Value,
    },

    /// Tool result
    ToolResult {
        tool_use_id: String,
        content: Value,
        #[serde(default)]
        is_error: bool,
    },
}

/// User message content
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UserMessage {
    #[serde(default)]
    pub content: Vec<UserContentBlock>,
}

/// User content block
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum UserContentBlock {
    /// Text content
    Text { text: String },

    /// Tool result
    ToolResult {
        tool_use_id: String,
        content: Value,
        #[serde(default)]
        is_error: bool,
    },
}

/// OpenCode stream message types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenCodeMessage {
    #[serde(rename = "type")]
    pub msg_type: String,
    #[serde(default)]
    pub content: Option<String>,
    #[serde(default)]
    pub tool_name: Option<String>,
    #[serde(default)]
    pub tool_input: Option<Value>,
    #[serde(default)]
    pub tool_output: Option<Value>,
    #[serde(default)]
    pub is_error: Option<bool>,
}

/// Parser for Claude Code stream messages
pub struct ClaudeStreamParser;

impl ClaudeStreamParser {
    /// Parse a JSON line from Claude Code output
    pub fn parse_line(line: &str) -> Option<ClaudeStreamMessage> {
        serde_json::from_str(line).ok()
    }

    /// Convert a Claude stream message to a normalized message
    pub fn normalize(msg: ClaudeStreamMessage) -> Vec<NormalizedMessage> {
        let mut messages = Vec::new();

        match msg {
            ClaudeStreamMessage::System { subtype, message } => {
                if subtype == "init" {
                    messages.push(NormalizedMessage::start());
                } else if let Some(text) = message {
                    messages.push(NormalizedMessage::progress(subtype, text, None));
                }
            }

            ClaudeStreamMessage::Assistant { message } => {
                for block in message.content {
                    match block {
                        ContentBlock::Text { text } => {
                            messages.push(NormalizedMessage::text(text));
                        }
                        ContentBlock::Thinking { thinking } => {
                            messages.push(NormalizedMessage::thinking(thinking));
                        }
                        ContentBlock::ToolUse { name, input, .. } => {
                            messages.push(NormalizedMessage::tool_use(name, input));
                        }
                        ContentBlock::ToolResult {
                            content,
                            is_error,
                            ..
                        } => {
                            messages.push(NormalizedMessage::tool_result(
                                "unknown",
                                content,
                                !is_error,
                            ));
                        }
                    }
                }
            }

            ClaudeStreamMessage::User { message } => {
                for block in message.content {
                    match block {
                        UserContentBlock::Text { text } => {
                            // Check if this is a question response
                            messages.push(NormalizedMessage::Raw {
                                content: text,
                                timestamp: chrono::Utc::now(),
                            });
                        }
                        UserContentBlock::ToolResult {
                            content, is_error, ..
                        } => {
                            messages.push(NormalizedMessage::tool_result(
                                "unknown",
                                content,
                                !is_error,
                            ));
                        }
                    }
                }
            }

            ClaudeStreamMessage::Result {
                is_error, result, ..
            } => {
                messages.push(NormalizedMessage::complete(!is_error, result));
            }
        }

        messages
    }
}

/// Parser for OpenCode stream messages
pub struct OpenCodeStreamParser;

impl OpenCodeStreamParser {
    /// Parse a JSON line from OpenCode output
    pub fn parse_line(line: &str) -> Option<OpenCodeMessage> {
        serde_json::from_str(line).ok()
    }

    /// Convert an OpenCode message to a normalized message
    pub fn normalize(msg: OpenCodeMessage) -> Option<NormalizedMessage> {
        match msg.msg_type.as_str() {
            "start" => Some(NormalizedMessage::start()),
            "text" | "assistant" => msg.content.map(NormalizedMessage::text),
            "thinking" => msg.content.map(NormalizedMessage::thinking),
            "tool_use" => {
                let name = msg.tool_name.unwrap_or_default();
                let input = msg.tool_input.unwrap_or(Value::Null);
                Some(NormalizedMessage::tool_use(name, input))
            }
            "tool_result" => {
                let name = msg.tool_name.unwrap_or_default();
                let output = msg.tool_output.unwrap_or(Value::Null);
                let success = !msg.is_error.unwrap_or(false);
                Some(NormalizedMessage::tool_result(name, output, success))
            }
            "question" => {
                let question = msg.content.unwrap_or_default();
                Some(NormalizedMessage::user_question(question, None))
            }
            "complete" | "done" => Some(NormalizedMessage::complete(
                !msg.is_error.unwrap_or(false),
                msg.content,
            )),
            "error" => Some(NormalizedMessage::error(
                msg.content.unwrap_or_else(|| "Unknown error".to_string()),
                None,
            )),
            _ => msg.content.map(NormalizedMessage::raw),
        }
    }
}

/// Detects question patterns in agent output
pub fn detect_user_question(text: &str) -> Option<(String, Vec<QuestionOption>)> {
    // Common patterns for questions in agent output
    let question_patterns = [
        "Would you like",
        "Do you want",
        "Should I",
        "Which ",
        "What ",
        "Please choose",
        "Select ",
        "Enter ",
    ];

    // Check if the text contains a question pattern
    let is_question = question_patterns
        .iter()
        .any(|p| text.contains(p) && text.contains('?'));

    if !is_question {
        return None;
    }

    // Extract options if they follow a numbered list pattern
    let options = extract_numbered_options(text);

    Some((text.to_string(), options))
}

/// Extract numbered options from text
fn extract_numbered_options(text: &str) -> Vec<QuestionOption> {
    let mut options = Vec::new();

    for line in text.lines() {
        let trimmed = line.trim();

        // Match patterns like "1.", "1)", "[1]", "a.", "a)"
        if let Some((label, rest)) = parse_option_line(trimmed) {
            options.push(QuestionOption {
                label: label.to_string(),
                value: label.to_string(),
                description: Some(rest.trim().to_string()),
            });
        }
    }

    options
}

/// Parse an option line and extract the label and description
fn parse_option_line(line: &str) -> Option<(&str, &str)> {
    // Match "1. text" or "1) text" or "[1] text"
    let chars: Vec<char> = line.chars().collect();
    if chars.is_empty() {
        return None;
    }

    let first = chars[0];
    if first.is_ascii_digit() || first.is_ascii_alphabetic() {
        if chars.len() > 1 {
            let second = chars[1];
            if second == '.' || second == ')' {
                let label_end = if chars.len() > 2 && chars[2] == ' ' {
                    1
                } else {
                    1
                };
                let rest_start = if chars.len() > 2 { 2 } else { chars.len() };
                return Some((&line[..label_end], &line[rest_start..]));
            }
        }
    }

    // Match "[1] text"
    if first == '[' {
        if let Some(close_bracket) = line.find(']') {
            let label = &line[1..close_bracket];
            let rest = &line[close_bracket + 1..];
            return Some((label, rest));
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_claude_stream_parser() {
        let json = r#"{"type":"assistant","message":{"content":[{"type":"text","text":"Hello!"}]}}"#;
        let msg = ClaudeStreamParser::parse_line(json).unwrap();
        let normalized = ClaudeStreamParser::normalize(msg);

        assert_eq!(normalized.len(), 1);
        assert!(matches!(&normalized[0], NormalizedMessage::Text { content, .. } if content == "Hello!"));
    }

    #[test]
    fn test_opencode_stream_parser() {
        let json = r#"{"type":"text","content":"Hello from OpenCode!"}"#;
        let msg = OpenCodeStreamParser::parse_line(json).unwrap();
        let normalized = OpenCodeStreamParser::normalize(msg);

        assert!(
            matches!(normalized, Some(NormalizedMessage::Text { content, .. }) if content == "Hello from OpenCode!")
        );
    }

    #[test]
    fn test_detect_user_question() {
        let text = "Which option would you prefer?\n1. Option A\n2. Option B";
        let result = detect_user_question(text);

        assert!(result.is_some());
        let (question, options) = result.unwrap();
        assert!(question.contains("Which option"));
        assert_eq!(options.len(), 2);
    }
}
