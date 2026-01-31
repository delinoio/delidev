//! Authentication API endpoints.

use std::sync::Arc;

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Redirect},
    Extension, Json,
};
use rpc_protocol::{requests::*, responses::*};
use serde::Deserialize;
use task_store::TaskStore;

use crate::error::{ServerError, ServerResult};
use crate::middleware::AuthenticatedUser;
use crate::state::AppState;

/// Query parameters for OIDC callback.
#[derive(Debug, Deserialize)]
pub struct CallbackQuery {
    pub code: String,
    pub state: String,
}

/// Gets the OIDC login URL.
pub async fn get_login_url<S: TaskStore>(
    State(state): State<Arc<AppState<S>>>,
    Json(request): Json<GetLoginUrlRequest>,
) -> ServerResult<Json<GetLoginUrlResponse>> {
    // In single-user mode, auth is disabled
    if !state.auth_enabled() {
        return Err(ServerError::InvalidRequest(
            "Authentication is disabled in single-user mode".to_string(),
        ));
    }

    if !state.config.oidc_configured() {
        return Err(ServerError::InvalidRequest(
            "OIDC is not configured".to_string(),
        ));
    }

    // Generate PKCE code verifier and challenge
    let pkce = auth::PkceChallenge::new();
    let _code_verifier = &pkce.verifier;
    let code_challenge = &pkce.challenge;

    // Generate state parameter for CSRF protection
    let auth_state = uuid::Uuid::new_v4().to_string();

    // TODO: Store state and code_verifier in database for validation during callback

    // Build authorization URL
    let issuer_url = state.config.oidc_issuer_url.as_ref().unwrap();
    let client_id = state.config.oidc_client_id.as_ref().unwrap();

    let login_url = format!(
        "{}/authorize?response_type=code&client_id={}&redirect_uri={}&scope=openid%20email%20profile&state={}&code_challenge={}&code_challenge_method=S256",
        issuer_url,
        urlencoding::encode(client_id),
        urlencoding::encode(&request.redirect_uri),
        auth_state,
        code_challenge
    );

    tracing::info!("Generated login URL for OIDC flow");

    Ok(Json(GetLoginUrlResponse { login_url }))
}

/// Handles the OIDC callback.
pub async fn handle_callback<S: TaskStore>(
    State(state): State<Arc<AppState<S>>>,
    Query(query): Query<CallbackQuery>,
) -> impl IntoResponse {
    // In single-user mode, auth is disabled
    if !state.auth_enabled() {
        return (StatusCode::BAD_REQUEST, "Authentication is disabled").into_response();
    }

    // TODO: Validate state parameter against stored value
    // TODO: Exchange authorization code for tokens using stored code_verifier
    // TODO: Validate ID token and extract user info
    // TODO: Create or update user in database
    // TODO: Generate JWT and redirect back to client

    tracing::info!(state = %query.state, "Processing OIDC callback");

    // For now, just redirect to a placeholder
    Redirect::to("/").into_response()
}

/// Refreshes the access token.
pub async fn refresh_token<S: TaskStore>(
    State(state): State<Arc<AppState<S>>>,
    Json(_request): Json<RefreshTokenRequest>,
) -> ServerResult<Json<RefreshTokenResponse>> {
    // In single-user mode, auth is disabled
    if !state.auth_enabled() {
        return Err(ServerError::InvalidRequest(
            "Authentication is disabled in single-user mode".to_string(),
        ));
    }

    let _jwt_manager = state
        .jwt_manager
        .as_ref()
        .ok_or_else(|| ServerError::Internal("JWT manager not configured".to_string()))?;

    // TODO: Validate refresh token and issue new tokens
    // For now, this is a placeholder

    tracing::info!("Token refresh requested");

    Err(ServerError::InvalidRequest(
        "Token refresh not yet implemented".to_string(),
    ))
}

/// Gets the current authenticated user.
pub async fn get_current_user<S: TaskStore>(
    State(state): State<Arc<AppState<S>>>,
    user: Option<Extension<AuthenticatedUser>>,
) -> ServerResult<Json<GetCurrentUserResponse>> {
    // In single-user mode, return a placeholder user
    if !state.auth_enabled() {
        return Ok(Json(GetCurrentUserResponse {
            user: rpc_protocol::User {
                id: "local".to_string(),
                email: "local@localhost".to_string(),
                name: Some("Local User".to_string()),
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            },
        }));
    }

    let user = user
        .ok_or(ServerError::AuthenticationRequired)?
        .0;

    // Get user from database
    let db_user = state
        .store
        .get_user(user.id)
        .await?
        .ok_or_else(|| ServerError::NotFound("User not found".to_string()))?;

    Ok(Json(GetCurrentUserResponse {
        user: rpc_protocol::User {
            id: db_user.id.to_string(),
            email: db_user.email,
            name: db_user.name,
            created_at: db_user.created_at,
            updated_at: db_user.updated_at,
        },
    }))
}

/// Logs out the current user.
pub async fn logout<S: TaskStore>(
    State(state): State<Arc<AppState<S>>>,
) -> ServerResult<Json<LogoutResponse>> {
    // In single-user mode, auth is disabled
    if !state.auth_enabled() {
        return Ok(Json(LogoutResponse {}));
    }

    // TODO: Invalidate the current token (add to blocklist)

    tracing::info!("User logged out");

    Ok(Json(LogoutResponse {}))
}
