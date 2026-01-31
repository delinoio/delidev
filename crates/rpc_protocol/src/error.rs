//! JSON-RPC error types

use serde::{Deserialize, Serialize};

/// Standard JSON-RPC error codes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    /// Invalid JSON was received
    ParseError = -32700,
    /// The JSON sent is not a valid Request object
    InvalidRequest = -32600,
    /// The method does not exist / is not available
    MethodNotFound = -32601,
    /// Invalid method parameter(s)
    InvalidParams = -32602,
    /// Internal JSON-RPC error
    InternalError = -32603,

    // Server-defined errors (-32000 to -32099)
    /// Authentication required
    Unauthorized = -32001,
    /// User is authenticated but not allowed to perform this action
    Forbidden = -32002,
    /// The requested resource was not found
    NotFound = -32003,
    /// The request conflicts with the current state
    Conflict = -32004,
    /// The server is temporarily unavailable
    ServiceUnavailable = -32005,
    /// The operation timed out
    Timeout = -32006,
    /// Rate limit exceeded
    RateLimited = -32007,
}

impl From<i32> for ErrorCode {
    fn from(code: i32) -> Self {
        match code {
            -32700 => ErrorCode::ParseError,
            -32600 => ErrorCode::InvalidRequest,
            -32601 => ErrorCode::MethodNotFound,
            -32602 => ErrorCode::InvalidParams,
            -32603 => ErrorCode::InternalError,
            -32001 => ErrorCode::Unauthorized,
            -32002 => ErrorCode::Forbidden,
            -32003 => ErrorCode::NotFound,
            -32004 => ErrorCode::Conflict,
            -32005 => ErrorCode::ServiceUnavailable,
            -32006 => ErrorCode::Timeout,
            -32007 => ErrorCode::RateLimited,
            _ => ErrorCode::InternalError,
        }
    }
}

/// JSON-RPC error object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    /// Error code
    pub code: i32,
    /// Error message
    pub message: String,
    /// Optional additional data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

impl JsonRpcError {
    /// Creates a new error
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code: code as i32,
            message: message.into(),
            data: None,
        }
    }

    /// Creates a new error with additional data
    pub fn with_data(code: ErrorCode, message: impl Into<String>, data: serde_json::Value) -> Self {
        Self {
            code: code as i32,
            message: message.into(),
            data: Some(data),
        }
    }

    /// Creates a parse error
    pub fn parse_error(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::ParseError, message)
    }

    /// Creates an invalid request error
    pub fn invalid_request(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::InvalidRequest, message)
    }

    /// Creates a method not found error
    pub fn method_not_found(method: &str) -> Self {
        Self::new(
            ErrorCode::MethodNotFound,
            format!("Method '{}' not found", method),
        )
    }

    /// Creates an invalid params error
    pub fn invalid_params(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::InvalidParams, message)
    }

    /// Creates an internal error
    pub fn internal_error(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::InternalError, message)
    }

    /// Creates an unauthorized error
    pub fn unauthorized() -> Self {
        Self::new(ErrorCode::Unauthorized, "Authentication required")
    }

    /// Creates a forbidden error
    pub fn forbidden(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::Forbidden, message)
    }

    /// Creates a not found error
    pub fn not_found(resource: &str) -> Self {
        Self::new(ErrorCode::NotFound, format!("{} not found", resource))
    }

    /// Creates a conflict error
    pub fn conflict(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::Conflict, message)
    }

    /// Creates a service unavailable error
    pub fn service_unavailable(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::ServiceUnavailable, message)
    }

    /// Creates a timeout error
    pub fn timeout(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::Timeout, message)
    }

    /// Creates a rate limited error
    pub fn rate_limited(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::RateLimited, message)
    }
}

impl std::fmt::Display for JsonRpcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.code, self.message)
    }
}

impl std::error::Error for JsonRpcError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_serialization() {
        let error = JsonRpcError::new(ErrorCode::NotFound, "Task not found");
        let json = serde_json::to_string(&error).unwrap();

        assert!(json.contains("-32003"));
        assert!(json.contains("Task not found"));
    }

    #[test]
    fn test_error_with_data() {
        let error = JsonRpcError::with_data(
            ErrorCode::InvalidParams,
            "Missing required field",
            serde_json::json!({"field": "prompt"}),
        );

        assert!(error.data.is_some());
    }
}
