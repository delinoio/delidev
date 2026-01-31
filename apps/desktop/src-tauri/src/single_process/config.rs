//! Configuration for single-process mode
//!
//! This module defines configuration options specific to single-process mode.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// The process mode for the desktop application
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ProcessMode {
    /// Single process mode: server, worker, and client all in one process
    #[default]
    SingleProcess,

    /// Client mode: connects to a remote server
    Remote,
}

/// Configuration for single-process mode
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SingleProcessConfig {
    /// The process mode
    #[serde(default)]
    pub mode: ProcessMode,

    /// SQLite database path for single-process mode
    #[serde(default = "default_database_path")]
    pub database_path: PathBuf,

    /// Remote server URL (used when mode is Remote)
    #[serde(default)]
    pub server_url: Option<String>,

    /// Whether to enable debug logging
    #[serde(default)]
    pub debug: bool,
}

fn default_database_path() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("delidev")
        .join("delidev.db")
}

impl Default for SingleProcessConfig {
    fn default() -> Self {
        Self {
            mode: ProcessMode::SingleProcess,
            database_path: default_database_path(),
            server_url: None,
            debug: false,
        }
    }
}

impl SingleProcessConfig {
    /// Creates a new configuration for single-process mode
    pub fn single_process() -> Self {
        Self {
            mode: ProcessMode::SingleProcess,
            ..Default::default()
        }
    }

    /// Creates a new configuration for remote mode
    pub fn remote(server_url: String) -> Self {
        Self {
            mode: ProcessMode::Remote,
            server_url: Some(server_url),
            ..Default::default()
        }
    }

    /// Checks if running in single-process mode
    pub fn is_single_process(&self) -> bool {
        self.mode == ProcessMode::SingleProcess
    }

    /// Checks if running in remote mode
    pub fn is_remote(&self) -> bool {
        self.mode == ProcessMode::Remote
    }
}
