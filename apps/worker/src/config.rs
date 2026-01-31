//! Worker configuration

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Worker configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerConfig {
    /// Main server URL
    #[serde(default = "default_server_url")]
    pub main_server_url: String,

    /// Worker ID (auto-generated if not specified)
    #[serde(default)]
    pub worker_id: Option<String>,

    /// Worker bind address for health checks
    #[serde(default = "default_bind_address")]
    pub bind_address: String,

    /// Maximum concurrent tasks
    #[serde(default = "default_max_concurrent_tasks")]
    pub max_concurrent_tasks: u32,

    /// Heartbeat interval in seconds
    #[serde(default = "default_heartbeat_interval")]
    pub heartbeat_interval_secs: u64,

    /// Container runtime (docker or podman)
    #[serde(default = "default_container_runtime")]
    pub container_runtime: String,

    /// Use container for execution
    #[serde(default = "default_use_container")]
    pub use_container: bool,

    /// Worktree directory for task execution
    #[serde(default = "default_worktree_dir")]
    pub worktree_dir: PathBuf,

    /// Log level
    #[serde(default = "default_log_level")]
    pub log_level: String,
}

fn default_server_url() -> String {
    "http://localhost:8080".to_string()
}

fn default_bind_address() -> String {
    "0.0.0.0:9000".to_string()
}

fn default_max_concurrent_tasks() -> u32 {
    4
}

fn default_heartbeat_interval() -> u64 {
    30
}

fn default_container_runtime() -> String {
    "docker".to_string()
}

fn default_use_container() -> bool {
    true
}

fn default_worktree_dir() -> PathBuf {
    PathBuf::from("/tmp/delidev/worktrees")
}

fn default_log_level() -> String {
    "info".to_string()
}

impl Default for WorkerConfig {
    fn default() -> Self {
        Self {
            main_server_url: default_server_url(),
            worker_id: None,
            bind_address: default_bind_address(),
            max_concurrent_tasks: default_max_concurrent_tasks(),
            heartbeat_interval_secs: default_heartbeat_interval(),
            container_runtime: default_container_runtime(),
            use_container: default_use_container(),
            worktree_dir: default_worktree_dir(),
            log_level: default_log_level(),
        }
    }
}

impl WorkerConfig {
    /// Load configuration from environment and optional config file
    pub fn load() -> Result<Self, ConfigError> {
        // Load .env file if present
        dotenvy::dotenv().ok();

        // Start with defaults
        let mut config = Self::default();

        // Override with environment variables
        if let Ok(url) = std::env::var("DELIDEV_SERVER_URL") {
            config.main_server_url = url;
        }

        if let Ok(id) = std::env::var("DELIDEV_WORKER_ID") {
            config.worker_id = Some(id);
        }

        if let Ok(addr) = std::env::var("DELIDEV_WORKER_BIND_ADDRESS") {
            config.bind_address = addr;
        }

        if let Ok(max) = std::env::var("DELIDEV_WORKER_MAX_TASKS") {
            config.max_concurrent_tasks = max.parse().unwrap_or(4);
        }

        if let Ok(interval) = std::env::var("DELIDEV_WORKER_HEARTBEAT_INTERVAL") {
            config.heartbeat_interval_secs = interval.parse().unwrap_or(30);
        }

        if let Ok(runtime) = std::env::var("DELIDEV_CONTAINER_RUNTIME") {
            config.container_runtime = runtime;
        }

        if let Ok(val) = std::env::var("DELIDEV_USE_CONTAINER") {
            config.use_container = val.parse().unwrap_or(true);
        }

        if let Ok(dir) = std::env::var("DELIDEV_WORKTREE_DIR") {
            config.worktree_dir = PathBuf::from(dir);
        }

        if let Ok(level) = std::env::var("DELIDEV_LOG_LEVEL") {
            config.log_level = level;
        }

        // Try to load from config file
        if let Some(config_path) = Self::find_config_file() {
            if let Ok(contents) = std::fs::read_to_string(&config_path) {
                if let Ok(file_config) = toml::from_str::<WorkerConfig>(&contents) {
                    // Merge file config (env takes precedence)
                    if config.main_server_url == default_server_url() {
                        config.main_server_url = file_config.main_server_url;
                    }
                    if config.worker_id.is_none() {
                        config.worker_id = file_config.worker_id;
                    }
                }
            }
        }

        // Generate worker ID if not specified
        if config.worker_id.is_none() {
            config.worker_id = Some(uuid::Uuid::new_v4().to_string());
        }

        Ok(config)
    }

    /// Find the config file in standard locations
    fn find_config_file() -> Option<PathBuf> {
        let locations = [
            PathBuf::from("delidev-worker.toml"),
            PathBuf::from("/etc/delidev/worker.toml"),
            dirs::config_dir()
                .map(|p| p.join("delidev").join("worker.toml"))
                .unwrap_or_default(),
        ];

        locations.into_iter().find(|p| p.exists())
    }

    /// Get the worker ID (always returns a value after load)
    pub fn worker_id(&self) -> &str {
        self.worker_id.as_deref().expect("Worker ID should be set after load")
    }
}

/// Configuration errors
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
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
        let config = WorkerConfig::default();
        assert_eq!(config.main_server_url, "http://localhost:8080");
        assert_eq!(config.max_concurrent_tasks, 4);
        assert!(config.use_container);
    }
}
