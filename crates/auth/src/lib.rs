//! Authentication and authorization for DeliDev
//!
//! This crate provides JWT-based authentication and OpenID Connect integration
//! for the DeliDev server.
//!
//! ## Features
//!
//! - **JWT Authentication**: Create and verify JWT tokens for API authentication
//! - **OpenID Connect**: Integration with OIDC providers (Google, GitHub, Keycloak, etc.)
//! - **Role-Based Access Control**: User roles (User, Admin, Worker) for authorization
//!
//! ## Usage
//!
//! ### JWT Authentication
//!
//! ```rust
//! use auth::JwtAuth;
//!
//! // Create a JWT auth service
//! let auth = JwtAuth::new_hs256(b"your-secret-key");
//!
//! // Create a token
//! let token = auth.create_token("user-123", Some("user@example.com".to_string()), None).unwrap();
//!
//! // Verify and authenticate
//! let user = auth.authenticate(&token).unwrap();
//! ```
//!
//! ### OIDC Authentication
//!
//! ```rust,ignore
//! use auth::{OidcAuth, OidcConfig, AuthorizationState};
//!
//! // Create OIDC config
//! let config = OidcConfig::new(
//!     "https://accounts.google.com",
//!     "your-client-id",
//!     "your-client-secret",
//!     "https://your-app.com/callback",
//! );
//!
//! // Create OIDC client
//! let oidc = OidcAuth::new(config);
//!
//! // Generate authorization URL
//! let state = AuthorizationState::new();
//! let auth_url = oidc.authorization_url(&state)?;
//! ```

mod error;
mod jwt;
mod oidc;
mod state_store;
mod user;

pub use error::*;
pub use jwt::*;
pub use oidc::*;
pub use state_store::*;
pub use user::*;
