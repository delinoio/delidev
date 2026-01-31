//! JWT and OIDC authentication for DeliDev.
//!
//! This crate provides:
//! - JWT token generation and validation
//! - OIDC authentication flow with PKCE support
//! - Token refresh mechanism

mod error;
mod jwt;
mod oidc;
mod pkce;

pub use error::*;
pub use jwt::*;
pub use oidc::*;
pub use pkce::*;

/// Default JWT expiration time in hours.
pub const DEFAULT_JWT_EXPIRATION_HOURS: u64 = 24;

/// Default JWT issuer.
pub const DEFAULT_JWT_ISSUER: &str = "delidev";
