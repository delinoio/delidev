use std::sync::Arc;

use tauri::State;

use crate::{
    entities::LicenseInfo,
    services::{AppState, LicenseService},
};

/// Gets the current license information
#[tauri::command]
pub async fn get_license_info(state: State<'_, Arc<AppState>>) -> Result<LicenseInfo, String> {
    Ok(state.license_service.get_license_info().await)
}

/// Checks if a license is configured
#[tauri::command]
pub async fn has_license(state: State<'_, Arc<AppState>>) -> Result<bool, String> {
    Ok(state.license_service.has_license().await)
}

/// Checks if the license is valid and active
#[tauri::command]
pub async fn is_license_valid(state: State<'_, Arc<AppState>>) -> Result<bool, String> {
    Ok(state.license_service.is_license_valid().await)
}

/// Validates the current license key with Polar.sh
#[tauri::command]
pub async fn validate_license(state: State<'_, Arc<AppState>>) -> Result<LicenseInfo, String> {
    state
        .license_service
        .validate_license()
        .await
        .map_err(|e| e.to_string())
}

/// Activates a license key for this device
#[tauri::command]
pub async fn activate_license(
    state: State<'_, Arc<AppState>>,
    key: String,
    device_label: Option<String>,
) -> Result<LicenseInfo, String> {
    let label = device_label.unwrap_or_else(LicenseService::get_device_label);
    state
        .license_service
        .activate_license(&key, &label)
        .await
        .map_err(|e| e.to_string())
}

/// Deactivates the license for this device
#[tauri::command]
pub async fn deactivate_license(state: State<'_, Arc<AppState>>) -> Result<(), String> {
    state
        .license_service
        .deactivate_license()
        .await
        .map_err(|e| e.to_string())
}

/// Sets a license key without activation (for keys that don't require
/// activation)
#[tauri::command]
pub async fn set_license_key(
    state: State<'_, Arc<AppState>>,
    key: String,
) -> Result<LicenseInfo, String> {
    state
        .license_service
        .set_license_key(&key)
        .await
        .map_err(|e| e.to_string())
}

/// Removes the license key from this device
#[tauri::command]
pub async fn remove_license(state: State<'_, Arc<AppState>>) -> Result<(), String> {
    state
        .license_service
        .remove_license()
        .await
        .map_err(|e| e.to_string())
}

/// Gets the suggested device label for activation
#[tauri::command]
pub async fn get_device_label() -> Result<String, String> {
    Ok(LicenseService::get_device_label())
}
