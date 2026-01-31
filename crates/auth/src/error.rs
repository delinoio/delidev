//! Authentication error types

use thiserror::Error;

/// Errors that can occur during authentication
#[derive(Error, Debug)]
pub enum AuthError {
    /// JWT encoding error
    #[error("Failed to encode JWT: {0}")]
    JwtEncode(String),

    /// JWT decoding error
    #[error("Failed to decode JWT: {0}")]
    JwtDecode(String),

    /// Token expired
    #[error("Token expired")]
    TokenExpired,

    /// Token not yet valid
    #[error("Token not yet valid")]
    TokenNotYetValid,

    /// Invalid token signature
    #[error("Invalid token signature")]
    InvalidSignature,

    /// Invalid token claims
    #[error("Invalid token claims: {0}")]
    InvalidClaims(String),

    /// Missing required claim
    #[error("Missing required claim: {0}")]
    MissingClaim(String),

    /// OpenID Connect error
    #[error("OpenID Connect error: {0}")]
    Oidc(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    Configuration(String),

    /// Invalid credentials
    #[error("Invalid credentials")]
    InvalidCredentials,

    /// User not found
    #[error("User not found: {0}")]
    UserNotFound(String),
}

impl From<jsonwebtoken::errors::Error> for AuthError {
    fn from(err: jsonwebtoken::errors::Error) -> Self {
        use jsonwebtoken::errors::ErrorKind;

        match err.kind() {
            ErrorKind::ExpiredSignature => AuthError::TokenExpired,
            ErrorKind::ImmatureSignature => AuthError::TokenNotYetValid,
            ErrorKind::InvalidSignature => AuthError::InvalidSignature,
            _ => AuthError::JwtDecode(err.to_string()),
        }
    }
}

/// Result type for authentication operations
pub type AuthResult<T> = Result<T, AuthError>;
