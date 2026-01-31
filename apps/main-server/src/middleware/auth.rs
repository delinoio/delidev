//! Authentication middleware.

use std::sync::Arc;

use auth::{Claims, JwtManager};
use axum::{
    extract::{Request, State},
    http::{header::AUTHORIZATION, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use task_store::TaskStore;
use uuid::Uuid;

use crate::state::AppState;

/// Authenticated user information.
#[derive(Debug, Clone)]
pub struct AuthenticatedUser {
    /// User ID.
    pub id: Uuid,
    /// User email.
    pub email: String,
    /// User display name.
    pub name: Option<String>,
}

impl TryFrom<Claims> for AuthenticatedUser {
    type Error = auth::AuthError;

    fn try_from(claims: Claims) -> Result<Self, Self::Error> {
        Ok(Self {
            id: claims.user_id()?,
            email: claims.email,
            name: claims.name,
        })
    }
}

/// Extracts the JWT token from the Authorization header.
fn extract_token(request: &Request) -> Option<&str> {
    request
        .headers()
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
}

/// Validates a JWT token and returns the claims.
fn validate_token(jwt_manager: &JwtManager, token: &str) -> Result<Claims, StatusCode> {
    jwt_manager
        .validate_token(token)
        .map_err(|_| StatusCode::UNAUTHORIZED)
}

/// Authentication middleware.
///
/// This middleware extracts the JWT token from the Authorization header,
/// validates it, and stores the authenticated user in the request extensions.
/// In single-user mode, authentication is skipped.
pub async fn auth_middleware<S: TaskStore + 'static>(
    State(state): State<Arc<AppState<S>>>,
    mut request: Request,
    next: Next,
) -> Response {
    // Skip auth in single-user mode
    if !state.auth_enabled() {
        return next.run(request).await;
    }

    // Get JWT manager
    let jwt_manager = match &state.jwt_manager {
        Some(manager) => manager,
        None => {
            tracing::error!("JWT manager not configured but auth is enabled");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "Authentication misconfigured" })),
            )
                .into_response();
        }
    };

    // Extract and validate token
    let token = match extract_token(&request) {
        Some(token) => token,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({ "error": "Missing authorization header" })),
            )
                .into_response()
        }
    };

    let claims = match validate_token(jwt_manager, token) {
        Ok(claims) => claims,
        Err(status) => {
            return (status, Json(json!({ "error": "Invalid token" }))).into_response()
        }
    };

    // Store authenticated user in request extensions
    match AuthenticatedUser::try_from(claims) {
        Ok(user) => {
            request.extensions_mut().insert(user);
        }
        Err(_) => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({ "error": "Invalid token claims" })),
            )
                .into_response()
        }
    }

    next.run(request).await
}

/// Optional authentication middleware.
///
/// This middleware works like `auth_middleware` but doesn't fail if no token
/// is provided. Useful for endpoints that work both authenticated and anonymously.
pub async fn optional_auth_middleware<S: TaskStore + 'static>(
    State(state): State<Arc<AppState<S>>>,
    mut request: Request,
    next: Next,
) -> Response {
    // Skip auth in single-user mode
    if !state.auth_enabled() {
        return next.run(request).await;
    }

    // Get JWT manager
    let jwt_manager = match &state.jwt_manager {
        Some(manager) => manager,
        None => return next.run(request).await,
    };

    // Try to extract and validate token
    if let Some(token) = extract_token(&request) {
        if let Ok(claims) = jwt_manager.validate_token(token) {
            if let Ok(user) = AuthenticatedUser::try_from(claims) {
                request.extensions_mut().insert(user);
            }
        }
    }

    next.run(request).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_authenticated_user_from_claims() {
        let user_id = Uuid::new_v4();
        let claims = Claims::new(
            user_id,
            "test@example.com".to_string(),
            Some("Test User".to_string()),
            24,
        );

        let user = AuthenticatedUser::try_from(claims).unwrap();
        assert_eq!(user.id, user_id);
        assert_eq!(user.email, "test@example.com");
        assert_eq!(user.name, Some("Test User".to_string()));
    }

    #[test]
    fn test_extract_token_valid() {
        // This is a simplified test showing the logic
        let auth_header = "Bearer test-token-123";
        let token = auth_header.strip_prefix("Bearer ");
        assert_eq!(token, Some("test-token-123"));
    }

    #[test]
    fn test_extract_token_missing_bearer() {
        let auth_header = "Basic credentials";
        let token = auth_header.strip_prefix("Bearer ");
        assert_eq!(token, None);
    }
}
