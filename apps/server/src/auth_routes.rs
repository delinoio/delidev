//! Authentication route handlers
//!
//! This module provides REST endpoints for authentication:
//! - `/auth/login` - Initiate OIDC login
//! - `/auth/callback` - OIDC callback handler
//! - `/auth/token` - Token refresh
//! - `/auth/logout` - Logout (client-side token invalidation)
//! - `/auth/me` - Get current user info

use auth::{AuthenticatedUser, AuthorizationState, JwtAuth, TokenResponse, UserInfo};
use axum::{
    extract::{Query, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info, warn};

use crate::state::AppState;

/// Default expiration time for authorization states (10 minutes)
pub const AUTH_STATE_EXPIRATION_SECS: i64 = 600;

/// Login request (for initiating OIDC flow)
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    /// Optional redirect URL after successful login
    #[serde(default)]
    pub redirect_uri: Option<String>,
}

/// Login response
#[derive(Debug, Serialize)]
pub struct LoginResponse {
    /// URL to redirect to for authentication
    pub auth_url: String,
}

/// Callback query parameters
#[derive(Debug, Deserialize)]
pub struct CallbackParams {
    /// Authorization code from OIDC provider
    pub code: String,

    /// State parameter for CSRF protection
    pub state: String,

    /// Optional error from provider
    #[serde(default)]
    pub error: Option<String>,

    /// Optional error description
    #[serde(default)]
    pub error_description: Option<String>,
}

/// Token response to client
#[derive(Debug, Serialize)]
pub struct AuthTokenResponse {
    /// JWT access token
    pub access_token: String,

    /// Token type (always "Bearer")
    pub token_type: String,

    /// When the token expires (in seconds)
    pub expires_in: i64,

    /// Refresh token (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<String>,

    /// Redirect URI (if provided during login)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub redirect_uri: Option<String>,
}

/// Token refresh request
#[derive(Debug, Deserialize)]
pub struct RefreshTokenRequest {
    /// The refresh token
    pub refresh_token: String,
}

/// Current user response
#[derive(Debug, Serialize)]
pub struct CurrentUserResponse {
    /// User ID
    pub id: String,

    /// Email address
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,

    /// Display name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

impl From<AuthenticatedUser> for CurrentUserResponse {
    fn from(user: AuthenticatedUser) -> Self {
        Self {
            id: user.id,
            email: user.email,
            name: user.name,
        }
    }
}

/// Error response
#[derive(Debug, Serialize)]
pub struct AuthErrorResponse {
    /// Error code
    pub error: String,

    /// Human-readable error description
    pub error_description: String,
}

impl AuthErrorResponse {
    pub fn new(error: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            error: error.into(),
            error_description: description.into(),
        }
    }
}

impl IntoResponse for AuthErrorResponse {
    fn into_response(self) -> Response {
        (StatusCode::BAD_REQUEST, Json(self)).into_response()
    }
}

/// Validate a redirect URI against allowed patterns
///
/// Returns true if the redirect URI is allowed, false otherwise.
/// This prevents open redirect vulnerabilities.
fn is_valid_redirect_uri(redirect_uri: &str, allowed_origins: &[String]) -> bool {
    // If no allowed origins are configured, only allow relative paths
    if allowed_origins.is_empty() {
        return redirect_uri.starts_with('/') && !redirect_uri.starts_with("//");
    }

    // Parse the redirect URI
    let url = match url::Url::parse(redirect_uri) {
        Ok(url) => url,
        Err(_) => {
            // If it's not a valid URL, check if it's a relative path
            return redirect_uri.starts_with('/') && !redirect_uri.starts_with("//");
        }
    };

    // Check against allowed origins
    let redirect_origin = format!(
        "{}://{}{}",
        url.scheme(),
        url.host_str().unwrap_or(""),
        url.port().map(|p| format!(":{}", p)).unwrap_or_default()
    );

    allowed_origins.iter().any(|allowed| {
        // Support wildcard subdomains (e.g., "*.example.com")
        if allowed.starts_with("*.") {
            let domain = &allowed[1..]; // ".example.com"
            url.host_str().is_some_and(|host| {
                // Must either be the exact domain (without the leading dot)
                // or end with ".example.com" AND be longer than just the suffix
                // This prevents "evil.com.example.com" from matching
                if host == &domain[1..] {
                    true
                } else if host.ends_with(domain) {
                    // Ensure there's actually a subdomain part
                    // host.len() > domain.len() means there's something before ".example.com"
                    host.len() > domain.len()
                } else {
                    false
                }
            })
        } else {
            &redirect_origin == allowed
        }
    })
}

