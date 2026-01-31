//! Keychain trait and implementations.

use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use tokio::sync::RwLock;

use crate::{SecretKey, SecretsResult};

/// Service name used for keychain entries.
pub const KEYCHAIN_SERVICE: &str = "com.delidev.app";

/// Trait for keychain operations.
#[async_trait]
pub trait Keychain: Send + Sync {
    /// Gets a secret by key.
    async fn get(&self, key: &SecretKey) -> SecretsResult<Option<String>>;

    /// Gets a secret by key name string.
    async fn get_by_name(&self, key_name: &str) -> SecretsResult<Option<String>>;

    /// Sets a secret.
    async fn set(&self, key: &SecretKey, value: &str) -> SecretsResult<()>;

    /// Sets a secret by key name string.
    async fn set_by_name(&self, key_name: &str, value: &str) -> SecretsResult<()>;

    /// Deletes a secret.
    async fn delete(&self, key: &SecretKey) -> SecretsResult<()>;

    /// Deletes a secret by key name string.
    async fn delete_by_name(&self, key_name: &str) -> SecretsResult<()>;

    /// Lists all secret keys that have values.
    async fn list(&self) -> SecretsResult<Vec<SecretKey>>;

    /// Gets multiple secrets at once.
    async fn get_many(&self, keys: &[SecretKey]) -> SecretsResult<HashMap<SecretKey, String>> {
        let mut result = HashMap::new();
        for key in keys {
            if let Some(value) = self.get(key).await? {
                result.insert(*key, value);
            }
        }
        Ok(result)
    }

    /// Gets all known secrets that have values.
    async fn get_all(&self) -> SecretsResult<HashMap<SecretKey, String>> {
        self.get_many(SecretKey::all()).await
    }
}

/// Native keychain implementation using the system keychain.
pub struct NativeKeychain {
    service: String,
}

impl NativeKeychain {
    /// Creates a new native keychain accessor.
    pub fn new() -> Self {
        Self {
            service: KEYCHAIN_SERVICE.to_string(),
        }
    }

    /// Creates a native keychain with a custom service name.
    pub fn with_service(service: impl Into<String>) -> Self {
        Self {
            service: service.into(),
        }
    }

    fn get_entry(&self, key_name: &str) -> keyring::Result<keyring::Entry> {
        keyring::Entry::new(&self.service, key_name)
    }
}

impl Default for NativeKeychain {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Keychain for NativeKeychain {
    async fn get(&self, key: &SecretKey) -> SecretsResult<Option<String>> {
        self.get_by_name(key.key_name()).await
    }

    async fn get_by_name(&self, key_name: &str) -> SecretsResult<Option<String>> {
        let entry = self.get_entry(key_name)?;
        match entry.get_password() {
            Ok(password) => Ok(Some(password)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    async fn set(&self, key: &SecretKey, value: &str) -> SecretsResult<()> {
        self.set_by_name(key.key_name(), value).await
    }

    async fn set_by_name(&self, key_name: &str, value: &str) -> SecretsResult<()> {
        let entry = self.get_entry(key_name)?;
        entry.set_password(value)?;
        Ok(())
    }

    async fn delete(&self, key: &SecretKey) -> SecretsResult<()> {
        self.delete_by_name(key.key_name()).await
    }

    async fn delete_by_name(&self, key_name: &str) -> SecretsResult<()> {
        let entry = self.get_entry(key_name)?;
        match entry.delete_credential() {
            Ok(()) => Ok(()),
            Err(keyring::Error::NoEntry) => Ok(()), // Already deleted
            Err(e) => Err(e.into()),
        }
    }

    async fn list(&self) -> SecretsResult<Vec<SecretKey>> {
        let mut result = Vec::new();
        for key in SecretKey::all() {
            if self.get(key).await?.is_some() {
                result.push(*key);
            }
        }
        Ok(result)
    }
}

/// In-memory keychain implementation for testing.
#[derive(Debug, Default)]
pub struct MemoryKeychain {
    secrets: Arc<RwLock<HashMap<String, String>>>,
}

impl MemoryKeychain {
    /// Creates a new in-memory keychain.
    pub fn new() -> Self {
        Self {
            secrets: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl Keychain for MemoryKeychain {
    async fn get(&self, key: &SecretKey) -> SecretsResult<Option<String>> {
        self.get_by_name(key.key_name()).await
    }

    async fn get_by_name(&self, key_name: &str) -> SecretsResult<Option<String>> {
        let secrets = self.secrets.read().await;
        Ok(secrets.get(key_name).cloned())
    }

    async fn set(&self, key: &SecretKey, value: &str) -> SecretsResult<()> {
        self.set_by_name(key.key_name(), value).await
    }

    async fn set_by_name(&self, key_name: &str, value: &str) -> SecretsResult<()> {
        let mut secrets = self.secrets.write().await;
        secrets.insert(key_name.to_string(), value.to_string());
        Ok(())
    }

    async fn delete(&self, key: &SecretKey) -> SecretsResult<()> {
        self.delete_by_name(key.key_name()).await
    }

    async fn delete_by_name(&self, key_name: &str) -> SecretsResult<()> {
        let mut secrets = self.secrets.write().await;
        secrets.remove(key_name);
        Ok(())
    }

    async fn list(&self) -> SecretsResult<Vec<SecretKey>> {
        let secrets = self.secrets.read().await;
        let mut result = Vec::new();
        for key in SecretKey::all() {
            if secrets.contains_key(key.key_name()) {
                result.push(*key);
            }
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_memory_keychain() {
        let keychain = MemoryKeychain::new();

        // Test set and get
        keychain
            .set(&SecretKey::AnthropicApiKey, "test-api-key")
            .await
            .unwrap();
        let value = keychain.get(&SecretKey::AnthropicApiKey).await.unwrap();
        assert_eq!(value, Some("test-api-key".to_string()));

        // Test list
        let keys = keychain.list().await.unwrap();
        assert!(keys.contains(&SecretKey::AnthropicApiKey));

        // Test delete
        keychain.delete(&SecretKey::AnthropicApiKey).await.unwrap();
        let value = keychain.get(&SecretKey::AnthropicApiKey).await.unwrap();
        assert!(value.is_none());
    }

    #[tokio::test]
    async fn test_get_many() {
        let keychain = MemoryKeychain::new();

        keychain
            .set(&SecretKey::AnthropicApiKey, "anthropic-key")
            .await
            .unwrap();
        keychain
            .set(&SecretKey::OpenAiApiKey, "openai-key")
            .await
            .unwrap();

        let secrets = keychain
            .get_many(&[
                SecretKey::AnthropicApiKey,
                SecretKey::OpenAiApiKey,
                SecretKey::GithubToken,
            ])
            .await
            .unwrap();

        assert_eq!(secrets.len(), 2);
        assert_eq!(
            secrets.get(&SecretKey::AnthropicApiKey),
            Some(&"anthropic-key".to_string())
        );
        assert_eq!(
            secrets.get(&SecretKey::OpenAiApiKey),
            Some(&"openai-key".to_string())
        );
        assert!(!secrets.contains_key(&SecretKey::GithubToken));
    }
}
