//! Server configuration

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Server bind address (e.g., "0.0.0.0:8080")
    #[serde(default = "default_bind_address")]
    pub bind_address: String,

    /// Whether to run in single-user mode (uses SQLite, no auth)
    #[serde(default)]
    pub single_user_mode: bool,

    /// Database URL for PostgreSQL (used in multi-user mode)
    #[serde(default)]
    pub database_url: Option<String>,

    /// SQLite database path (used in single-user mode)
    #[serde(default = "default_database_path")]
    pub database_path: PathBuf,

    /// JWT secret for token signing
    #[serde(default = "default_jwt_secret")]
    pub jwt_secret: String,

    /// JWT token expiration in hours
    #[serde(default = "default_jwt_expiration_hours")]
    pub jwt_expiration_hours: i64,

    /// JWT issuer (for token validation)
    #[serde(default = "default_jwt_issuer")]
    pub jwt_issuer: String,

    /// Whether to enable CORS
    #[serde(default = "default_enable_cors")]
    pub enable_cors: bool,

    /// Allowed CORS origins
    #[serde(default)]
    pub cors_origins: Vec<String>,

    /// Allowed redirect origins for OIDC (prevents open redirect attacks)
    /// Supports wildcard subdomains (e.g., "*.example.com")
    /// If empty, only relative paths are allowed
    #[serde(default)]
    pub allowed_redirect_origins: Vec<String>,

    /// Worker heartbeat timeout in seconds
    #[serde(default = "default_worker_heartbeat_timeout")]
    pub worker_heartbeat_timeout_secs: u64,

    /// Log level (trace, debug, info, warn, error)
    #[serde(default = "default_log_level")]
    pub log_level: String,

    /// OIDC configuration (optional, for OpenID Connect authentication)
    #[serde(default)]
    pub oidc: Option<OidcServerConfig>,
}

/// OIDC configuration for the server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OidcServerConfig {
    /// OIDC provider issuer URL
    pub issuer_url: String,

    /// OAuth2 client ID
    pub client_id: String,

    /// OAuth2 client secret
    pub client_secret: String,

    /// Redirect URL after authentication
    pub redirect_url: String,

    /// Scopes to request
    #[serde(default = "default_oidc_scopes")]
    pub scopes: Vec<String>,
}

fn default_oidc_scopes() -> Vec<String> {
    vec![
        "openid".to_string(),
        "email".to_string(),
        "profile".to_string(),
    ]
}

fn default_bind_address() -> String {
    "0.0.0.0:8080".to_string()
}

fn default_database_path() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("delidev")
        .join("delidev.db")
}

fn default_jwt_secret() -> String {
    // In production, this should be loaded from environment
    "change-me-in-production".to_string()
}

fn default_jwt_expiration_hours() -> i64 {
    24
}

fn default_jwt_issuer() -> String {
    "delidev".to_string()
}

fn default_enable_cors() -> bool {
    true
}

fn default_worker_heartbeat_timeout() -> u64 {
    60
}

fn default_log_level() -> String {
    "info".to_string()
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind_address: default_bind_address(),
            single_user_mode: false,
            database_url: None,
            database_path: default_database_path(),
            jwt_secret: default_jwt_secret(),
            jwt_expiration_hours: default_jwt_expiration_hours(),
            jwt_issuer: default_jwt_issuer(),
            enable_cors: default_enable_cors(),
            cors_origins: Vec::new(),
            allowed_redirect_origins: Vec::new(),
            worker_heartbeat_timeout_secs: default_worker_heartbeat_timeout(),
            log_level: default_log_level(),
            oidc: None,
        }
    }
}

