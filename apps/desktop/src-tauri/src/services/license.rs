use std::sync::Arc;

use reqwest::Client;
use thiserror::Error;
use tokio::sync::RwLock;

use crate::{
    config::ConfigManager,
    entities::{
        LicenseCredentials, LicenseInfo, LicenseStatus, PolarActivateRequest,
        PolarActivateResponse, PolarDeactivateRequest, PolarValidateRequest, PolarValidateResponse,
        POLAR_ORGANIZATION_ID,
    },
};

/// Polar.sh API base URL
const POLAR_API_BASE: &str = "https://api.polar.sh";

/// Customer portal endpoint for license operations (no auth required)
const POLAR_LICENSE_VALIDATE: &str = "/v1/customer-portal/license-keys/validate";
const POLAR_LICENSE_ACTIVATE: &str = "/v1/customer-portal/license-keys/activate";
const POLAR_LICENSE_DEACTIVATE: &str = "/v1/customer-portal/license-keys/deactivate";

#[derive(Error, Debug)]
pub enum LicenseError {
    #[error("HTTP request failed: {0}")]
    HttpError(#[from] reqwest::Error),
    #[error("License key not found")]
    NotFound,
    #[error("License key validation failed: {0}")]
    ValidationFailed(String),
    #[error("License activation failed: {0}")]
    ActivationFailed(String),
    #[error("License deactivation failed: {0}")]
    DeactivationFailed(String),
    #[error("Config error: {0}")]
    ConfigError(String),
    #[error("No license configured")]
    NoLicense,
    #[error("Activation limit reached")]
    ActivationLimitReached,
}

pub type LicenseResult<T> = Result<T, LicenseError>;

/// Service for managing license validation with Polar.sh
pub struct LicenseService {
    /// HTTP client for API requests
    client: Client,
    /// Configuration manager for persisting license credentials
    config_manager: Arc<ConfigManager>,
    /// Cached license credentials
    credentials: Arc<RwLock<LicenseCredentials>>,
    /// Cached license info
    license_info: Arc<RwLock<LicenseInfo>>,
}

impl LicenseService {
    /// Creates a new license service
    pub fn new(config_manager: Arc<ConfigManager>) -> Self {
        let credentials = config_manager
            .load_license_credentials()
            .unwrap_or_default();
        let license_info = LicenseInfo::default();

        Self {
            client: Client::new(),
            config_manager,
            credentials: Arc::new(RwLock::new(credentials)),
            license_info: Arc::new(RwLock::new(license_info)),
        }
    }

    /// Gets the current license info
    pub async fn get_license_info(&self) -> LicenseInfo {
        self.license_info.read().await.clone()
    }

    /// Checks if a license is configured
    pub async fn has_license(&self) -> bool {
        self.credentials.read().await.key.is_some()
    }

    /// Gets the raw license key (for API calls)
    pub async fn get_license_key(&self) -> Option<String> {
        self.credentials.read().await.key.clone()
    }

    /// Checks if the license is valid and active
    pub async fn is_license_valid(&self) -> bool {
        let info = self.license_info.read().await;
        info.status == LicenseStatus::Active
    }

    /// Validates the current license key with Polar.sh
    pub async fn validate_license(&self) -> LicenseResult<LicenseInfo> {
        let creds = self.credentials.read().await;
        let key = creds.key.as_ref().ok_or(LicenseError::NoLicense)?;

        let request = PolarValidateRequest {
            key: key.clone(),
            organization_id: POLAR_ORGANIZATION_ID.to_string(),
            activation_id: creds.activation_id.clone(),
        };

        let response = self
            .client
            .post(format!("{}{}", POLAR_API_BASE, POLAR_LICENSE_VALIDATE))
            .json(&request)
            .send()
            .await?;

        let status = response.status();
        if status.is_success() {
            let validate_response: PolarValidateResponse = response.json().await?;
            let license_info = LicenseInfo::from(&validate_response);

            // Update cached info
            *self.license_info.write().await = license_info.clone();

            Ok(license_info)
        } else if status.as_u16() == 404 {
            let info = LicenseInfo {
                status: LicenseStatus::Invalid,
                ..Default::default()
            };
            *self.license_info.write().await = info.clone();
            Err(LicenseError::NotFound)
        } else {
            let error_text = response.text().await.unwrap_or_default();
            let info = LicenseInfo {
                status: LicenseStatus::Invalid,
                ..Default::default()
            };
            *self.license_info.write().await = info;
            Err(LicenseError::ValidationFailed(error_text))
        }
    }

    /// Activates a license key for this device
    pub async fn activate_license(
        &self,
        key: &str,
        device_label: &str,
    ) -> LicenseResult<LicenseInfo> {
        let request = PolarActivateRequest {
            key: key.to_string(),
            organization_id: POLAR_ORGANIZATION_ID.to_string(),
            label: device_label.to_string(),
            conditions: None,
            meta: Some(serde_json::json!({
                "app": "delidev",
                "platform": std::env::consts::OS,
                "arch": std::env::consts::ARCH,
            })),
        };

        let response = self
            .client
            .post(format!("{}{}", POLAR_API_BASE, POLAR_LICENSE_ACTIVATE))
            .json(&request)
            .send()
            .await?;

        let status = response.status();
        if status.is_success() {
            let activate_response: PolarActivateResponse = response.json().await?;

            // Save credentials
            let credentials = LicenseCredentials {
                key: Some(key.to_string()),
                activation_id: Some(activate_response.id.clone()),
                device_label: Some(device_label.to_string()),
            };

            self.config_manager
                .save_license_credentials(&credentials)
                .map_err(|e| LicenseError::ConfigError(e.to_string()))?;

            *self.credentials.write().await = credentials;

            // Get license info from the response
            let license_info = if let Some(license_key) = &activate_response.license_key {
                LicenseInfo::from(license_key)
            } else {
                // Validate to get full info
                self.validate_license().await?
            };

            *self.license_info.write().await = license_info.clone();

            Ok(license_info)
        } else if status.as_u16() == 404 {
            Err(LicenseError::NotFound)
        } else if status.as_u16() == 422 {
            let error_text = response.text().await.unwrap_or_default();
            if error_text.contains("limit") {
                Err(LicenseError::ActivationLimitReached)
            } else {
                Err(LicenseError::ActivationFailed(error_text))
            }
        } else {
            let error_text = response.text().await.unwrap_or_default();
            Err(LicenseError::ActivationFailed(error_text))
        }
    }

