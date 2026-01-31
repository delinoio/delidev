//! Keyring-based keychain implementation for Windows and Linux

use keyring::Entry;

use crate::{KeychainAccess, SecretError, SecretResult, KEYCHAIN_SERVICE};

/// Keyring-based keychain accessor for Windows and Linux
#[derive(Debug, Default)]
pub struct KeyringKeychain;

impl KeyringKeychain {
    /// Creates a new keyring-based keychain accessor
    pub fn new() -> Self {
        Self
    }

    /// Gets a keyring entry for the given account
    fn entry(&self, account: &str) -> Result<Entry, SecretError> {
        Entry::new(KEYCHAIN_SERVICE, account)
            .map_err(|e| SecretError::Keychain(format!("Failed to create entry: {}", e)))
    }
}

impl KeychainAccess for KeyringKeychain {
    fn get(&self, account: &str) -> SecretResult<Option<String>> {
        let entry = self.entry(account)?;

        match entry.get_password() {
            Ok(password) => Ok(Some(password)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(SecretError::Keychain(e.to_string())),
        }
    }

    fn set(&self, account: &str, secret: &str) -> SecretResult<()> {
        let entry = self.entry(account)?;

        entry
            .set_password(secret)
            .map_err(|e| SecretError::Keychain(e.to_string()))
    }

    fn delete(&self, account: &str) -> SecretResult<()> {
        let entry = self.entry(account)?;

        match entry.delete_credential() {
            Ok(()) => Ok(()),
            Err(keyring::Error::NoEntry) => Ok(()), // Already deleted
            Err(e) => Err(SecretError::Keychain(e.to_string())),
        }
    }
}

/// Creates the default keychain accessor for Windows/Linux
pub fn create_keychain() -> impl KeychainAccess {
    KeyringKeychain::new()
}
