//! OIDC (OpenID Connect) authentication flow.

use serde::{Deserialize, Serialize};
use url::Url;
use uuid::Uuid;

use crate::{AuthError, AuthResult, PkceChallenge};

/// OIDC configuration.
#[derive(Debug, Clone)]
pub struct OidcConfig {
    /// OIDC provider issuer URL.
    pub issuer_url: String,
    /// OAuth2 client ID.
    pub client_id: String,
    /// OAuth2 client secret.
    pub client_secret: String,
    /// Redirect URL after authentication.
    pub redirect_url: String,
    /// OAuth2 scopes.
    pub scopes: Vec<String>,
}

impl OidcConfig {
    /// Creates a new OIDC configuration.
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
            scopes: vec!["openid".to_string(), "email".to_string(), "profile".to_string()],
        }
    }

    /// Sets the scopes.
    pub fn with_scopes(mut self, scopes: Vec<String>) -> Self {
        self.scopes = scopes;
        self
    }
}

/// OIDC provider discovery document.
#[derive(Debug, Clone, Deserialize)]
pub struct OidcDiscovery {
    /// Authorization endpoint.
    pub authorization_endpoint: String,
    /// Token endpoint.
    pub token_endpoint: String,
    /// User info endpoint.
    pub userinfo_endpoint: String,
    /// JWKS URI.
    pub jwks_uri: String,
    /// Issuer.
    pub issuer: String,
}

/// OIDC authentication state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthState {
    /// Random state parameter.
    pub state: String,
    /// PKCE code verifier.
    pub code_verifier: String,
    /// Redirect URI.
    pub redirect_uri: String,
    /// When this state was created.
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// When this state expires.
    pub expires_at: chrono::DateTime<chrono::Utc>,
}

impl AuthState {
    /// Creates a new authentication state.
    pub fn new(redirect_uri: impl Into<String>) -> Self {
        let pkce = PkceChallenge::new();
        let now = chrono::Utc::now();

        Self {
            state: Uuid::new_v4().to_string(),
            code_verifier: pkce.verifier,
            redirect_uri: redirect_uri.into(),
            created_at: now,
            expires_at: now + chrono::Duration::minutes(10),
        }
    }

    /// Returns true if this state is expired.
    pub fn is_expired(&self) -> bool {
        chrono::Utc::now() > self.expires_at
    }
}

/// OIDC token response.
#[derive(Debug, Clone, Deserialize)]
pub struct TokenResponse {
    /// Access token.
    pub access_token: String,
    /// Token type (usually "Bearer").
    pub token_type: String,
    /// ID token (JWT).
    pub id_token: Option<String>,
    /// Refresh token.
    pub refresh_token: Option<String>,
    /// Token expiration in seconds.
    pub expires_in: Option<u64>,
}

/// OIDC user info response.
#[derive(Debug, Clone, Deserialize)]
pub struct UserInfo {
    /// Subject (user ID from provider).
    pub sub: String,
    /// Email address.
    pub email: Option<String>,
    /// Whether email is verified.
    pub email_verified: Option<bool>,
    /// Full name.
    pub name: Option<String>,
    /// Given name.
    pub given_name: Option<String>,
    /// Family name.
    pub family_name: Option<String>,
    /// Profile picture URL.
    pub picture: Option<String>,
}

/// OIDC client for handling authentication flows.
#[derive(Debug, Clone)]
pub struct OidcClient {
    config: OidcConfig,
    http_client: reqwest::Client,
    discovery: Option<OidcDiscovery>,
}

impl OidcClient {
    /// Creates a new OIDC client.
    pub fn new(config: OidcConfig) -> Self {
        Self {
            config,
            http_client: reqwest::Client::new(),
            discovery: None,
        }
    }

    /// Discovers the OIDC provider configuration.
    pub async fn discover(&mut self) -> AuthResult<OidcDiscovery> {
        if let Some(ref discovery) = self.discovery {
            return Ok(discovery.clone());
        }

        let discovery_url = format!(
            "{}/.well-known/openid-configuration",
            self.config.issuer_url.trim_end_matches('/')
        );

        let discovery: OidcDiscovery = self
            .http_client
            .get(&discovery_url)
            .send()
            .await?
            .json()
            .await?;

        self.discovery = Some(discovery.clone());
        Ok(discovery)
    }

