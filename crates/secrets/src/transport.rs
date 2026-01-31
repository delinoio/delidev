//! Secure secret transport

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::SecretResult;

/// Payload for sending secrets from client to server
///
/// Secrets are encrypted in transit via TLS. This struct provides
/// additional metadata for verification and replay attack prevention.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretPayload {
    /// Task ID the secrets are for
    pub task_id: String,

    /// The secrets (key -> value)
    pub secrets: HashMap<String, String>,

    /// Timestamp to prevent replay attacks
    pub timestamp: i64,

    /// Client-generated nonce
    pub nonce: String,

    /// Optional signature for verification
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
}

impl SecretPayload {
    /// Creates a new secret payload
    pub fn new(task_id: impl Into<String>, secrets: HashMap<String, String>) -> Self {
        Self {
            task_id: task_id.into(),
            secrets,
            timestamp: chrono::Utc::now().timestamp(),
            nonce: uuid::Uuid::new_v4().to_string(),
            signature: None,
        }
    }

    /// Verifies that the payload is not too old (within 5 minutes)
    pub fn is_valid_timestamp(&self) -> bool {
        let now = chrono::Utc::now().timestamp();
        let age = now - self.timestamp;
        age >= 0 && age < 300 // 5 minutes
    }

    /// Returns the number of secrets in the payload
    pub fn secret_count(&self) -> usize {
        self.secrets.len()
    }

    /// Checks if the payload contains a specific secret
    pub fn has_secret(&self, key: &str) -> bool {
        self.secrets.contains_key(key)
    }
}

/// Configuration for secret transport
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransportConfig {
    /// Maximum age of payloads in seconds
    pub max_payload_age_seconds: i64,

    /// Whether to require signatures
    pub require_signature: bool,

    /// Allowed secret keys (if empty, all are allowed)
    pub allowed_keys: Vec<String>,
}

impl Default for TransportConfig {
    fn default() -> Self {
        Self {
            max_payload_age_seconds: 300, // 5 minutes
            require_signature: false,
            allowed_keys: Vec::new(),
        }
    }
}

impl TransportConfig {
    /// Validates a secret payload against this config
    pub fn validate_payload(&self, payload: &SecretPayload) -> SecretResult<()> {
        use crate::SecretError;

        // Check timestamp
        let now = chrono::Utc::now().timestamp();
        let age = now - payload.timestamp;
        if age < 0 || age > self.max_payload_age_seconds {
            return Err(SecretError::Transport(format!(
                "Payload too old or has future timestamp (age: {} seconds)",
                age
            )));
        }

        // Check signature if required
        if self.require_signature && payload.signature.is_none() {
            return Err(SecretError::Transport(
                "Signature required but not provided".to_string(),
            ));
        }

        // Check allowed keys
        if !self.allowed_keys.is_empty() {
            for key in payload.secrets.keys() {
                if !self.allowed_keys.contains(key) {
                    return Err(SecretError::Transport(format!(
                        "Secret key '{}' not in allowed list",
                        key
                    )));
                }
            }
        }

        Ok(())
    }
}

/// Redacts secrets for logging/display
pub fn redact_secrets(secrets: &HashMap<String, String>) -> HashMap<String, String> {
    secrets
        .iter()
        .map(|(k, v)| {
            let redacted = if v.len() > 8 {
                format!("{}...{}", &v[..4], &v[v.len() - 4..])
            } else {
                "****".to_string()
            };
            (k.clone(), redacted)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_secret_payload_creation() {
        let mut secrets = HashMap::new();
        secrets.insert("API_KEY".to_string(), "secret-value".to_string());

        let payload = SecretPayload::new("task-123", secrets);

        assert_eq!(payload.task_id, "task-123");
        assert_eq!(payload.secret_count(), 1);
        assert!(payload.has_secret("API_KEY"));
        assert!(payload.is_valid_timestamp());
    }

    #[test]
    fn test_transport_config_validation() {
        let config = TransportConfig::default();

        let mut secrets = HashMap::new();
        secrets.insert("API_KEY".to_string(), "value".to_string());
        let payload = SecretPayload::new("task-123", secrets);

        assert!(config.validate_payload(&payload).is_ok());
    }

    #[test]
    fn test_allowed_keys_validation() {
        let config = TransportConfig {
            allowed_keys: vec!["ALLOWED_KEY".to_string()],
            ..Default::default()
        };

        let mut secrets = HashMap::new();
        secrets.insert("DISALLOWED_KEY".to_string(), "value".to_string());
        let payload = SecretPayload::new("task-123", secrets);

        assert!(config.validate_payload(&payload).is_err());
    }

    #[test]
    fn test_redact_secrets() {
        let mut secrets = HashMap::new();
        secrets.insert("API_KEY".to_string(), "sk-1234567890abcdef".to_string());
        secrets.insert("SHORT".to_string(), "abc".to_string());

        let redacted = redact_secrets(&secrets);

        assert_eq!(redacted.get("API_KEY"), Some(&"sk-1...cdef".to_string()));
        assert_eq!(redacted.get("SHORT"), Some(&"****".to_string()));
    }
}
