//! Tauri commands for secrets/keychain management

use std::collections::HashMap;

/// Status of a secret in the keychain
#[derive(serde::Serialize)]
pub struct SecretStatus {
    /// The secret key
    pub key: String,
    /// Whether the secret exists
    pub exists: bool,
}

/// Gets the status of all known secrets (whether they exist, not their values)
#[tauri::command]
pub async fn get_secrets_status() -> Result<Vec<SecretStatus>, String> {
    #[cfg(target_os = "macos")]
    {
        use secrets::{keychain_macos::MacOSKeychain, known_keys, KeychainAccess};

        let keychain = MacOSKeychain::new();
        let mut status = Vec::new();

        for key in known_keys::ALL {
            let exists = keychain.exists(key).unwrap_or(false);
            status.push(SecretStatus {
                key: key.to_string(),
                exists,
            });
        }

        Ok(status)
    }

    #[cfg(any(target_os = "windows", target_os = "linux"))]
    {
        use secrets::{keychain_keyring::KeyringKeychain, known_keys, KeychainAccess};

        let keychain = KeyringKeychain::new();
        let mut status = Vec::new();

        for key in known_keys::ALL {
            let exists = keychain.exists(key).unwrap_or(false);
            status.push(SecretStatus {
                key: key.to_string(),
                exists,
            });
        }

        Ok(status)
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        // For unsupported platforms, return empty
        Ok(Vec::new())
    }
}

/// Gets a specific secret from the keychain
#[tauri::command]
pub async fn get_secret(key: String) -> Result<Option<String>, String> {
    #[cfg(target_os = "macos")]
    {
        use secrets::{keychain_macos::MacOSKeychain, KeychainAccess};

        let keychain = MacOSKeychain::new();
        keychain.get(&key).map_err(|e| e.to_string())
    }

    #[cfg(any(target_os = "windows", target_os = "linux"))]
    {
        use secrets::{keychain_keyring::KeyringKeychain, KeychainAccess};

        let keychain = KeyringKeychain::new();
        keychain.get(&key).map_err(|e| e.to_string())
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        let _ = key;
        Err("Platform not supported".to_string())
    }
}

/// Sets a secret in the keychain
#[tauri::command]
pub async fn set_secret(key: String, value: String) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        use secrets::{keychain_macos::MacOSKeychain, KeychainAccess};

        let keychain = MacOSKeychain::new();
        keychain.set(&key, &value).map_err(|e| e.to_string())
    }

    #[cfg(any(target_os = "windows", target_os = "linux"))]
    {
        use secrets::{keychain_keyring::KeyringKeychain, KeychainAccess};

        let keychain = KeyringKeychain::new();
        keychain.set(&key, &value).map_err(|e| e.to_string())
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        let _ = (key, value);
        Err("Platform not supported".to_string())
    }
}

/// Deletes a secret from the keychain
#[tauri::command]
pub async fn delete_secret(key: String) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        use secrets::{keychain_macos::MacOSKeychain, KeychainAccess};

        let keychain = MacOSKeychain::new();
        keychain.delete(&key).map_err(|e| e.to_string())
    }

    #[cfg(any(target_os = "windows", target_os = "linux"))]
    {
        use secrets::{keychain_keyring::KeyringKeychain, KeychainAccess};

        let keychain = KeyringKeychain::new();
        keychain.delete(&key).map_err(|e| e.to_string())
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        let _ = key;
        Err("Platform not supported".to_string())
    }
}

/// Gets all secrets for AI agent execution (from keychain)
#[tauri::command]
pub async fn get_all_secrets_for_execution() -> Result<HashMap<String, String>, String> {
    #[cfg(target_os = "macos")]
    {
        use secrets::{get_all_secrets, keychain_macos::MacOSKeychain, KeychainAccess};

        let keychain = MacOSKeychain::new();
        get_all_secrets(&keychain as &dyn KeychainAccess).map_err(|e| e.to_string())
    }

    #[cfg(any(target_os = "windows", target_os = "linux"))]
    {
        use secrets::{get_all_secrets, keychain_keyring::KeyringKeychain, KeychainAccess};

        let keychain = KeyringKeychain::new();
        get_all_secrets(&keychain as &dyn KeychainAccess).map_err(|e| e.to_string())
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        Ok(HashMap::new())
    }
}

/// Gets the list of known secret keys
#[tauri::command]
pub fn get_known_secret_keys() -> Vec<String> {
    secrets::known_keys::ALL
        .iter()
        .map(|s| s.to_string())
        .collect()
}
