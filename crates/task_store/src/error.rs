//! Error types for task store operations

use thiserror::Error;

/// Errors that can occur during store operations
#[derive(Error, Debug)]
pub enum StoreError {
    /// Database connection error
    #[error("Database connection error: {0}")]
    Connection(String),

    /// Query execution error
    #[error("Query error: {0}")]
    Query(String),

    /// Entity not found
    #[error("Entity not found: {0}")]
    NotFound(String),

    /// Constraint violation (e.g., unique constraint)
    #[error("Constraint violation: {0}")]
    Constraint(String),

    /// Migration error
    #[error("Migration error: {0}")]
    Migration(String),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// Invalid state transition
    #[error("Invalid state transition: {0}")]
    InvalidStateTransition(String),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

#[cfg(not(any(target_os = "android", target_os = "ios")))]
impl From<sqlx::Error> for StoreError {
    fn from(err: sqlx::Error) -> Self {
        match err {
            sqlx::Error::RowNotFound => StoreError::NotFound("Row not found".to_string()),
            sqlx::Error::Database(db_err) => {
                if db_err.is_unique_violation() {
                    StoreError::Constraint(db_err.to_string())
                } else {
                    StoreError::Query(db_err.to_string())
                }
            }
            _ => StoreError::Query(err.to_string()),
        }
    }
}

impl From<serde_json::Error> for StoreError {
    fn from(err: serde_json::Error) -> Self {
        StoreError::Serialization(err.to_string())
    }
}

/// Result type for store operations
pub type StoreResult<T> = Result<T, StoreError>;
