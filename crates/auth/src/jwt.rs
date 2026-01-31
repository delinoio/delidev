//! JWT token handling

use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

use crate::{AuthError, AuthResult, AuthenticatedUser};

/// JWT claims
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// Subject (user ID)
    pub sub: String,

    /// Email address
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,

    /// User name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// Issued at timestamp
    pub iat: i64,

    /// Expiration timestamp
    pub exp: i64,

    /// Not before timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nbf: Option<i64>,

    /// Issuer
    #[serde(skip_serializing_if = "Option::is_none")]
    pub iss: Option<String>,

    /// Audience
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aud: Option<String>,
}

impl Claims {
    /// Creates new claims for a user
    pub fn new(user_id: impl Into<String>, email: Option<String>, name: Option<String>) -> Self {
        let now = Utc::now().timestamp();
        let exp = (Utc::now() + Duration::hours(24)).timestamp();

        Self {
            sub: user_id.into(),
            email,
            name,
            iat: now,
            exp,
            nbf: Some(now),
            iss: None,
            aud: None,
        }
    }

    /// Sets the expiration time
    pub fn with_expiration(mut self, exp: i64) -> Self {
        self.exp = exp;
        self
    }

    /// Sets the issuer
    pub fn with_issuer(mut self, iss: impl Into<String>) -> Self {
        self.iss = Some(iss.into());
        self
    }

    /// Sets the audience
    pub fn with_audience(mut self, aud: impl Into<String>) -> Self {
        self.aud = Some(aud.into());
        self
    }

    /// Converts claims to an authenticated user
    pub fn to_user(&self) -> AuthenticatedUser {
        AuthenticatedUser {
            id: self.sub.clone(),
            email: self.email.clone(),
            name: self.name.clone(),
        }
    }
}

/// JWT authentication service
#[derive(Clone)]
pub struct JwtAuth {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    validation: Validation,
    issuer: Option<String>,
}

impl JwtAuth {
    /// Creates a new JWT auth service with the given secret
    pub fn new(secret: &[u8]) -> Self {
        Self {
            encoding_key: EncodingKey::from_secret(secret),
            decoding_key: DecodingKey::from_secret(secret),
            validation: Validation::default(),
            issuer: None,
        }
    }

    /// Creates a JWT auth service with HS256 algorithm
    pub fn new_hs256(secret: &[u8]) -> Self {
        let mut validation = Validation::new(jsonwebtoken::Algorithm::HS256);
        validation.validate_exp = true;
        validation.validate_nbf = true;

        Self {
            encoding_key: EncodingKey::from_secret(secret),
            decoding_key: DecodingKey::from_secret(secret),
            validation,
            issuer: None,
        }
    }

    /// Sets the issuer for token validation
    pub fn with_issuer(mut self, issuer: impl Into<String>) -> Self {
        let iss = issuer.into();
        self.validation.set_issuer(&[&iss]);
        self.issuer = Some(iss);
        self
    }

    /// Sets the audience for token validation
    pub fn with_audience(mut self, audience: impl Into<String>) -> Self {
        self.validation.set_audience(&[audience.into()]);
        self
    }

    /// Creates a JWT token for a user
    pub fn create_token(
        &self,
        user_id: &str,
        email: Option<String>,
        name: Option<String>,
    ) -> AuthResult<String> {
        let mut claims = Claims::new(user_id, email, name);

        if let Some(ref iss) = self.issuer {
            claims = claims.with_issuer(iss.clone());
        }

        encode(&Header::default(), &claims, &self.encoding_key)
            .map_err(|e| AuthError::JwtEncode(e.to_string()))
    }

    /// Creates a JWT token with custom expiration
    pub fn create_token_with_expiration(
        &self,
        user_id: &str,
        email: Option<String>,
        name: Option<String>,
        expires_in_hours: i64,
    ) -> AuthResult<String> {
        let exp = (Utc::now() + Duration::hours(expires_in_hours)).timestamp();
        let mut claims = Claims::new(user_id, email, name).with_expiration(exp);

        if let Some(ref iss) = self.issuer {
            claims = claims.with_issuer(iss.clone());
        }

        encode(&Header::default(), &claims, &self.encoding_key)
            .map_err(|e| AuthError::JwtEncode(e.to_string()))
    }

    /// Verifies a JWT token and returns the claims
    pub fn verify_token(&self, token: &str) -> AuthResult<Claims> {
        let token_data = decode::<Claims>(token, &self.decoding_key, &self.validation)?;
        Ok(token_data.claims)
    }

    /// Verifies a JWT token and returns the authenticated user
    pub fn authenticate(&self, token: &str) -> AuthResult<AuthenticatedUser> {
        let claims = self.verify_token(token)?;
        Ok(claims.to_user())
    }

    /// Extracts the token from an Authorization header
    pub fn extract_token(auth_header: &str) -> Option<&str> {
        auth_header.strip_prefix("Bearer ")
    }

    /// Refreshes a token if it's close to expiration
    pub fn refresh_token(&self, token: &str, threshold_hours: i64) -> AuthResult<Option<String>> {
        let claims = self.verify_token(token)?;

        let now = Utc::now().timestamp();
        let threshold = threshold_hours * 3600;

        // Only refresh if token expires within threshold
        if claims.exp - now < threshold {
            let new_token = self.create_token(&claims.sub, claims.email, claims.name)?;
            Ok(Some(new_token))
        } else {
            Ok(None)
        }
    }
}

impl std::fmt::Debug for JwtAuth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JwtAuth")
            .field("issuer", &self.issuer)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_and_verify_token() {
        let auth = JwtAuth::new(b"test-secret-key-that-is-long-enough");

        let token = auth
            .create_token("user-123", Some("test@example.com".to_string()), None)
            .unwrap();

        let claims = auth.verify_token(&token).unwrap();
        assert_eq!(claims.sub, "user-123");
        assert_eq!(claims.email, Some("test@example.com".to_string()));
    }

    #[test]
    fn test_authenticate() {
        let auth = JwtAuth::new(b"test-secret-key-that-is-long-enough");

        let token = auth
            .create_token(
                "user-456",
                Some("user@example.com".to_string()),
                Some("Test User".to_string()),
            )
            .unwrap();

        let user = auth.authenticate(&token).unwrap();
        assert_eq!(user.id, "user-456");
        assert_eq!(user.email, Some("user@example.com".to_string()));
        assert_eq!(user.name, Some("Test User".to_string()));
    }

    #[test]
    fn test_extract_token() {
        let header = "Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9";
        let token = JwtAuth::extract_token(header);
        assert_eq!(token, Some("eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9"));

        let invalid = "Basic credentials";
        assert_eq!(JwtAuth::extract_token(invalid), None);
    }

    #[test]
    fn test_invalid_token() {
        let auth = JwtAuth::new(b"test-secret-key-that-is-long-enough");

        let result = auth.verify_token("invalid-token");
        assert!(result.is_err());
    }

    #[test]
    fn test_wrong_secret() {
        let auth1 = JwtAuth::new(b"secret-one-that-is-long-enough");
        let auth2 = JwtAuth::new(b"secret-two-that-is-long-enough");

        let token = auth1.create_token("user-789", None, None).unwrap();

        let result = auth2.verify_token(&token);
        assert!(result.is_err());
    }
}
