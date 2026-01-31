//! RPC error types.

use thiserror::Error;

use crate::error_codes;

/// Errors that can occur during RPC operations.
#[derive(Debug, Error)]
pub enum RpcError {
    /// Invalid request format.
    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    /// Method not found.
    #[error("Method not found: {0}")]
    MethodNotFound(String),

    /// Invalid parameters.
    #[error("Invalid parameters: {0}")]
    InvalidParams(String),

    /// Internal server error.
    #[error("Internal error: {0}")]
    InternalError(String),

    /// Authentication required.
    #[error("Authentication required")]
    AuthenticationRequired,

    /// Permission denied.
    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    /// Resource not found.
    #[error("Resource not found: {0}")]
    ResourceNotFound(String),

    /// No worker available.
    #[error("No worker available")]
    WorkerUnavailable,

    /// Task execution failed.
    #[error("Task execution failed: {0}")]
    TaskExecutionFailed(String),
}

impl RpcError {
    /// Returns the error code for this error.
    pub fn code(&self) -> i32 {
        match self {
            RpcError::InvalidRequest(_) => error_codes::INVALID_REQUEST,
            RpcError::MethodNotFound(_) => error_codes::METHOD_NOT_FOUND,
            RpcError::InvalidParams(_) => error_codes::INVALID_PARAMS,
            RpcError::InternalError(_) => error_codes::INTERNAL_ERROR,
            RpcError::AuthenticationRequired => error_codes::AUTHENTICATION_REQUIRED,
            RpcError::PermissionDenied(_) => error_codes::PERMISSION_DENIED,
            RpcError::ResourceNotFound(_) => error_codes::RESOURCE_NOT_FOUND,
            RpcError::WorkerUnavailable => error_codes::WORKER_UNAVAILABLE,
            RpcError::TaskExecutionFailed(_) => error_codes::TASK_EXECUTION_FAILED,
        }
    }

    /// Returns the error message.
    pub fn message(&self) -> String {
        self.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_codes() {
        assert_eq!(
            RpcError::InvalidRequest("test".to_string()).code(),
            error_codes::INVALID_REQUEST
        );
        assert_eq!(
            RpcError::AuthenticationRequired.code(),
            error_codes::AUTHENTICATION_REQUIRED
        );
        assert_eq!(
            RpcError::ResourceNotFound("task".to_string()).code(),
            error_codes::RESOURCE_NOT_FOUND
        );
    }
}
