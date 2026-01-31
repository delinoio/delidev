//! Secret error types

use thiserror::Error;

/// Errors that can occur during secret operations
#[derive(Error, Debug)]
pub enum SecretError {
    /// Keychain access error
    #[error("Keychain error: {0}")]
    Keychain(String),

    /// Secret not found
    #[error("Secret not found: {0}")]
    NotFound(String),

    /// Permission denied
    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    /// Invalid secret format
    #[error("Invalid secret format: {0}")]
    InvalidFormat(String),

    /// Transport error
    #[error("Transport error: {0}")]
    Transport(String),

    /// Platform not supported
    #[error("Platform not supported: {0}")]
    PlatformNotSupported(String),
}

/// Result type for secret operations
pub type SecretResult<T> = Result<T, SecretError>;
