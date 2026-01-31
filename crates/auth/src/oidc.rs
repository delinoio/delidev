//! OpenID Connect (OIDC) integration
//!
//! This module provides OIDC authentication support for DeliDev server.
//! It supports standard OIDC providers like Google, GitHub, Keycloak, etc.

use serde::{Deserialize, Serialize};

use crate::{AuthError, AuthResult, AuthenticatedUser};

/// OpenID Connect configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OidcConfig {
    /// The OIDC provider's issuer URL (e.g., "https://accounts.google.com")
    pub issuer_url: String,

    /// OAuth2 client ID
    pub client_id: String,

    /// OAuth2 client secret
    pub client_secret: String,

    /// Redirect URL after authentication
    pub redirect_url: String,

    /// Scopes to request (defaults to ["openid", "email", "profile"])
    #[serde(default = "default_scopes")]
    pub scopes: Vec<String>,
}

fn default_scopes() -> Vec<String> {
    vec![
        "openid".to_string(),
        "email".to_string(),
        "profile".to_string(),
    ]
}

impl OidcConfig {
    /// Create a new OIDC configuration
    pub fn new(
        issuer_url: impl Into<String>,
        client_id: impl Into<String>,
        client_secret: impl Into<String>,
        redirect_url: impl Into<String>,
    ) -> Self {
        Self {
            issuer_url: issuer_url.into(),
            client_id: client_id.into(),
            client_secret: client_secret.into(),
            redirect_url: redirect_url.into(),
            scopes: default_scopes(),
        }
    }

    /// Set custom scopes
    pub fn with_scopes(mut self, scopes: Vec<String>) -> Self {
        self.scopes = scopes;
        self
    }

    /// Load from environment variables
    pub fn from_env() -> AuthResult<Self> {
        let issuer_url = std::env::var("DELIDEV_OIDC_ISSUER_URL")
            .map_err(|_| AuthError::Configuration("DELIDEV_OIDC_ISSUER_URL not set".to_string()))?;

        let client_id = std::env::var("DELIDEV_OIDC_CLIENT_ID")
            .map_err(|_| AuthError::Configuration("DELIDEV_OIDC_CLIENT_ID not set".to_string()))?;

        let client_secret = std::env::var("DELIDEV_OIDC_CLIENT_SECRET").map_err(|_| {
            AuthError::Configuration("DELIDEV_OIDC_CLIENT_SECRET not set".to_string())
        })?;

        let redirect_url = std::env::var("DELIDEV_OIDC_REDIRECT_URL").map_err(|_| {
            AuthError::Configuration("DELIDEV_OIDC_REDIRECT_URL not set".to_string())
        })?;

        let scopes = std::env::var("DELIDEV_OIDC_SCOPES")
            .map(|s| s.split(',').map(|s| s.trim().to_string()).collect())
            .unwrap_or_else(|_| default_scopes());

        Ok(Self {
            issuer_url,
            client_id,
            client_secret,
            redirect_url,
            scopes,
        })
    }
}

/// OIDC provider metadata (subset of fields we need)
#[derive(Debug, Clone, Deserialize)]
pub struct OidcProviderMetadata {
    /// The issuer identifier
    pub issuer: String,

    /// URL of the authorization endpoint
    pub authorization_endpoint: String,

    /// URL of the token endpoint
    pub token_endpoint: String,

    /// URL of the userinfo endpoint
    #[serde(default)]
    pub userinfo_endpoint: Option<String>,

    /// URL of the JWKS endpoint
    pub jwks_uri: String,

    /// Supported scopes
    #[serde(default)]
    pub scopes_supported: Vec<String>,

    /// Supported response types
    #[serde(default)]
    pub response_types_supported: Vec<String>,

    /// Supported grant types
    #[serde(default)]
    pub grant_types_supported: Vec<String>,
}

/// Token response from the OIDC provider
#[derive(Debug, Clone, Deserialize)]
pub struct TokenResponse {
    /// The access token
    pub access_token: String,

    /// Token type (usually "Bearer")
    pub token_type: String,

    /// When the token expires (in seconds)
    #[serde(default)]
    pub expires_in: Option<u64>,

    /// The refresh token (if granted)
    #[serde(default)]
    pub refresh_token: Option<String>,

    /// The ID token (JWT containing user claims)
    #[serde(default)]
    pub id_token: Option<String>,

    /// Scopes granted
    #[serde(default)]
    pub scope: Option<String>,
}

/// User info from the OIDC provider
#[derive(Debug, Clone, Deserialize)]
pub struct UserInfo {
    /// Subject identifier (unique user ID)
    pub sub: String,

