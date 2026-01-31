//! Server error types.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

/// Server error type.
#[derive(Debug, thiserror::Error)]
pub enum ServerError {
    /// Invalid request parameters.
    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    /// Resource not found.
    #[error("Not found: {0}")]
    NotFound(String),

    /// Authentication required.
    #[error("Authentication required")]
    AuthenticationRequired,

    /// Permission denied.
    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    /// Database error.
    #[error("Database error: {0}")]
    Database(#[from] task_store::TaskStoreError),

    /// Authentication error.
    #[error("Auth error: {0}")]
    Auth(#[from] auth::AuthError),

    /// Internal server error.
    #[error("Internal error: {0}")]
    Internal(String),

    /// Worker unavailable.
    #[error("Worker unavailable")]
    WorkerUnavailable,

    /// Task execution failed.
    #[error("Task execution failed: {0}")]
    TaskExecutionFailed(String),
}

impl IntoResponse for ServerError {
    fn into_response(self) -> Response {
        let (status, error_code, message) = match &self {
            ServerError::InvalidRequest(msg) => {
                (StatusCode::BAD_REQUEST, rpc_protocol::error_codes::INVALID_REQUEST, msg.clone())
            }
            ServerError::NotFound(msg) => {
                (StatusCode::NOT_FOUND, rpc_protocol::error_codes::RESOURCE_NOT_FOUND, msg.clone())
            }
            ServerError::AuthenticationRequired => {
                (StatusCode::UNAUTHORIZED, rpc_protocol::error_codes::AUTHENTICATION_REQUIRED, "Authentication required".to_string())
            }
            ServerError::PermissionDenied(msg) => {
                (StatusCode::FORBIDDEN, rpc_protocol::error_codes::PERMISSION_DENIED, msg.clone())
            }
            ServerError::Database(e) => {
                (StatusCode::INTERNAL_SERVER_ERROR, rpc_protocol::error_codes::INTERNAL_ERROR, e.to_string())
            }
            ServerError::Auth(e) => {
                (StatusCode::UNAUTHORIZED, rpc_protocol::error_codes::AUTHENTICATION_REQUIRED, e.to_string())
            }
            ServerError::Internal(msg) => {
                (StatusCode::INTERNAL_SERVER_ERROR, rpc_protocol::error_codes::INTERNAL_ERROR, msg.clone())
            }
            ServerError::WorkerUnavailable => {
                (StatusCode::SERVICE_UNAVAILABLE, rpc_protocol::error_codes::WORKER_UNAVAILABLE, "No workers available".to_string())
            }
            ServerError::TaskExecutionFailed(msg) => {
                (StatusCode::INTERNAL_SERVER_ERROR, rpc_protocol::error_codes::TASK_EXECUTION_FAILED, msg.clone())
            }
        };

        let body = json!({
            "error": {
                "code": error_code,
                "message": message,
            }
        });

        (status, Json(body)).into_response()
    }
}

/// Result type alias for server operations.
pub type ServerResult<T> = Result<T, ServerError>;
