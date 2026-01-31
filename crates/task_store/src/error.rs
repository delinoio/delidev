//! Task store error types.

use thiserror::Error;

/// Errors that can occur during task store operations.
#[derive(Debug, Error)]
pub enum TaskStoreError {
    /// Entity not found.
    #[error("{entity_type} not found: {id}")]
    NotFound {
        entity_type: &'static str,
        id: String,
    },

    /// Duplicate entity.
    #[error("{entity_type} already exists: {id}")]
    AlreadyExists {
        entity_type: &'static str,
        id: String,
    },

    /// Database error.
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    /// Serialization error.
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Invalid state transition.
    #[error("Invalid state transition from {from} to {to}")]
    InvalidStateTransition { from: String, to: String },

    /// Foreign key constraint violation.
    #[error("Foreign key constraint violation: {0}")]
    ForeignKeyViolation(String),

    /// Other error.
    #[error("{0}")]
    Other(String),
}

impl TaskStoreError {
    /// Creates a not found error.
    pub fn not_found(entity_type: &'static str, id: impl Into<String>) -> Self {
        Self::NotFound {
            entity_type,
            id: id.into(),
        }
    }

    /// Creates an already exists error.
    pub fn already_exists(entity_type: &'static str, id: impl Into<String>) -> Self {
        Self::AlreadyExists {
            entity_type,
            id: id.into(),
        }
    }
}

/// Result type for task store operations.
pub type TaskStoreResult<T> = Result<T, TaskStoreError>;
