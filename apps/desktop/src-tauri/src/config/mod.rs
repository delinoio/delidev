use std::path::{Path, PathBuf};

use thiserror::Error;

use crate::entities::{GlobalConfig, RepositoryConfig, VCSCredentials};

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Failed to read config file: {0}")]
    ReadError(#[from] std::io::Error),
    #[error("Failed to parse config file: {0}")]
    ParseError(#[from] toml::de::Error),
    #[error("Failed to serialize config: {0}")]
    SerializeError(#[from] toml::ser::Error),
    #[error("Config directory not found")]
    ConfigDirNotFound,
}

pub type ConfigResult<T> = Result<T, ConfigError>;

/// Configuration manager
pub struct ConfigManager {
    /// Path to global config directory (~/.delidev/)
    global_config_dir: PathBuf,
}

impl ConfigManager {
    /// Creates a new config manager
    pub fn new() -> ConfigResult<Self> {
        let global_config_dir = Self::get_global_config_dir()?;

        // Ensure global config directory exists
        std::fs::create_dir_all(&global_config_dir)?;

        Ok(Self { global_config_dir })
    }

    /// Returns the global config directory path
    fn get_global_config_dir() -> ConfigResult<PathBuf> {
        dirs::home_dir()
            .map(|p| p.join(".delidev"))
            .ok_or(ConfigError::ConfigDirNotFound)
    }

    /// Returns path to global config file
    pub fn global_config_path(&self) -> PathBuf {
        self.global_config_dir.join("config.toml")
    }

    /// Returns path to credentials file
    pub fn credentials_path(&self) -> PathBuf {
        self.global_config_dir.join("credentials.toml")
    }

    /// Returns path to database file
    pub fn database_path(&self) -> PathBuf {
        self.global_config_dir.join("delidev.db")
    }

    /// Loads global configuration
    pub fn load_global_config(&self) -> ConfigResult<GlobalConfig> {
        let path = self.global_config_path();
        if !path.exists() {
            return Ok(GlobalConfig::default());
        }

        let content = std::fs::read_to_string(&path)?;
        let config: GlobalConfig = toml::from_str(&content)?;
        Ok(config)
    }

    /// Saves global configuration
    pub fn save_global_config(&self, config: &GlobalConfig) -> ConfigResult<()> {
        let path = self.global_config_path();
        let content = toml::to_string_pretty(config)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Loads VCS credentials
    pub fn load_credentials(&self) -> ConfigResult<VCSCredentials> {
        let path = self.credentials_path();
        if !path.exists() {
            return Ok(VCSCredentials::default());
        }

        let content = std::fs::read_to_string(&path)?;
        let creds: VCSCredentials = toml::from_str(&content)?;
        Ok(creds)
    }

    /// Saves VCS credentials
    pub fn save_credentials(&self, creds: &VCSCredentials) -> ConfigResult<()> {
        let path = self.credentials_path();
        let content = toml::to_string_pretty(creds)?;
        std::fs::write(&path, content)?;

        // Set restrictive permissions on credentials file (Unix only)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&path)?.permissions();
            perms.set_mode(0o600);
            std::fs::set_permissions(&path, perms)?;
        }

        #[cfg(windows)]
        {
            tracing::warn!(
                "Credentials file saved at {:?}. On Windows, please ensure file permissions are \
                 restricted to your user account only.",
                path
            );
        }

        Ok(())
    }

    /// Loads repository-specific configuration
    pub fn load_repository_config(repo_path: &Path) -> ConfigResult<RepositoryConfig> {
        let config_path = repo_path.join(".delidev").join("config.toml");
        if !config_path.exists() {
            return Ok(RepositoryConfig::default());
        }

        let content = std::fs::read_to_string(&config_path)?;
        let config: RepositoryConfig = toml::from_str(&content)?;
        Ok(config)
    }

    /// Saves repository-specific configuration
    pub fn save_repository_config(repo_path: &Path, config: &RepositoryConfig) -> ConfigResult<()> {
        let config_dir = repo_path.join(".delidev");
        std::fs::create_dir_all(&config_dir)?;

        let config_path = config_dir.join("config.toml");
        let content = toml::to_string_pretty(config)?;
        std::fs::write(config_path, content)?;
        Ok(())
    }

    /// Initializes repository with default configuration
    pub fn init_repository_config(repo_path: &Path) -> ConfigResult<RepositoryConfig> {
        let config = RepositoryConfig::default();
        Self::save_repository_config(repo_path, &config)?;
        Ok(config)
    }
}

impl Default for ConfigManager {
    fn default() -> Self {
        Self::new().expect("Failed to create config manager")
    }
}

/// Gets effective configuration by merging global and repository configs
pub struct EffectiveConfig {
    pub global: GlobalConfig,
    pub repository: RepositoryConfig,
}

impl EffectiveConfig {
    pub fn new(global: GlobalConfig, repository: RepositoryConfig) -> Self {
        Self { global, repository }
    }

    /// Returns effective auto_learn_from_reviews setting
    pub fn auto_learn_from_reviews(&self) -> bool {
        self.repository.effective_auto_learn(&self.global)
    }

    /// Returns branch name for a task
    pub fn branch_name(&self, task_id: &str, slug: &str) -> String {
        self.repository.generate_branch_name(task_id, slug)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_global_config() {
        let config = GlobalConfig::default();
        assert!(!config.learning.auto_learn_from_reviews);
        assert_eq!(config.hotkey.open_chat, "Option+Z");
    }

    #[test]
    fn test_default_repository_config() {
        let config = RepositoryConfig::default();
        assert!(config.automation.auto_fix_review_comments);
        assert!(config.automation.auto_fix_ci_failures);
        assert_eq!(config.automation.max_auto_fix_attempts, 3);
    }

    #[test]
    fn test_branch_name_generation() {
        let config = RepositoryConfig::default();
        let branch = config.generate_branch_name("abc123", "add-feature");
        assert_eq!(branch, "delidev/abc123");
    }
}
