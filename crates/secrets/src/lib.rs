//! Secret management for DeliDev including keychain access
//!
//! This crate provides cross-platform keychain access and secure secret
//! transport for AI agent credentials.

mod env;
mod error;
mod transport;

// Platform-specific keychain implementations
#[cfg(target_os = "macos")]
mod keychain_macos;

#[cfg(any(target_os = "windows", target_os = "linux"))]
mod keychain_keyring;

use std::collections::HashMap;

pub use env::*;
pub use error::*;
pub use transport::*;

/// Known secret keys used by AI agents
pub mod known_keys {
    /// Claude Code OAuth token
    pub const CLAUDE_CODE_OAUTH_TOKEN: &str = "CLAUDE_CODE_OAUTH_TOKEN";

    /// Anthropic API key
    pub const ANTHROPIC_API_KEY: &str = "ANTHROPIC_API_KEY";

    /// OpenAI API key
    pub const OPENAI_API_KEY: &str = "OPENAI_API_KEY";

    /// Google AI API key
    pub const GOOGLE_AI_API_KEY: &str = "GOOGLE_AI_API_KEY";

    /// GitHub token
    pub const GITHUB_TOKEN: &str = "GITHUB_TOKEN";

    /// All known keys
    pub const ALL: &[&str] = &[
        CLAUDE_CODE_OAUTH_TOKEN,
        ANTHROPIC_API_KEY,
        OPENAI_API_KEY,
        GOOGLE_AI_API_KEY,
        GITHUB_TOKEN,
    ];
}

/// Service name for keychain storage
pub const KEYCHAIN_SERVICE: &str = "com.delino.delidev";

/// Trait for keychain access
pub trait KeychainAccess: Send + Sync {
    /// Gets a secret from the keychain
    fn get(&self, account: &str) -> SecretResult<Option<String>>;

    /// Sets a secret in the keychain
    fn set(&self, account: &str, secret: &str) -> SecretResult<()>;

    /// Deletes a secret from the keychain
    fn delete(&self, account: &str) -> SecretResult<()>;

    /// Checks if a secret exists
    fn exists(&self, account: &str) -> SecretResult<bool> {
        Ok(self.get(account)?.is_some())
    }
}

/// Gets all relevant secrets for AI agents from the keychain
pub fn get_all_secrets(keychain: &dyn KeychainAccess) -> SecretResult<HashMap<String, String>> {
    let mut secrets = HashMap::new();

    for key in known_keys::ALL {
        if let Some(value) = keychain.get(key)? {
            secrets.insert(key.to_string(), value);
        }
    }

    Ok(secrets)
}

/// In-memory keychain for testing
#[derive(Debug, Default)]
pub struct MemoryKeychain {
    secrets: std::sync::RwLock<HashMap<String, String>>,
}

impl MemoryKeychain {
    /// Creates a new in-memory keychain
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a keychain with pre-populated secrets
    pub fn with_secrets(secrets: HashMap<String, String>) -> Self {
        Self {
            secrets: std::sync::RwLock::new(secrets),
        }
    }
}

impl KeychainAccess for MemoryKeychain {
    fn get(&self, account: &str) -> SecretResult<Option<String>> {
        let secrets = self.secrets.read().unwrap();
        Ok(secrets.get(account).cloned())
    }

    fn set(&self, account: &str, secret: &str) -> SecretResult<()> {
        let mut secrets = self.secrets.write().unwrap();
        secrets.insert(account.to_string(), secret.to_string());
        Ok(())
    }

    fn delete(&self, account: &str) -> SecretResult<()> {
        let mut secrets = self.secrets.write().unwrap();
        secrets.remove(account);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_keychain() {
        let keychain = MemoryKeychain::new();

        // Set a secret
        keychain.set("test_key", "test_value").unwrap();

        // Get it back
        let value = keychain.get("test_key").unwrap();
        assert_eq!(value, Some("test_value".to_string()));

        // Check existence
        assert!(keychain.exists("test_key").unwrap());
        assert!(!keychain.exists("nonexistent").unwrap());

        // Delete
        keychain.delete("test_key").unwrap();
        assert!(!keychain.exists("test_key").unwrap());
    }

    #[test]
    fn test_get_all_secrets() {
        let mut initial = HashMap::new();
        initial.insert(
            known_keys::ANTHROPIC_API_KEY.to_string(),
            "sk-ant-123".to_string(),
        );
        initial.insert(
            known_keys::OPENAI_API_KEY.to_string(),
            "sk-openai-456".to_string(),
        );

        let keychain = MemoryKeychain::with_secrets(initial);
        let secrets = get_all_secrets(&keychain).unwrap();

        assert_eq!(secrets.len(), 2);
        assert_eq!(
            secrets.get(known_keys::ANTHROPIC_API_KEY),
            Some(&"sk-ant-123".to_string())
        );
    }
}
