//! Docker sandboxing for AI coding agents

use std::{collections::HashMap, path::PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Container runtime type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ContainerRuntime {
    #[default]
    Docker,
    Podman,
}

impl ContainerRuntime {
    /// Returns the command for this runtime
    pub fn command(&self) -> &'static str {
        match self {
            Self::Docker => "docker",
            Self::Podman => "podman",
        }
    }
}

/// Errors that can occur during sandbox operations
#[derive(Error, Debug)]
pub enum SandboxError {
    #[error("Container runtime not available: {0}")]
    RuntimeNotAvailable(String),

    #[error("Failed to create sandbox: {0}")]
    CreateFailed(String),

    #[error("Failed to execute in sandbox: {0}")]
    ExecuteFailed(String),

    #[error("Sandbox not running")]
    NotRunning,

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Timeout after {0} seconds")]
    Timeout(u64),
}

/// Configuration for creating a sandbox
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxConfig {
    /// Container runtime to use
    pub runtime: ContainerRuntime,

    /// Docker image to use
    pub image: String,

    /// Working directory inside the container
    pub work_dir: PathBuf,

    /// Volumes to mount (host_path -> container_path)
    pub volumes: HashMap<String, String>,

    /// Environment variables
    pub env: HashMap<String, String>,

    /// Network mode (e.g., "none", "host", "bridge")
    pub network_mode: Option<String>,

    /// Memory limit (e.g., "2g")
    pub memory_limit: Option<String>,

    /// CPU limit (e.g., "2.0")
    pub cpu_limit: Option<String>,

    /// Whether to run in privileged mode
    pub privileged: bool,

    /// Custom container socket path
    pub socket_path: Option<String>,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            runtime: ContainerRuntime::default(),
            image: "node:20-slim".to_string(),
            work_dir: PathBuf::from("/workspace"),
            volumes: HashMap::new(),
            env: HashMap::new(),
            network_mode: None,
            memory_limit: Some("4g".to_string()),
            cpu_limit: Some("4.0".to_string()),
            privileged: false,
            socket_path: None,
        }
    }
}

impl SandboxConfig {
    /// Creates a new sandbox config with the given image
    pub fn new(image: impl Into<String>) -> Self {
        Self {
            image: image.into(),
            ..Default::default()
        }
    }

    /// Sets the runtime
    pub fn with_runtime(mut self, runtime: ContainerRuntime) -> Self {
        self.runtime = runtime;
        self
    }

    /// Sets the working directory
    pub fn with_work_dir(mut self, work_dir: PathBuf) -> Self {
        self.work_dir = work_dir;
        self
    }

    /// Adds a volume mount
    pub fn with_volume(
        mut self,
        host_path: impl Into<String>,
        container_path: impl Into<String>,
    ) -> Self {
        self.volumes.insert(host_path.into(), container_path.into());
        self
    }

    /// Adds an environment variable
    pub fn with_env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.insert(key.into(), value.into());
        self
    }

    /// Sets the network mode
    pub fn with_network_mode(mut self, mode: impl Into<String>) -> Self {
        self.network_mode = Some(mode.into());
        self
    }

    /// Sets the memory limit
    pub fn with_memory_limit(mut self, limit: impl Into<String>) -> Self {
        self.memory_limit = Some(limit.into());
        self
    }

    /// Sets the CPU limit
    pub fn with_cpu_limit(mut self, limit: impl Into<String>) -> Self {
        self.cpu_limit = Some(limit.into());
        self
    }
}

/// Handle to a running execution in the sandbox
#[derive(Debug)]
pub struct ExecHandle {
    /// Container ID
    pub container_id: String,

    /// Exit code (if completed)
    pub exit_code: Option<i32>,
}

/// Docker sandbox for isolated agent execution
#[derive(Debug)]
pub struct DockerSandbox {
    /// Configuration
    config: SandboxConfig,

    /// Container ID (if running)
    container_id: Option<String>,
}

impl DockerSandbox {
    /// Creates a new sandbox with the given configuration
    ///
    /// Note: This does not start the container. Call `start()` to create and
    /// start the container.
    pub fn new(config: SandboxConfig) -> Self {
        Self {
            config,
            container_id: None,
        }
    }