    /// Generates the authorization URL for the login flow.
    pub async fn get_authorization_url(&mut self, state: &AuthState) -> AuthResult<String> {
        let discovery = self.discover().await?;
        let pkce = PkceChallenge::from_verifier(&state.code_verifier);

        let mut url = Url::parse(&discovery.authorization_endpoint)
            .map_err(|e| AuthError::Configuration(e.to_string()))?;

        url.query_pairs_mut()
            .append_pair("client_id", &self.config.client_id)
            .append_pair("redirect_uri", &state.redirect_uri)
            .append_pair("response_type", "code")
            .append_pair("scope", &self.config.scopes.join(" "))
            .append_pair("state", &state.state)
            .append_pair("code_challenge", &pkce.challenge)
            .append_pair("code_challenge_method", pkce.method.as_str());

        Ok(url.to_string())
    }

    /// Exchanges an authorization code for tokens.
    pub async fn exchange_code(
        &mut self,
        code: &str,
        state: &AuthState,
    ) -> AuthResult<TokenResponse> {
        let discovery = self.discover().await?;

        let client_id = self.config.client_id.clone();
        let client_secret = self.config.client_secret.clone();
        let redirect_uri = state.redirect_uri.clone();
        let code_verifier = state.code_verifier.clone();

        let params = [
            ("grant_type", "authorization_code".to_string()),
            ("code", code.to_string()),
            ("redirect_uri", redirect_uri),
            ("client_id", client_id),
            ("client_secret", client_secret),
            ("code_verifier", code_verifier),
        ];

        let response = self
            .http_client
            .post(&discovery.token_endpoint)
            .form(&params)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(AuthError::Oidc(format!(
                "Token exchange failed: {}",
                error_text
            )));
        }

        response
            .json()
            .await
            .map_err(|e| AuthError::Oidc(e.to_string()))
    }

    /// Fetches user info using an access token.
    pub async fn get_user_info(&mut self, access_token: &str) -> AuthResult<UserInfo> {
        let discovery = self.discover().await?;

        let response = self
            .http_client
            .get(&discovery.userinfo_endpoint)
            .bearer_auth(access_token)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(AuthError::Oidc(format!(
                "User info request failed: {}",
                error_text
            )));
        }

        response
            .json()
            .await
            .map_err(|e| AuthError::Oidc(e.to_string()))
    }

    /// Refreshes tokens using a refresh token.
    pub async fn refresh_tokens(&mut self, refresh_token: &str) -> AuthResult<TokenResponse> {
        let discovery = self.discover().await?;

        let client_id = self.config.client_id.clone();
        let client_secret = self.config.client_secret.clone();

        let params = [
            ("grant_type", "refresh_token".to_string()),
            ("refresh_token", refresh_token.to_string()),
            ("client_id", client_id),
            ("client_secret", client_secret),
        ];

        let response = self
            .http_client
            .post(&discovery.token_endpoint)
            .form(&params)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(AuthError::Oidc(format!(
                "Token refresh failed: {}",
                error_text
            )));
        }

        response
            .json()
            .await
            .map_err(|e| AuthError::Oidc(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_state_creation() {
        let state = AuthState::new("http://localhost/callback");

        assert!(!state.state.is_empty());
        assert!(!state.code_verifier.is_empty());
        assert_eq!(state.redirect_uri, "http://localhost/callback");
        assert!(!state.is_expired());
    }

    #[test]
    fn test_oidc_config_creation() {
        let config = OidcConfig::new(
            "https://accounts.google.com",
            "client-id",
            "client-secret",
            "http://localhost/callback",
        )
        .with_scopes(vec!["openid".to_string(), "email".to_string()]);

        assert_eq!(config.issuer_url, "https://accounts.google.com");
        assert_eq!(config.client_id, "client-id");
        assert_eq!(config.scopes.len(), 2);
    }
}
