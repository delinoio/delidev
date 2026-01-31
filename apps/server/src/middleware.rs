//! Authentication middleware

#![allow(dead_code)]

use auth::{AuthenticatedUser, JwtAuth};
use axum::{
    extract::{Request, State},
    http::{header, StatusCode},
    middleware::Next,
    response::Response,
};
use tracing::debug;

use crate::state::AppState;

/// Authentication middleware
pub async fn auth_middleware(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Skip auth if not configured (single process mode)
    if state.auth.is_none() {
        request
            .extensions_mut()
            .insert::<Option<AuthenticatedUser>>(None);
        return Ok(next.run(request).await);
    }

    let auth = state.auth.as_ref().unwrap();

    // Extract token from Authorization header
    let token = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .and_then(|h| JwtAuth::extract_token(h));

    match token {
        Some(token) => match auth.authenticate(token) {
            Ok(user) => {
                debug!(user_id = %user.id, "Authenticated user");
                request.extensions_mut().insert(Some(user));
                Ok(next.run(request).await)
            }
            Err(e) => {
                debug!("Authentication failed: {}", e);
                Err(StatusCode::UNAUTHORIZED)
            }
        },
        None => {
            // No token provided
            debug!("No authentication token provided");
            Err(StatusCode::UNAUTHORIZED)
        }
    }
}

/// Extract authenticated user from request extensions
pub async fn extract_user(request: Request) -> Option<AuthenticatedUser> {
    request
        .extensions()
        .get::<Option<AuthenticatedUser>>()
        .cloned()
        .flatten()
}
