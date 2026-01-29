use serde::{Deserialize, Serialize};

/// Polar.sh organization ID for DeliDev
/// This is the organization ID used to validate license keys
pub const POLAR_ORGANIZATION_ID: &str = "d189c776-0bf6-47ae-aa93-2fbb7c03e479";

/// License key status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum LicenseStatus {
    /// License is valid and active
    Active,
    /// License has expired
    Expired,
    /// License key is invalid
    Invalid,
    /// License has been revoked
    Revoked,
    /// License validation is pending
    Pending,
    /// No license configured
    #[default]
    NotConfigured,
}

/// Information about the current license
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LicenseInfo {
    /// The license key (masked for display)
    pub display_key: Option<String>,
    /// Current status of the license
    pub status: LicenseStatus,
    /// Customer email associated with the license
    pub customer_email: Option<String>,
    /// Customer name
    pub customer_name: Option<String>,
    /// When the license expires (ISO 8601)
    pub expires_at: Option<String>,
    /// Last validation timestamp (ISO 8601)
    pub last_validated_at: Option<String>,
    /// Number of activations used
    pub activations_used: Option<u32>,
    /// Maximum activations allowed
    pub activation_limit: Option<u32>,
    /// Activation ID for this device
    pub activation_id: Option<String>,
}

/// Stored license credentials
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LicenseCredentials {
    /// The license key
    pub key: Option<String>,
    /// Activation ID for this device (received from Polar after activation)
    pub activation_id: Option<String>,
    /// Device label used during activation
    pub device_label: Option<String>,
}

/// Request body for validating a license key with Polar.sh
#[derive(Debug, Serialize)]
pub struct PolarValidateRequest {
    /// The license key to validate
    pub key: String,
    /// The Polar organization ID
    pub organization_id: String,
    /// Optional activation ID for activated licenses
    #[serde(skip_serializing_if = "Option::is_none")]
    pub activation_id: Option<String>,
}

/// Request body for activating a license key with Polar.sh
#[derive(Debug, Serialize)]
pub struct PolarActivateRequest {
    /// The license key to activate
    pub key: String,
    /// The Polar organization ID
    pub organization_id: String,
    /// Label for this activation (e.g., device name)
    pub label: String,
    /// Optional conditions for activation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conditions: Option<serde_json::Value>,
    /// Optional metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<serde_json::Value>,
}

/// Request body for deactivating a license key with Polar.sh
#[derive(Debug, Serialize)]
pub struct PolarDeactivateRequest {
    /// The license key
    pub key: String,
    /// The Polar organization ID
    pub organization_id: String,
    /// The activation ID to deactivate
    pub activation_id: String,
}

/// Customer information from Polar.sh
#[derive(Debug, Clone, Deserialize)]
pub struct PolarCustomer {
    pub id: String,
    pub email: Option<String>,
    pub name: Option<String>,
}

/// Activation information from Polar.sh
#[derive(Debug, Clone, Deserialize)]
pub struct PolarActivation {
    pub id: String,
    pub license_key_id: String,
    pub label: String,
    #[serde(default)]
    pub meta: serde_json::Value,
    pub created_at: String,
    pub modified_at: Option<String>,
}

/// Response from Polar.sh license key validation
#[derive(Debug, Clone, Deserialize)]
pub struct PolarValidateResponse {
    pub id: String,
    pub created_at: String,
    pub modified_at: Option<String>,
    pub organization_id: String,
    pub customer_id: String,
    pub customer: Option<PolarCustomer>,
    pub benefit_id: String,
    pub key: String,
    pub display_key: String,
    /// Status can be "granted", "revoked", or "disabled"
    pub status: String,
    pub limit_activations: Option<u32>,
    pub usage: Option<u32>,
    pub limit_usage: Option<u32>,
    pub validations: Option<u32>,
    pub last_validated_at: Option<String>,
    pub expires_at: Option<String>,
    pub activation: Option<PolarActivation>,
}

/// Response from Polar.sh license key activation
#[derive(Debug, Clone, Deserialize)]
pub struct PolarActivateResponse {
    pub id: String,
    pub license_key_id: String,
    pub label: String,
    #[serde(default)]
    pub meta: serde_json::Value,
    pub created_at: String,
    pub modified_at: Option<String>,
    pub license_key: Option<PolarValidateResponse>,
}

impl From<&PolarValidateResponse> for LicenseInfo {
    fn from(response: &PolarValidateResponse) -> Self {
        let status = match response.status.as_str() {
            "granted" => {
                // Check if expired
                if let Some(expires_at) = &response.expires_at {
                    if let Ok(expiry) = chrono::DateTime::parse_from_rfc3339(expires_at) {
                        if expiry < chrono::Utc::now() {
                            LicenseStatus::Expired
                        } else {
                            LicenseStatus::Active
                        }
                    } else {
                        LicenseStatus::Active
                    }
                } else {
                    LicenseStatus::Active
                }
            }
            "revoked" => LicenseStatus::Revoked,
            "disabled" => LicenseStatus::Invalid,
            _ => LicenseStatus::Invalid,
        };

        LicenseInfo {
            display_key: Some(response.display_key.clone()),
            status,
            customer_email: response.customer.as_ref().and_then(|c| c.email.clone()),
            customer_name: response.customer.as_ref().and_then(|c| c.name.clone()),
            expires_at: response.expires_at.clone(),
            last_validated_at: response.last_validated_at.clone(),
            activations_used: response.limit_activations.map(|_limit| {
                // Polar doesn't directly provide used count, so we infer
                // If we have an activation, at least 1 is used
                if response.activation.is_some() {
                    1
                } else {
                    0
                }
            }),
            activation_limit: response.limit_activations,
            activation_id: response.activation.as_ref().map(|a| a.id.clone()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_license_status_serialization() {
        let status = LicenseStatus::Active;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, "\"active\"");

        let deserialized: LicenseStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, LicenseStatus::Active);
    }

    #[test]
    fn test_license_info_default() {
        let info = LicenseInfo::default();
        assert_eq!(info.status, LicenseStatus::NotConfigured);
        assert!(info.display_key.is_none());
    }

    #[test]
    fn test_polar_validate_request_serialization() {
        let request = PolarValidateRequest {
            key: "test-key".to_string(),
            organization_id: "test-org".to_string(),
            activation_id: Some("test-activation".to_string()),
        };
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("test-key"));
        assert!(json.contains("organization_id"));
    }
}
