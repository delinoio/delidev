//! JSON-RPC request/response types

use serde::{Deserialize, Serialize};

use crate::JsonRpcError;

/// JSON-RPC version string
pub const JSONRPC_VERSION: &str = "2.0";

/// JSON-RPC request ID
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RequestId {
    /// String ID
    String(String),
    /// Number ID
    Number(i64),
    /// Null ID (for notifications)
    Null,
}

impl Default for RequestId {
    fn default() -> Self {
        Self::String(uuid::Uuid::new_v4().to_string())
    }
}

impl From<String> for RequestId {
    fn from(s: String) -> Self {
        Self::String(s)
    }
}

impl From<&str> for RequestId {
    fn from(s: &str) -> Self {
        Self::String(s.to_string())
    }
}

impl From<i64> for RequestId {
    fn from(n: i64) -> Self {
        Self::Number(n)
    }
}

/// JSON-RPC request object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    /// JSON-RPC version (always "2.0")
    pub jsonrpc: String,
    /// Request ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<RequestId>,
    /// Method name
    pub method: String,
    /// Method parameters
    #[serde(default, skip_serializing_if = "serde_json::Value::is_null")]
    pub params: serde_json::Value,
}

impl JsonRpcRequest {
    /// Creates a new request
    pub fn new(method: impl Into<String>, params: serde_json::Value) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_string(),
            id: Some(RequestId::default()),
            method: method.into(),
            params,
        }
    }

    /// Creates a new notification (no response expected)
    pub fn notification(method: impl Into<String>, params: serde_json::Value) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_string(),
            id: None,
            method: method.into(),
            params,
        }
    }

    /// Sets the request ID
    pub fn with_id(mut self, id: impl Into<RequestId>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Returns true if this is a notification (no ID)
    pub fn is_notification(&self) -> bool {
        self.id.is_none()
    }
}

/// JSON-RPC response object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    /// JSON-RPC version (always "2.0")
    pub jsonrpc: String,
    /// Request ID (matches the request)
    pub id: Option<RequestId>,
    /// Result (present on success)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    /// Error (present on failure)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

impl JsonRpcResponse {
    /// Creates a success response
    pub fn success(id: Option<RequestId>, result: serde_json::Value) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_string(),
            id,
            result: Some(result),
            error: None,
        }
    }

    /// Creates an error response
    pub fn error(id: Option<RequestId>, error: JsonRpcError) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_string(),
            id,
            result: None,
            error: Some(error),
        }
    }

    /// Returns true if this response is successful
    pub fn is_success(&self) -> bool {
        self.error.is_none()
    }

    /// Returns true if this response is an error
    pub fn is_error(&self) -> bool {
        self.error.is_some()
    }

    /// Converts to a Result
    pub fn into_result<T: serde::de::DeserializeOwned>(self) -> Result<T, JsonRpcError> {
        if let Some(error) = self.error {
            Err(error)
        } else if let Some(result) = self.result {
            serde_json::from_value(result).map_err(|e| {
                JsonRpcError::internal_error(format!("Failed to deserialize result: {}", e))
            })
        } else {
            Err(JsonRpcError::internal_error(
                "Response has neither result nor error",
            ))
        }
    }
}

/// Batch request (array of requests)
pub type JsonRpcBatchRequest = Vec<JsonRpcRequest>;

/// Batch response (array of responses)
pub type JsonRpcBatchResponse = Vec<JsonRpcResponse>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_serialization() {
        let request = JsonRpcRequest::new("test.method", serde_json::json!({"foo": "bar"}));
        let json = serde_json::to_string(&request).unwrap();

        assert!(json.contains("\"jsonrpc\":\"2.0\""));
        assert!(json.contains("\"method\":\"test.method\""));
        assert!(json.contains("\"foo\":\"bar\""));
    }

    #[test]
    fn test_notification() {
        let notification = JsonRpcRequest::notification("test.event", serde_json::json!({}));
        assert!(notification.is_notification());
    }

    #[test]
    fn test_response_success() {
        let response = JsonRpcResponse::success(
            Some(RequestId::String("test-123".to_string())),
            serde_json::json!({"data": "value"}),
        );

        assert!(response.is_success());
        assert!(!response.is_error());
    }

    #[test]
    fn test_response_error() {
        let response =
            JsonRpcResponse::error(Some(RequestId::Number(1)), JsonRpcError::not_found("Task"));

        assert!(!response.is_success());
        assert!(response.is_error());
    }

    #[test]
    fn test_into_result() {
        let response = JsonRpcResponse::success(None, serde_json::json!("hello"));
        let result: Result<String, _> = response.into_result();
        assert_eq!(result.unwrap(), "hello");
    }
}