    /// User's email address
    #[serde(default)]
    pub email: Option<String>,

    /// Whether the email is verified
    #[serde(default)]
    pub email_verified: Option<bool>,

    /// User's name
    #[serde(default)]
    pub name: Option<String>,

    /// User's given name (first name)
    #[serde(default)]
    pub given_name: Option<String>,

    /// User's family name (last name)
    #[serde(default)]
    pub family_name: Option<String>,

    /// User's preferred username
    #[serde(default)]
    pub preferred_username: Option<String>,

    /// URL to user's profile picture
    #[serde(default)]
    pub picture: Option<String>,
}

impl UserInfo {
    /// Convert to an authenticated user
    pub fn to_authenticated_user(&self) -> AuthenticatedUser {
        AuthenticatedUser {
            id: self.sub.clone(),
            email: self.email.clone(),
            name: self.name.clone().or_else(|| {
                // Fall back to preferred_username or given_name
                self.preferred_username.clone().or_else(|| {
                    self.given_name.as_ref().map(|given| {
                        if let Some(family) = &self.family_name {
                            format!("{} {}", given, family)
                        } else {
                            given.clone()
                        }
                    })
                })
            }),
        }
    }
}

/// Authorization state for CSRF protection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorizationState {
    /// CSRF token
    pub state: String,

    /// Nonce for ID token validation
    pub nonce: String,

    /// PKCE code verifier (optional)
    #[serde(default)]
    pub code_verifier: Option<String>,

    /// Timestamp when this state was created
    pub created_at: i64,
}

impl AuthorizationState {
    /// Create new authorization state
    pub fn new() -> Self {
        Self {
            state: generate_random_string(32),
            nonce: generate_random_string(32),
            code_verifier: Some(generate_random_string(64)),
            created_at: chrono::Utc::now().timestamp(),
        }
    }

    /// Check if this state has expired (default: 10 minutes)
    pub fn is_expired(&self, max_age_secs: i64) -> bool {
        let now = chrono::Utc::now().timestamp();
        now - self.created_at > max_age_secs
    }

    /// Generate PKCE code challenge (S256)
    pub fn code_challenge(&self) -> Option<String> {
        self.code_verifier.as_ref().map(|verifier| {
            use sha2::{Digest, Sha256};
            let mut hasher = Sha256::new();
            hasher.update(verifier.as_bytes());
            let hash = hasher.finalize();
            base64_url_encode(&hash)
        })
    }
}

impl Default for AuthorizationState {
    fn default() -> Self {
        Self::new()
    }
}