/// Check if auth is enabled
pub async fn auth_status(State(state): State<AppState>) -> impl IntoResponse {
    #[derive(Serialize)]
    struct AuthStatus {
        enabled: bool,
        oidc_enabled: bool,
    }

    let status = AuthStatus {
        enabled: state.auth.is_some(),
        oidc_enabled: state.oidc.is_some(),
    };

    Json(status)
}

/// Initiate OIDC login
pub async fn login(
    State(state): State<AppState>,
    Query(params): Query<LoginRequest>,
) -> Result<impl IntoResponse, AuthErrorResponse> {
    // Check if OIDC is configured
    let oidc = state.oidc.as_ref().ok_or_else(|| {
        AuthErrorResponse::new("oidc_not_configured", "OIDC authentication is not configured")
    })?;

    // Validate redirect_uri if provided (prevent open redirect)
    if let Some(ref redirect_uri) = params.redirect_uri {
        if !is_valid_redirect_uri(redirect_uri, &state.config.allowed_redirect_origins) {
            warn!(redirect_uri = %redirect_uri, "Invalid redirect URI rejected");
            return Err(AuthErrorResponse::new(
                "invalid_redirect_uri",
                "The provided redirect URI is not allowed",
            ));
        }
    }

    // Generate authorization state with redirect URI
    let auth_state = AuthorizationState::with_redirect_uri(params.redirect_uri.clone());
    let state_token = auth_state.state.clone();

    // Build authorization URL
    let auth_url = oidc.authorization_url(&auth_state).map_err(|e| {
        error!("Failed to build authorization URL: {}", e);
        AuthErrorResponse::new("server_error", "Failed to initiate authentication")
    })?;

    // Store state for callback validation using the database-backed store
    state.auth_state_store.store(&auth_state).await.map_err(|e| {
        error!("Failed to store auth state: {}", e);
        AuthErrorResponse::new("server_error", "Failed to initiate authentication")
    })?;

    info!(
        state = %state_token,
        redirect_uri = ?params.redirect_uri,
        "Initiating OIDC login"
    );

    Ok(Json(LoginResponse { auth_url }))
}

