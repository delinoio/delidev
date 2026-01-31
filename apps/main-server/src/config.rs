//! Server configuration.

use std::env;

/// Server configuration loaded from environment variables.
#[derive(Debug, Clone)]
pub struct Config {
    /// Server host address.
    pub host: String,
    /// Server port.
    pub port: u16,
    /// Database URL.
    pub database_url: String,
    /// Whether running in single-user mode.
    pub single_user_mode: bool,
    /// JWT secret (required in multi-user mode).
    pub jwt_secret: Option<String>,
    /// JWT expiration in hours.
    pub jwt_expiration_hours: u64,
    /// OIDC issuer URL.
    pub oidc_issuer_url: Option<String>,
    /// OIDC client ID.
    pub oidc_client_id: Option<String>,
    /// OIDC client secret.
    pub oidc_client_secret: Option<String>,
    /// OIDC redirect URL.
    pub oidc_redirect_url: Option<String>,
    /// Log level.
    pub log_level: String,
}

impl Config {
    /// Loads configuration from environment variables.
    pub fn from_env() -> anyhow::Result<Self> {
        let single_user_mode = env::var("DELIDEV_SINGLE_USER_MODE")
            .map(|v| v.to_lowercase() == "true" || v == "1")
            .unwrap_or(true);

        let database_url = if single_user_mode {
            env::var("DATABASE_URL")
                .unwrap_or_else(|_| "sqlite:~/.delidev/data.db?mode=rwc".to_string())
        } else {
            env::var("DATABASE_URL").map_err(|_| {
                anyhow::anyhow!("DATABASE_URL is required in multi-user mode")
            })?
        };

        let jwt_secret = env::var("DELIDEV_JWT_SECRET").ok();
        if !single_user_mode && jwt_secret.is_none() {
            anyhow::bail!("DELIDEV_JWT_SECRET is required in multi-user mode");
        }

        Ok(Self {
            host: env::var("DELIDEV_SERVER_HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            port: env::var("DELIDEV_SERVER_PORT")
                .unwrap_or_else(|_| "54871".to_string())
                .parse()
                .unwrap_or(54871),
            database_url,
            single_user_mode,
            jwt_secret,
            jwt_expiration_hours: env::var("DELIDEV_JWT_EXPIRATION_HOURS")
                .unwrap_or_else(|_| "24".to_string())
                .parse()
                .unwrap_or(24),
            oidc_issuer_url: env::var("DELIDEV_OIDC_ISSUER_URL").ok(),
            oidc_client_id: env::var("DELIDEV_OIDC_CLIENT_ID").ok(),
            oidc_client_secret: env::var("DELIDEV_OIDC_CLIENT_SECRET").ok(),
            oidc_redirect_url: env::var("DELIDEV_OIDC_REDIRECT_URL").ok(),
            log_level: env::var("DELIDEV_LOG_LEVEL").unwrap_or_else(|_| "info".to_string()),
        })
    }

    /// Returns the server address.
    pub fn server_addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }

    /// Returns true if authentication should be enabled.
    pub fn auth_enabled(&self) -> bool {
        !self.single_user_mode
    }

    /// Returns true if OIDC is configured.
    pub fn oidc_configured(&self) -> bool {
        self.oidc_issuer_url.is_some()
            && self.oidc_client_id.is_some()
            && self.oidc_client_secret.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_single_user_mode() {
        // Clear any existing env vars
        // SAFETY: Tests run serially or in isolation
        unsafe {
            env::remove_var("DELIDEV_SINGLE_USER_MODE");
            env::remove_var("DATABASE_URL");
        }

        let config = Config::from_env().unwrap();
        assert!(config.single_user_mode);
        assert!(!config.auth_enabled());
    }
}