/// Generate a random string for state/nonce
fn generate_random_string(len: usize) -> String {
    use rand::Rng;
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    let mut rng = rand::rng();
    (0..len)
        .map(|_| {
            let idx = rng.random_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}

/// Base64 URL encode (no padding)
fn base64_url_encode(data: &[u8]) -> String {
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
    URL_SAFE_NO_PAD.encode(data)
}

/// OIDC authentication client
///
/// This is a lightweight OIDC client that supports the authorization code flow.
/// For production use, it's recommended to use a full OIDC library.
#[derive(Debug, Clone)]
pub struct OidcAuth {
    config: OidcConfig,
    metadata: Option<OidcProviderMetadata>,
}

impl OidcAuth {
    /// Create a new OIDC auth client
    pub fn new(config: OidcConfig) -> Self {
        Self {
            config,
            metadata: None,
        }
    }

    /// Create from environment variables
    pub fn from_env() -> AuthResult<Self> {
        Ok(Self::new(OidcConfig::from_env()?))
    }

    /// Get the discovery URL for the OIDC provider
    pub fn discovery_url(&self) -> String {
        format!(
            "{}/.well-known/openid-configuration",
            self.config.issuer_url.trim_end_matches('/')
        )
    }

    /// Set the provider metadata (call after discovering)
    pub fn with_metadata(mut self, metadata: OidcProviderMetadata) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// Get the current configuration
    pub fn config(&self) -> &OidcConfig {
        &self.config
    }

    /// Get the cached metadata
    pub fn metadata(&self) -> Option<&OidcProviderMetadata> {
        self.metadata.as_ref()
    }

    /// Build the authorization URL
    ///
    /// Returns the URL and the authorization state that should be stored
    /// for validation when the callback is received.
    pub fn authorization_url(&self, state: &AuthorizationState) -> AuthResult<String> {
        let metadata = self
            .metadata
            .as_ref()
            .ok_or_else(|| AuthError::Oidc("Provider metadata not loaded".to_string()))?;

        let mut url = url::Url::parse(&metadata.authorization_endpoint)
            .map_err(|e| AuthError::Oidc(format!("Invalid authorization endpoint: {}", e)))?;

        {
            let mut query = url.query_pairs_mut();
            query.append_pair("response_type", "code");
            query.append_pair("client_id", &self.config.client_id);
            query.append_pair("redirect_uri", &self.config.redirect_url);
            query.append_pair("scope", &self.config.scopes.join(" "));
            query.append_pair("state", &state.state);
            query.append_pair("nonce", &state.nonce);

            // Add PKCE if available
            if let Some(challenge) = state.code_challenge() {
                query.append_pair("code_challenge", &challenge);
                query.append_pair("code_challenge_method", "S256");
            }
        }

        Ok(url.to_string())
    }

    /// Build token request parameters
    ///
    /// Returns the parameters that should be sent to the token endpoint.
    pub fn token_request_params(
        &self,
        code: &str,
        state: &AuthorizationState,
    ) -> AuthResult<Vec<(String, String)>> {
        let mut params = vec![
            ("grant_type".to_string(), "authorization_code".to_string()),
            ("code".to_string(), code.to_string()),
            ("redirect_uri".to_string(), self.config.redirect_url.clone()),
            ("client_id".to_string(), self.config.client_id.clone()),
            (
                "client_secret".to_string(),
                self.config.client_secret.clone(),
            ),
        ];

        // Add PKCE code verifier if used
        if let Some(ref verifier) = state.code_verifier {
            params.push(("code_verifier".to_string(), verifier.clone()));
        }

        Ok(params)
    }

    /// Get the token endpoint URL
    pub fn token_endpoint(&self) -> AuthResult<&str> {
        self.metadata
            .as_ref()
            .map(|m| m.token_endpoint.as_str())
            .ok_or_else(|| AuthError::Oidc("Provider metadata not loaded".to_string()))
    }

    /// Get the userinfo endpoint URL
    pub fn userinfo_endpoint(&self) -> AuthResult<&str> {
        self.metadata
            .as_ref()
            .and_then(|m| m.userinfo_endpoint.as_deref())
            .ok_or_else(|| AuthError::Oidc("Userinfo endpoint not available".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_oidc_config_new() {
        let config = OidcConfig::new(
            "https://accounts.example.com",
            "client-123",
            "secret-456",
            "https://app.example.com/callback",
        );

        assert_eq!(config.issuer_url, "https://accounts.example.com");
        assert_eq!(config.client_id, "client-123");
        assert_eq!(config.scopes, vec!["openid", "email", "profile"]);
    }

    #[test]
    fn test_authorization_state() {
        let state = AuthorizationState::new();

        assert!(!state.state.is_empty());
        assert!(!state.nonce.is_empty());
        assert!(state.code_verifier.is_some());
        assert!(!state.is_expired(600)); // Not expired within 10 minutes
    }

    #[test]
    fn test_authorization_state_expired() {
        let mut state = AuthorizationState::new();
        state.created_at = chrono::Utc::now().timestamp() - 1000; // 1000 seconds ago

        assert!(state.is_expired(600)); // Expired after 10 minutes
    }

    #[test]
    fn test_user_info_to_authenticated_user() {
        let user_info = UserInfo {
            sub: "user-123".to_string(),
            email: Some("test@example.com".to_string()),
            email_verified: Some(true),
            name: Some("Test User".to_string()),
            given_name: None,
            family_name: None,
            preferred_username: None,
            picture: None,
        };

        let auth_user = user_info.to_authenticated_user();
        assert_eq!(auth_user.id, "user-123");
        assert_eq!(auth_user.email, Some("test@example.com".to_string()));
        assert_eq!(auth_user.name, Some("Test User".to_string()));
    }

    #[test]
    fn test_user_info_fallback_name() {
        let user_info = UserInfo {
            sub: "user-456".to_string(),
            email: Some("test@example.com".to_string()),
            email_verified: None,
            name: None,
            given_name: Some("John".to_string()),
            family_name: Some("Doe".to_string()),
            preferred_username: None,
            picture: None,
        };

        let auth_user = user_info.to_authenticated_user();
        assert_eq!(auth_user.name, Some("John Doe".to_string()));
    }

    #[test]
    fn test_pkce_code_challenge() {
        let state = AuthorizationState::new();
        let challenge = state.code_challenge();

        assert!(challenge.is_some());
        // Challenge should be base64url encoded SHA256 hash (43 chars without padding)
        assert_eq!(challenge.unwrap().len(), 43);
    }
}