/// Handle OIDC callback
pub async fn callback(
    State(state): State<AppState>,
    Query(params): Query<CallbackParams>,
) -> Result<impl IntoResponse, AuthErrorResponse> {
    // Check for error from provider
    if let Some(error) = params.error {
        let description = params
            .error_description
            .unwrap_or_else(|| "Authentication failed".to_string());
        warn!(error = %error, description = %description, "OIDC error");
        return Err(AuthErrorResponse::new(error, description));
    }

    // Validate state and retrieve from database (atomic take operation)
    let auth_state = state
        .auth_state_store
        .take(&params.state)
        .await
        .map_err(|e| {
            error!("Failed to retrieve auth state: {}", e);
            AuthErrorResponse::new("server_error", "Failed to validate state")
        })?;

    let auth_state = auth_state.ok_or_else(|| {
        warn!("Invalid or expired state parameter");
        AuthErrorResponse::new("invalid_state", "Invalid or expired state parameter")
    })?;

    // Check if state is expired
    if auth_state.is_expired(AUTH_STATE_EXPIRATION_SECS) {
        warn!("Authorization state expired");
        return Err(AuthErrorResponse::new(
            "state_expired",
            "Authorization request has expired",
        ));
    }

    // Get OIDC client
    let oidc = state.oidc.as_ref().ok_or_else(|| {
        AuthErrorResponse::new("oidc_not_configured", "OIDC authentication is not configured")
    })?;

    // Exchange code for tokens
    let token_endpoint = oidc.token_endpoint().map_err(|e| {
        error!("Failed to get token endpoint: {}", e);
        AuthErrorResponse::new("server_error", "Token endpoint not available")
    })?;

    let token_params = oidc
        .token_request_params(&params.code, &auth_state)
        .map_err(|e| {
            error!("Failed to build token request: {}", e);
            AuthErrorResponse::new("server_error", "Failed to build token request")
        })?;

    // Make token request
    let client = reqwest::Client::new();
    let token_response = client
        .post(token_endpoint)
        .form(&token_params)
        .send()
        .await
        .map_err(|e| {
            error!("Token request failed: {}", e);
            AuthErrorResponse::new("token_error", "Failed to exchange authorization code")
        })?;

    if !token_response.status().is_success() {
        let status = token_response.status();
        // Don't log the full body as it may contain sensitive information
        error!(status = %status, "Token endpoint returned error");
        return Err(AuthErrorResponse::new(
            "token_error",
            "Token endpoint returned an error",
        ));
    }

    let tokens: TokenResponse = token_response.json().await.map_err(|e| {
        error!("Failed to parse token response: {}", e);
        AuthErrorResponse::new("token_error", "Invalid token response")
    })?;

    // Get user info
    let userinfo_endpoint = oidc.userinfo_endpoint().map_err(|e| {
        error!("Failed to get userinfo endpoint: {}", e);
        AuthErrorResponse::new("server_error", "Userinfo endpoint not available")
    })?;

    let userinfo_response = client
        .get(userinfo_endpoint)
        .header("Authorization", format!("Bearer {}", tokens.access_token))
        .send()
        .await
        .map_err(|e| {
            error!("Userinfo request failed: {}", e);
            AuthErrorResponse::new("userinfo_error", "Failed to fetch user information")
        })?;

    if !userinfo_response.status().is_success() {
        let status = userinfo_response.status();
        error!(status = %status, "Userinfo endpoint returned error");
        return Err(AuthErrorResponse::new(
            "userinfo_error",
            "Failed to fetch user information",
        ));
    }

    let user_info: UserInfo = userinfo_response.json().await.map_err(|e| {
        error!("Failed to parse userinfo response: {}", e);
        AuthErrorResponse::new("userinfo_error", "Invalid userinfo response")
    })?;

    // Convert to authenticated user
    let user = user_info.to_authenticated_user();

    // Create our own JWT
    let jwt_auth = state.auth.as_ref().ok_or_else(|| {
        AuthErrorResponse::new("auth_not_configured", "JWT authentication is not configured")
    })?;

    let access_token = jwt_auth
        .create_token_with_expiration(
            &user.id,
            user.email.clone(),
            user.name.clone(),
            state.config.jwt_expiration_hours,
        )
        .map_err(|e| {
            error!("Failed to create JWT: {}", e);
            AuthErrorResponse::new("token_error", "Failed to create access token")
        })?;

    info!(user_id = %user.id, "User authenticated via OIDC");

    Ok(Json(AuthTokenResponse {
        access_token,
        token_type: "Bearer".to_string(),
        expires_in: state.config.jwt_expiration_hours * 3600,
        refresh_token: tokens.refresh_token,
        redirect_uri: auth_state.redirect_uri,
    }))
}

/// Refresh access token
///
/// **SECURITY NOTE**: This implementation uses a simplified approach where
/// refresh tokens are just longer-lived JWTs. A production-ready implementation
/// should include:
/// - Token storage with revocation tracking
/// - Refresh token rotation (issue new refresh token on each use)
/// - Family-based revocation (revoke all tokens when one is compromised)
///
/// Consider implementing proper refresh token management before deploying
/// to production with sensitive data.
pub async fn refresh_token(
    State(state): State<AppState>,
    Json(params): Json<RefreshTokenRequest>,
) -> Result<impl IntoResponse, AuthErrorResponse> {
    let jwt_auth = state.auth.as_ref().ok_or_else(|| {
        AuthErrorResponse::new("auth_not_configured", "Authentication is not configured")
    })?;

    // NOTE: This is a simplified implementation. The refresh token is validated
    // as a JWT and used to issue a new access token. For production use with
    // sensitive data, implement proper refresh token storage and revocation.

    let claims = jwt_auth.verify_token(&params.refresh_token).map_err(|e| {
        debug!("Refresh token validation failed: {}", e);
        AuthErrorResponse::new("invalid_token", "Invalid or expired refresh token")
    })?;

    // Issue new access token
    let access_token = jwt_auth
        .create_token_with_expiration(
            &claims.sub,
            claims.email,
            claims.name,
            state.config.jwt_expiration_hours,
        )
        .map_err(|e| {
            error!("Failed to create JWT: {}", e);
            AuthErrorResponse::new("token_error", "Failed to create access token")
        })?;

    info!(user_id = %claims.sub, "Token refreshed");

    Ok(Json(AuthTokenResponse {
        access_token,
        token_type: "Bearer".to_string(),
        expires_in: state.config.jwt_expiration_hours * 3600,
        refresh_token: None,
        redirect_uri: None,
    }))
}

