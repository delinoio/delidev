//! Secrets error types.

use thiserror::Error;

/// Errors that can occur during keychain operations.
#[derive(Debug, Error)]
pub enum SecretsError {
    /// Secret not found.
    #[error("Secret not found: {0}")]
    NotFound(String),

    /// Access denied.
    #[error("Access denied: {0}")]
    AccessDenied(String),

    /// Keychain unavailable.
    #[error("Keychain unavailable: {0}")]
    Unavailable(String),

    /// Invalid secret key.
    #[error("Invalid secret key: {0}")]
    InvalidKey(String),

    /// Platform-specific error.
    #[error("Platform error: {0}")]
    Platform(String),

    /// Other error.
    #[error("{0}")]
    Other(String),
}

impl From<keyring::Error> for SecretsError {
    fn from(e: keyring::Error) -> Self {
        match e {
            keyring::Error::NoEntry => SecretsError::NotFound("No entry found".to_string()),
            keyring::Error::Ambiguous(_) => {
                SecretsError::Platform("Ambiguous keychain entry".to_string())
            }
            keyring::Error::NoStorageAccess(_) => {
                SecretsError::AccessDenied("No storage access".to_string())
            }
            _ => SecretsError::Platform(e.to_string()),
        }
    }
}

/// Result type for secrets operations.
pub type SecretsResult<T> = Result<T, SecretsError>;
