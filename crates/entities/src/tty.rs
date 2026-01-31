//! TTY input request entity definitions.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Type of TTY input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TtyInputType {
    /// Free-form text input.
    Text,
    /// Select from options.
    Select,
    /// Yes/no confirmation.
    Confirm,
    /// Password input (hidden).
    Password,
}

impl Default for TtyInputType {
    fn default() -> Self {
        Self::Text
    }
}

/// Status of a TTY input request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TtyInputStatus {
    /// Waiting for user response.
    Pending,
    /// User has responded.
    Responded,
    /// Request timed out.
    Timeout,
    /// Request cancelled.
    Cancelled,
}

impl Default for TtyInputStatus {
    fn default() -> Self {
        Self::Pending
    }
}

/// A request for TTY input from an AI agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TtyInputRequest {
    /// Unique identifier.
    pub id: Uuid,
    /// Associated UnitTask ID.
    pub task_id: Uuid,
    /// Associated AgentSession ID.
    pub session_id: Uuid,
    /// Question from the agent.
    pub prompt: String,
    /// Type of input requested.
    pub input_type: TtyInputType,
    /// Available options (for select type).
    pub options: Option<Vec<String>>,
    /// Current status.
    pub status: TtyInputStatus,
    /// User's response.
    pub response: Option<String>,
    /// When this record was created.
    pub created_at: DateTime<Utc>,
    /// When the user responded.
    pub responded_at: Option<DateTime<Utc>>,
}

impl TtyInputRequest {
    /// Creates a new TTY input request.
    pub fn new(task_id: Uuid, session_id: Uuid, prompt: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            task_id,
            session_id,
            prompt: prompt.into(),
            input_type: TtyInputType::Text,
            options: None,
            status: TtyInputStatus::Pending,
            response: None,
            created_at: Utc::now(),
            responded_at: None,
        }
    }

    /// Sets the input type.
    pub fn with_input_type(mut self, input_type: TtyInputType) -> Self {
        self.input_type = input_type;
        self
    }

    /// Sets the options for select-type input.
    pub fn with_options(mut self, options: Vec<String>) -> Self {
        self.input_type = TtyInputType::Select;
        self.options = Some(options);
        self
    }

    /// Records a response to this request.
    pub fn respond(&mut self, response: impl Into<String>) {
        self.response = Some(response.into());
        self.status = TtyInputStatus::Responded;
        self.responded_at = Some(Utc::now());
    }

    /// Marks this request as timed out.
    pub fn timeout(&mut self) {
        self.status = TtyInputStatus::Timeout;
    }

    /// Marks this request as cancelled.
    pub fn cancel(&mut self) {
        self.status = TtyInputStatus::Cancelled;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tty_input_request_creation() {
        let task_id = Uuid::new_v4();
        let session_id = Uuid::new_v4();
        let request = TtyInputRequest::new(task_id, session_id, "What is your name?");

        assert_eq!(request.task_id, task_id);
        assert_eq!(request.session_id, session_id);
        assert_eq!(request.prompt, "What is your name?");
        assert_eq!(request.input_type, TtyInputType::Text);
        assert_eq!(request.status, TtyInputStatus::Pending);
    }

    #[test]
    fn test_tty_input_request_with_options() {
        let task_id = Uuid::new_v4();
        let session_id = Uuid::new_v4();
        let request =
            TtyInputRequest::new(task_id, session_id, "Choose an option:").with_options(vec![
                "Option A".to_string(),
                "Option B".to_string(),
                "Option C".to_string(),
            ]);

        assert_eq!(request.input_type, TtyInputType::Select);
        assert_eq!(request.options.as_ref().unwrap().len(), 3);
    }

    #[test]
    fn test_tty_input_request_respond() {
        let task_id = Uuid::new_v4();
        let session_id = Uuid::new_v4();
        let mut request = TtyInputRequest::new(task_id, session_id, "Question?");

        request.respond("My answer");

        assert_eq!(request.status, TtyInputStatus::Responded);
        assert_eq!(request.response, Some("My answer".to_string()));
        assert!(request.responded_at.is_some());
    }
}