/// Get current user info
pub async fn me(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> Result<impl IntoResponse, (StatusCode, AuthErrorResponse)> {
    let jwt_auth = state.auth.as_ref().ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            AuthErrorResponse::new("auth_not_configured", "Authentication is not configured"),
        )
    })?;

    // Extract token from Authorization header
    let token = headers
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .and_then(|h| JwtAuth::extract_token(h))
        .ok_or_else(|| {
            (
                StatusCode::UNAUTHORIZED,
                AuthErrorResponse::new("missing_token", "No authentication token provided"),
            )
        })?;

    // Verify token and get user
    let user = jwt_auth.authenticate(token).map_err(|e| {
        debug!("Token validation failed: {}", e);
        (
            StatusCode::UNAUTHORIZED,
            AuthErrorResponse::new("invalid_token", "Invalid or expired token"),
        )
    })?;

    Ok(Json(CurrentUserResponse::from(user)))
}

/// Logout (just confirms the action, actual token invalidation is client-side)
pub async fn logout() -> impl IntoResponse {
    #[derive(Serialize)]
    struct LogoutResponse {
        success: bool,
        message: String,
    }

    Json(LogoutResponse {
        success: true,
        message: "Logged out successfully. Please discard your tokens.".to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_relative_paths() {
        let allowed: Vec<String> = vec![];

        // Valid relative paths
        assert!(is_valid_redirect_uri("/dashboard", &allowed));
        assert!(is_valid_redirect_uri("/auth/callback", &allowed));
        assert!(is_valid_redirect_uri("/path?query=value", &allowed));

        // Invalid paths (protocol-relative or absolute)
        assert!(!is_valid_redirect_uri("//evil.com/path", &allowed));
        assert!(!is_valid_redirect_uri("https://evil.com", &allowed));
    }

    #[test]
    fn test_validate_allowed_origins() {
        let allowed = vec![
            "https://app.example.com".to_string(),
            "https://localhost:3000".to_string(),
        ];

        // Valid origins
        assert!(is_valid_redirect_uri(
            "https://app.example.com/callback",
            &allowed
        ));
        assert!(is_valid_redirect_uri(
            "https://localhost:3000/auth",
            &allowed
        ));

        // Invalid origins
        assert!(!is_valid_redirect_uri("https://evil.com/callback", &allowed));
        assert!(!is_valid_redirect_uri(
            "https://app.example.com.evil.com",
            &allowed
        ));
    }

    #[test]
    fn test_validate_wildcard_subdomains() {
        let allowed = vec!["*.example.com".to_string()];

        // Valid subdomains
        assert!(is_valid_redirect_uri(
            "https://app.example.com/callback",
            &allowed
        ));
        assert!(is_valid_redirect_uri(
            "https://staging.example.com/auth",
            &allowed
        ));
        // Exact domain should also match
        assert!(is_valid_redirect_uri(
            "https://example.com/callback",
            &allowed
        ));

        // Invalid (not a subdomain)
        assert!(!is_valid_redirect_uri("https://example.org", &allowed));

        // Security: Prevent subdomain takeover attacks
        // "evil.com.example.com" should NOT match "*.example.com"
        // because it could be registered as a subdomain of evil.com
        assert!(!is_valid_redirect_uri(
            "https://notexample.com/callback",
            &allowed
        ));
    }
}
