use std::{path::PathBuf, sync::Arc};

use tauri::State;

use crate::{
    config::ConfigManager,
    entities::{GlobalConfig, RepositoryConfig, VCSProviderType},
    services::{AppState, VCSUser},
};

/// Gets global configuration
#[tauri::command]
pub async fn get_global_config(state: State<'_, Arc<AppState>>) -> Result<GlobalConfig, String> {
    let config = state.global_config.read().await;
    Ok(config.clone())
}

/// Updates global configuration
#[tauri::command]
pub async fn update_global_config(
    state: State<'_, Arc<AppState>>,
    config: GlobalConfig,
) -> Result<(), String> {
    state
        .update_global_config(config)
        .await
        .map_err(|e| e.to_string())
}

/// Gets repository-specific configuration
#[tauri::command]
pub async fn get_repository_config(repo_path: String) -> Result<RepositoryConfig, String> {
    ConfigManager::load_repository_config(&PathBuf::from(repo_path)).map_err(|e| e.to_string())
}

/// Updates repository-specific configuration
#[tauri::command]
pub async fn update_repository_config(
    repo_path: String,
    config: RepositoryConfig,
) -> Result<(), String> {
    ConfigManager::save_repository_config(&PathBuf::from(repo_path), &config)
        .map_err(|e| e.to_string())
}

/// Gets VCS credentials status (not the actual tokens)
#[tauri::command]
pub async fn get_credentials_status(
    state: State<'_, Arc<AppState>>,
) -> Result<CredentialsStatus, String> {
    let creds = state.credentials.read().await;
    Ok(CredentialsStatus {
        github_configured: creds.github.is_some(),
        gitlab_configured: creds.gitlab.is_some(),
        bitbucket_configured: creds.bitbucket.is_some(),
    })
}

#[derive(serde::Serialize)]
pub struct CredentialsStatus {
    pub github_configured: bool,
    pub gitlab_configured: bool,
    pub bitbucket_configured: bool,
}

/// Sets GitHub credentials
#[tauri::command]
pub async fn set_github_token(
    state: State<'_, Arc<AppState>>,
    token: String,
) -> Result<VCSUser, String> {
    use crate::entities::GitHubCredentials;

    let github_creds = GitHubCredentials { token };

    // Validate token
    let user = state
        .vcs_service
        .validate_github(&github_creds)
        .await
        .map_err(|e| e.to_string())?;

    // Save credentials
    let mut creds = state.credentials.write().await;
    creds.github = Some(github_creds);
    state
        .config_manager
        .save_credentials(&creds)
        .map_err(|e| e.to_string())?;

    Ok(user)
}

/// Sets GitLab credentials
#[tauri::command]
pub async fn set_gitlab_token(
    state: State<'_, Arc<AppState>>,
    token: String,
) -> Result<VCSUser, String> {
    use crate::entities::GitLabCredentials;

    let gitlab_creds = GitLabCredentials { token };

    // Validate token
    let user = state
        .vcs_service
        .validate_gitlab(&gitlab_creds)
        .await
        .map_err(|e| e.to_string())?;

    // Save credentials
    let mut creds = state.credentials.write().await;
    creds.gitlab = Some(gitlab_creds);
    state
        .config_manager
        .save_credentials(&creds)
        .map_err(|e| e.to_string())?;

    Ok(user)
}

/// Sets Bitbucket credentials
#[tauri::command]
pub async fn set_bitbucket_credentials(
    state: State<'_, Arc<AppState>>,
    username: String,
    app_password: String,
) -> Result<VCSUser, String> {
    use crate::entities::BitbucketCredentials;

    let bitbucket_creds = BitbucketCredentials {
        username,
        app_password,
    };

    // Validate credentials
    let user = state
        .vcs_service
        .validate_bitbucket(&bitbucket_creds)
        .await
        .map_err(|e| e.to_string())?;

    // Save credentials
    let mut creds = state.credentials.write().await;
    creds.bitbucket = Some(bitbucket_creds);
    state
        .config_manager
        .save_credentials(&creds)
        .map_err(|e| e.to_string())?;

    Ok(user)
}

/// Validates VCS credentials for a provider
#[tauri::command]
pub async fn validate_vcs_credentials(
    state: State<'_, Arc<AppState>>,
    provider: VCSProviderType,
) -> Result<VCSUser, String> {
    let creds = state.credentials.read().await;

    match provider {
        VCSProviderType::GitHub => {
            let github_creds = creds
                .github
                .as_ref()
                .ok_or("GitHub credentials not configured")?;
            state
                .vcs_service
                .validate_github(github_creds)
                .await
                .map_err(|e| e.to_string())
        }
        VCSProviderType::GitLab => {
            let gitlab_creds = creds
                .gitlab
                .as_ref()
                .ok_or("GitLab credentials not configured")?;
            state
                .vcs_service
                .validate_gitlab(gitlab_creds)
                .await
                .map_err(|e| e.to_string())
        }
        VCSProviderType::Bitbucket => {
            let bitbucket_creds = creds
                .bitbucket
                .as_ref()
                .ok_or("Bitbucket credentials not configured")?;
            state
                .vcs_service
                .validate_bitbucket(bitbucket_creds)
                .await
                .map_err(|e| e.to_string())
        }
    }
}