    /// Returns the sandbox configuration
    pub fn config(&self) -> &SandboxConfig {
        &self.config
    }

    /// Returns the container ID if running
    pub fn container_id(&self) -> Option<&str> {
        self.container_id.as_deref()
    }

    /// Returns true if the sandbox is running
    pub fn is_running(&self) -> bool {
        self.container_id.is_some()
    }

    /// Starts the sandbox container
    ///
    /// This is an async operation that will be implemented by platform-specific
    /// code. The actual Docker/Podman API calls will be made by the desktop
    /// app or worker server.
    pub async fn start(&mut self) -> Result<(), SandboxError> {
        // This is a stub - actual implementation will use bollard or similar
        // For now, we just generate a mock container ID
        self.container_id = Some(format!("sandbox-{}", uuid::Uuid::new_v4()));
        Ok(())
    }

    /// Executes a command in the sandbox
    pub async fn exec(
        &self,
        _command: &str,
        _env: HashMap<String, String>,
    ) -> Result<ExecHandle, SandboxError> {
        let container_id = self.container_id.as_ref().ok_or(SandboxError::NotRunning)?;

        // This is a stub - actual implementation will use bollard or similar
        Ok(ExecHandle {
            container_id: container_id.clone(),
            exit_code: None,
        })
    }

    /// Stops and removes the sandbox container
    pub async fn destroy(mut self) -> Result<(), SandboxError> {
        // This is a stub - actual implementation will use bollard or similar
        self.container_id = None;
        Ok(())
    }

    /// Builds command-line arguments for creating the container
    ///
    /// This can be used by implementations that shell out to docker/podman CLI.
    pub fn build_create_args(&self) -> Vec<String> {
        let mut args = vec!["create".to_string()];

        // Working directory
        args.extend(["-w".to_string(), self.config.work_dir.display().to_string()]);

        // Volumes
        for (host, container) in &self.config.volumes {
            args.extend(["-v".to_string(), format!("{}:{}", host, container)]);
        }

        // Environment variables
        for (key, value) in &self.config.env {
            args.extend(["-e".to_string(), format!("{}={}", key, value)]);
        }

        // Network mode
        if let Some(ref network) = self.config.network_mode {
            args.extend(["--network".to_string(), network.clone()]);
        }

        // Memory limit
        if let Some(ref memory) = self.config.memory_limit {
            args.extend(["--memory".to_string(), memory.clone()]);
        }

        // CPU limit
        if let Some(ref cpu) = self.config.cpu_limit {
            args.extend(["--cpus".to_string(), cpu.clone()]);
        }

        // Privileged mode
        if self.config.privileged {
            args.push("--privileged".to_string());
        }

        // Image
        args.push(self.config.image.clone());

        args
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sandbox_config_builder() {
        let config = SandboxConfig::new("custom:image")
            .with_runtime(ContainerRuntime::Podman)
            .with_work_dir(PathBuf::from("/app"))
            .with_volume("/host/path", "/container/path")
            .with_env("FOO", "bar")
            .with_memory_limit("8g");

        assert_eq!(config.image, "custom:image");
        assert_eq!(config.runtime, ContainerRuntime::Podman);
        assert_eq!(config.work_dir, PathBuf::from("/app"));
        assert_eq!(
            config.volumes.get("/host/path"),
            Some(&"/container/path".to_string())
        );
        assert_eq!(config.env.get("FOO"), Some(&"bar".to_string()));
        assert_eq!(config.memory_limit, Some("8g".to_string()));
    }

    #[test]
    fn test_build_create_args() {
        let config = SandboxConfig::new("node:20")
            .with_work_dir(PathBuf::from("/workspace"))
            .with_env("TEST", "value");

        let sandbox = DockerSandbox::new(config);
        let args = sandbox.build_create_args();

        assert!(args.contains(&"create".to_string()));
        assert!(args.contains(&"node:20".to_string()));
        assert!(args.contains(&"-w".to_string()));
    }
}
