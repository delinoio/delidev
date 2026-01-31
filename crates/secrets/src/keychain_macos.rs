//! macOS Keychain implementation using security-framework

use security_framework::passwords::{
    delete_generic_password, get_generic_password, set_generic_password,
};

use crate::{KeychainAccess, SecretError, SecretResult, KEYCHAIN_SERVICE};

/// macOS Keychain accessor
#[derive(Debug, Default)]
pub struct MacOSKeychain;

impl MacOSKeychain {
    /// Creates a new macOS keychain accessor
    pub fn new() -> Self {
        Self
    }
}

impl KeychainAccess for MacOSKeychain {
    fn get(&self, account: &str) -> SecretResult<Option<String>> {
        match get_generic_password(KEYCHAIN_SERVICE, account) {
            Ok(password) => {
                let value = String::from_utf8(password.to_vec())
                    .map_err(|e| SecretError::InvalidFormat(e.to_string()))?;
                Ok(Some(value))
            }
            Err(e) => {
                // Check if it's a "not found" error
                let error_string = e.to_string();
                if error_string.contains("not found") || error_string.contains("ItemNotFound") {
                    Ok(None)
                } else {
                    Err(SecretError::Keychain(error_string))
                }
            }
        }
    }

    fn set(&self, account: &str, secret: &str) -> SecretResult<()> {
        // First try to delete any existing entry
        let _ = delete_generic_password(KEYCHAIN_SERVICE, account);

        set_generic_password(KEYCHAIN_SERVICE, account, secret.as_bytes())
            .map_err(|e| SecretError::Keychain(e.to_string()))
    }

    fn delete(&self, account: &str) -> SecretResult<()> {
        match delete_generic_password(KEYCHAIN_SERVICE, account) {
            Ok(()) => Ok(()),
            Err(e) => {
                let error_string = e.to_string();
                if error_string.contains("not found") || error_string.contains("ItemNotFound") {
                    Ok(()) // Already deleted, that's fine
                } else {
                    Err(SecretError::Keychain(error_string))
                }
            }
        }
    }
}

/// Creates the default keychain accessor for macOS
pub fn create_keychain() -> impl KeychainAccess {
    MacOSKeychain::new()
}
