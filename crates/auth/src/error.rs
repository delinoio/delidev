//! Authentication error types.

use thiserror::Error;

/// Errors that can occur during authentication operations.
#[derive(Debug, Error)]
pub enum AuthError {
    /// JWT validation failed.
    #[error("JWT validation failed: {0}")]
    JwtValidation(String),

    /// JWT encoding failed.
    #[error("JWT encoding failed: {0}")]
    JwtEncoding(String),

    /// JWT decoding failed.
    #[error("JWT decoding failed: {0}")]
    JwtDecoding(String),

    /// Token expired.
    #[error("Token expired")]
    TokenExpired,

    /// Invalid token.
    #[error("Invalid token")]
    InvalidToken,

    /// Invalid state.
    #[error("Invalid state: {0}")]
    InvalidState(String),

    /// OIDC error.
    #[error("OIDC error: {0}")]
    Oidc(String),

    /// HTTP request error.
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    /// Configuration error.
    #[error("Configuration error: {0}")]
    Configuration(String),

    /// User not found.
    #[error("User not found")]
    UserNotFound,

    /// Other error.
    #[error("{0}")]
    Other(String),
}

impl From<jsonwebtoken::errors::Error> for AuthError {
    fn from(e: jsonwebtoken::errors::Error) -> Self {
        match e.kind() {
            jsonwebtoken::errors::ErrorKind::ExpiredSignature => AuthError::TokenExpired,
            jsonwebtoken::errors::ErrorKind::InvalidToken => AuthError::InvalidToken,
            _ => AuthError::JwtValidation(e.to_string()),
        }
    }
}

/// Result type for authentication operations.
pub type AuthResult<T> = Result<T, AuthError>;
