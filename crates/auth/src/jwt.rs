//! JWT token generation and validation.

use chrono::{Duration, Utc};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{AuthError, AuthResult, DEFAULT_JWT_EXPIRATION_HOURS, DEFAULT_JWT_ISSUER};

/// JWT claims for DeliDev access tokens.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// Subject (user ID).
    pub sub: String,
    /// Email address.
    pub email: String,
    /// Display name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Issued at timestamp.
    pub iat: i64,
    /// Expiration timestamp.
    pub exp: i64,
    /// Issuer.
    pub iss: String,
    /// JWT ID.
    pub jti: String,
}

impl Claims {
    /// Creates new claims for a user.
    pub fn new(user_id: Uuid, email: String, name: Option<String>, expiration_hours: u64) -> Self {
        let now = Utc::now();
        let exp = now + Duration::hours(expiration_hours as i64);

        Self {
            sub: user_id.to_string(),
            email,
            name,
            iat: now.timestamp(),
            exp: exp.timestamp(),
            iss: DEFAULT_JWT_ISSUER.to_string(),
            jti: Uuid::new_v4().to_string(),
        }
    }

    /// Returns the user ID.
    pub fn user_id(&self) -> AuthResult<Uuid> {
        self.sub.parse().map_err(|_| AuthError::InvalidToken)
    }

    /// Returns true if the token is expired.
    pub fn is_expired(&self) -> bool {
        Utc::now().timestamp() > self.exp
    }
}

/// JWT configuration.
#[derive(Debug, Clone)]
pub struct JwtConfig {
    /// Secret key for signing tokens.
    pub secret: String,
    /// Token expiration in hours.
    pub expiration_hours: u64,
    /// Token issuer.
    pub issuer: String,
}

impl JwtConfig {
    /// Creates a new JWT configuration.
    pub fn new(secret: impl Into<String>) -> Self {
        Self {
            secret: secret.into(),
            expiration_hours: DEFAULT_JWT_EXPIRATION_HOURS,
            issuer: DEFAULT_JWT_ISSUER.to_string(),
        }
    }

    /// Sets the expiration time in hours.
    pub fn with_expiration_hours(mut self, hours: u64) -> Self {
        self.expiration_hours = hours;
        self
    }

    /// Sets the issuer.
    pub fn with_issuer(mut self, issuer: impl Into<String>) -> Self {
        self.issuer = issuer.into();
        self
    }
}

/// JWT token manager.
#[derive(Clone)]
pub struct JwtManager {
    config: JwtConfig,
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
}

impl std::fmt::Debug for JwtManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JwtManager")
            .field("config", &self.config)
            .finish_non_exhaustive()
    }
}

impl JwtManager {
    /// Creates a new JWT manager.
    pub fn new(config: JwtConfig) -> Self {
        let encoding_key = EncodingKey::from_secret(config.secret.as_bytes());
        let decoding_key = DecodingKey::from_secret(config.secret.as_bytes());

        Self {
            config,
            encoding_key,
            decoding_key,
        }
    }

    /// Generates an access token for a user.
    pub fn generate_token(
        &self,
        user_id: Uuid,
        email: String,
        name: Option<String>,
    ) -> AuthResult<String> {
        let claims = Claims::new(user_id, email, name, self.config.expiration_hours);

        encode(&Header::default(), &claims, &self.encoding_key)
            .map_err(|e| AuthError::JwtEncoding(e.to_string()))
    }

    /// Validates and decodes a token.
    pub fn validate_token(&self, token: &str) -> AuthResult<Claims> {
        let mut validation = Validation::default();
        validation.set_issuer(&[&self.config.issuer]);

        let token_data = decode::<Claims>(token, &self.decoding_key, &validation)?;

        Ok(token_data.claims)
    }

    /// Returns the expiration time in seconds.
    pub fn expiration_seconds(&self) -> u64 {
        self.config.expiration_hours * 3600
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jwt_generation_and_validation() {
        let config = JwtConfig::new("test-secret-key-must-be-long-enough-for-security");
        let manager = JwtManager::new(config);

        let user_id = Uuid::new_v4();
        let email = "test@example.com".to_string();
        let name = Some("Test User".to_string());

        let token = manager
            .generate_token(user_id, email.clone(), name.clone())
            .unwrap();

        let claims = manager.validate_token(&token).unwrap();

        assert_eq!(claims.user_id().unwrap(), user_id);
        assert_eq!(claims.email, email);
        assert_eq!(claims.name, name);
        assert!(!claims.is_expired());
    }

    #[test]
    fn test_invalid_token() {
        let config = JwtConfig::new("test-secret-key-must-be-long-enough-for-security");
        let manager = JwtManager::new(config);

        let result = manager.validate_token("invalid-token");
        assert!(result.is_err());
    }

    #[test]
    fn test_wrong_secret() {
        let config1 = JwtConfig::new("secret-one-must-be-long-enough");
        let config2 = JwtConfig::new("secret-two-must-be-long-enough");
        let manager1 = JwtManager::new(config1);
        let manager2 = JwtManager::new(config2);

        let user_id = Uuid::new_v4();
        let token = manager1
            .generate_token(user_id, "test@example.com".to_string(), None)
            .unwrap();

        let result = manager2.validate_token(&token);
        assert!(result.is_err());
    }
}