impl ServerConfig {
    /// Load configuration from environment and optional config file
    pub fn load() -> Result<Self, ConfigError> {
        // Load .env file if present
        dotenvy::dotenv().ok();

        // Start with defaults
        let mut config = Self::default();

        // Override with environment variables
        if let Ok(addr) = std::env::var("DELIDEV_BIND_ADDRESS") {
            config.bind_address = addr;
        }

        if let Ok(val) = std::env::var("DELIDEV_SINGLE_USER_MODE") {
            config.single_user_mode = val.parse().unwrap_or(false);
        }

        if let Ok(url) = std::env::var("DATABASE_URL") {
            config.database_url = Some(url);
        }

        if let Ok(path) = std::env::var("DELIDEV_DATABASE_PATH") {
            config.database_path = PathBuf::from(path);
        }

        if let Ok(secret) = std::env::var("DELIDEV_JWT_SECRET") {
            config.jwt_secret = secret;
        }

        if let Ok(hours) = std::env::var("DELIDEV_JWT_EXPIRATION_HOURS") {
            config.jwt_expiration_hours = hours.parse().unwrap_or(24);
        }

        if let Ok(val) = std::env::var("DELIDEV_ENABLE_CORS") {
            config.enable_cors = val.parse().unwrap_or(true);
        }

        if let Ok(origins) = std::env::var("DELIDEV_CORS_ORIGINS") {
            config.cors_origins = origins.split(',').map(|s| s.trim().to_string()).collect();
        }

        if let Ok(origins) = std::env::var("DELIDEV_ALLOWED_REDIRECT_ORIGINS") {
            config.allowed_redirect_origins =
                origins.split(',').map(|s| s.trim().to_string()).collect();
        }

        if let Ok(timeout) = std::env::var("DELIDEV_WORKER_HEARTBEAT_TIMEOUT") {
            config.worker_heartbeat_timeout_secs = timeout.parse().unwrap_or(60);
        }

        if let Ok(level) = std::env::var("DELIDEV_LOG_LEVEL") {
            config.log_level = level;
        }

        if let Ok(issuer) = std::env::var("DELIDEV_JWT_ISSUER") {
            config.jwt_issuer = issuer;
        }

        // Load OIDC config from environment if all required vars are present
        if let (Ok(issuer_url), Ok(client_id), Ok(client_secret), Ok(redirect_url)) = (
            std::env::var("DELIDEV_OIDC_ISSUER_URL"),
            std::env::var("DELIDEV_OIDC_CLIENT_ID"),
            std::env::var("DELIDEV_OIDC_CLIENT_SECRET"),
            std::env::var("DELIDEV_OIDC_REDIRECT_URL"),
        ) {
            let scopes = std::env::var("DELIDEV_OIDC_SCOPES")
                .map(|s| s.split(',').map(|s| s.trim().to_string()).collect())
                .unwrap_or_else(|_| default_oidc_scopes());

            config.oidc = Some(OidcServerConfig {
                issuer_url,
                client_id,
                client_secret,
                redirect_url,
                scopes,
            });
        }

        // Try to load from config file
        if let Some(config_path) = Self::find_config_file() {
            if let Ok(contents) = std::fs::read_to_string(&config_path) {
                if let Ok(file_config) = toml::from_str::<ServerConfig>(&contents) {
                    // Merge file config (file takes precedence over defaults, env takes precedence
                    // over file)
                    if config.bind_address == default_bind_address() {
                        config.bind_address = file_config.bind_address;
                    }
                    if !config.single_user_mode {
                        config.single_user_mode = file_config.single_user_mode;
                    }
                    if config.database_url.is_none() {
                        config.database_url = file_config.database_url;
                    }
                    if config.cors_origins.is_empty() {
                        config.cors_origins = file_config.cors_origins;
                    }
                    if config.allowed_redirect_origins.is_empty() {
                        config.allowed_redirect_origins = file_config.allowed_redirect_origins;
                    }
                }
            }
        }

        config.validate()?;
        Ok(config)
    }

    /// Find the config file in standard locations
    fn find_config_file() -> Option<PathBuf> {
        let locations = [
            PathBuf::from("delidev-server.toml"),
            PathBuf::from("/etc/delidev/server.toml"),
            dirs::config_dir()
                .map(|p| p.join("delidev").join("server.toml"))
                .unwrap_or_default(),
        ];

        locations.into_iter().find(|p| p.exists())
    }

    /// Validate the configuration
    fn validate(&self) -> Result<(), ConfigError> {
        if !self.single_user_mode && self.database_url.is_none() {
            return Err(ConfigError::MissingDatabaseUrl);
        }

        if self.jwt_secret == "change-me-in-production" && !self.single_user_mode {
            tracing::warn!("Using default JWT secret in multi-user mode is insecure!");
        }

        Ok(())
    }
}

/// Configuration errors
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Database URL is required for multi-user mode")]
    MissingDatabaseUrl,

    #[error("Failed to read config file: {0}")]
    FileRead(#[from] std::io::Error),

    #[error("Failed to parse config file: {0}")]
    Parse(#[from] toml::de::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ServerConfig::default();
        assert_eq!(config.bind_address, "0.0.0.0:8080");
        assert!(!config.single_user_mode);
        assert_eq!(config.jwt_expiration_hours, 24);
    }
}