    /// Deactivates the license for this device
    pub async fn deactivate_license(&self) -> LicenseResult<()> {
        let creds = self.credentials.read().await;
        let key = creds.key.as_ref().ok_or(LicenseError::NoLicense)?;
        let activation_id =
            creds
                .activation_id
                .as_ref()
                .ok_or(LicenseError::DeactivationFailed(
                    "No activation ID".to_string(),
                ))?;

        let request = PolarDeactivateRequest {
            key: key.clone(),
            organization_id: POLAR_ORGANIZATION_ID.to_string(),
            activation_id: activation_id.clone(),
        };

        let response = self
            .client
            .post(format!("{}{}", POLAR_API_BASE, POLAR_LICENSE_DEACTIVATE))
            .json(&request)
            .send()
            .await?;

        let status = response.status();
        drop(creds); // Release the read lock before writing

        if status.is_success() || status.as_u16() == 204 {
            // Clear credentials
            let credentials = LicenseCredentials::default();
            self.config_manager
                .save_license_credentials(&credentials)
                .map_err(|e| LicenseError::ConfigError(e.to_string()))?;

            *self.credentials.write().await = credentials;
            *self.license_info.write().await = LicenseInfo::default();

            Ok(())
        } else {
            let error_text = response.text().await.unwrap_or_default();
            Err(LicenseError::DeactivationFailed(error_text))
        }
    }

    /// Sets a license key without activation (for keys that don't require
    /// activation)
    pub async fn set_license_key(&self, key: &str) -> LicenseResult<LicenseInfo> {
        // First validate the key
        let request = PolarValidateRequest {
            key: key.to_string(),
            organization_id: POLAR_ORGANIZATION_ID.to_string(),
            activation_id: None,
        };

        let response = self
            .client
            .post(format!("{}{}", POLAR_API_BASE, POLAR_LICENSE_VALIDATE))
            .json(&request)
            .send()
            .await?;

        let status = response.status();
        if status.is_success() {
            let validate_response: PolarValidateResponse = response.json().await?;

            // Check if activation is required
            if validate_response.limit_activations.is_some() {
                return Err(LicenseError::ActivationFailed(
                    "This license key requires activation. Please use activate_license instead."
                        .to_string(),
                ));
            }

            // Save credentials (no activation needed)
            let credentials = LicenseCredentials {
                key: Some(key.to_string()),
                activation_id: None,
                device_label: None,
            };

            self.config_manager
                .save_license_credentials(&credentials)
                .map_err(|e| LicenseError::ConfigError(e.to_string()))?;

            *self.credentials.write().await = credentials;

            let license_info = LicenseInfo::from(&validate_response);
            *self.license_info.write().await = license_info.clone();

            Ok(license_info)
        } else if status.as_u16() == 404 {
            Err(LicenseError::NotFound)
        } else {
            let error_text = response.text().await.unwrap_or_default();
            Err(LicenseError::ValidationFailed(error_text))
        }
    }

    /// Removes the license key from this device
    pub async fn remove_license(&self) -> LicenseResult<()> {
        // If there's an activation, deactivate first
        let has_activation = self.credentials.read().await.activation_id.is_some();
        if has_activation {
            // Try to deactivate, but don't fail if it doesn't work
            let _ = self.deactivate_license().await;
        }

        // Clear credentials
        let credentials = LicenseCredentials::default();
        self.config_manager
            .save_license_credentials(&credentials)
            .map_err(|e| LicenseError::ConfigError(e.to_string()))?;

        *self.credentials.write().await = credentials;
        *self.license_info.write().await = LicenseInfo::default();

        Ok(())
    }

    /// Reloads license credentials from disk
    pub async fn reload_credentials(&self) -> LicenseResult<()> {
        let credentials = self
            .config_manager
            .load_license_credentials()
            .map_err(|e| LicenseError::ConfigError(e.to_string()))?;
        *self.credentials.write().await = credentials;

        // If we have a license key, try to validate it
        if self.credentials.read().await.key.is_some() {
            match self.validate_license().await {
                Ok(_) => {}
                Err(LicenseError::NoLicense) => {}
                Err(e) => {
                    tracing::warn!("Failed to validate stored license: {}", e);
                }
            }
        }

        Ok(())
    }

    /// Gets a machine-specific device label for activation
    pub fn get_device_label() -> String {
        // Get hostname
        let hostname = hostname::get()
            .map(|h| h.to_string_lossy().to_string())
            .unwrap_or_else(|_| "unknown".to_string());

        // Get platform info
        let platform = std::env::consts::OS;

        format!("{} ({})", hostname, platform)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_label() {
        let label = LicenseService::get_device_label();
        assert!(!label.is_empty());
        assert!(label.contains("("));
        assert!(label.contains(")"));
    }
}
