use std::{path::Path, time::Duration};

use bollard::{
    container::{Config, CreateContainerOptions, StartContainerOptions, StopContainerOptions},
    image::{BuildImageOptions, CreateImageOptions},
    Docker,
};
use futures_util::StreamExt;
use thiserror::Error;

use crate::entities::ContainerRuntime;

#[derive(Error, Debug)]
pub enum DockerError {
    #[error("{0} connection error: {1}")]
    Connection(String, String),
    #[error("Container not found: {0}")]
    ContainerNotFound(String),
    #[error("Image pull failed: {0}")]
    ImagePullFailed(String),
    #[error("Image build failed: {0}")]
    ImageBuildFailed(String),
    #[error("Execution timeout after {0} seconds")]
    Timeout(u64),
    #[error("Execution failed: {0}")]
    ExecutionFailed(String),
}

impl From<bollard::errors::Error> for DockerError {
    fn from(e: bollard::errors::Error) -> Self {
        DockerError::Connection("Container runtime".to_string(), e.to_string())
    }
}

/// Result of command execution in a container
#[derive(Debug)]
pub struct ExecResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i64>,
}

pub type DockerResult<T> = Result<T, DockerError>;

/// Service for managing Docker/Podman containers
pub struct DockerService {
    docker: Docker,
    runtime: ContainerRuntime,
}

impl DockerService {
    /// Creates a new container service with default settings
    pub fn new() -> DockerResult<Self> {
        Self::with_runtime(ContainerRuntime::Docker, None)
    }

    /// Creates a new container service with specified runtime and optional
    /// socket path
    pub fn with_runtime(
        runtime: ContainerRuntime,
        socket_path: Option<String>,
    ) -> DockerResult<Self> {
        let docker = if let Some(path) = socket_path {
            // Use custom socket path
            if let Some(stripped) = path.strip_prefix("unix://") {
                Docker::connect_with_socket(stripped, 120, bollard::API_DEFAULT_VERSION)
            } else if path.starts_with("npipe://") {
                #[cfg(windows)]
                {
                    Docker::connect_with_named_pipe(&path[8..], 120, bollard::API_DEFAULT_VERSION)
                }
                #[cfg(not(windows))]
                {
                    Err(bollard::errors::Error::UnsupportedURISchemeError { uri: path.clone() })
                }
            } else {
                Docker::connect_with_socket(&path, 120, bollard::API_DEFAULT_VERSION)
            }
        } else {
            // Use runtime's default socket path
            let default_path = runtime.default_socket_path();
            if let Some(stripped) = default_path.strip_prefix("unix://") {
                Docker::connect_with_socket(stripped, 120, bollard::API_DEFAULT_VERSION)
            } else {
                Docker::connect_with_local_defaults()
            }
        }
        .map_err(|e| DockerError::Connection(runtime.display_name().to_string(), e.to_string()))?;

        Ok(Self { docker, runtime })
    }

    /// Returns the container runtime type
    pub fn runtime(&self) -> ContainerRuntime {
        self.runtime
    }

    /// Returns the runtime display name
    pub fn runtime_name(&self) -> &'static str {
        self.runtime.display_name()
    }

    /// Checks if the container runtime is available
    pub async fn is_available(&self) -> bool {
        self.docker.ping().await.is_ok()
    }

    /// Gets version info
    pub async fn version(&self) -> DockerResult<String> {
        let version = self.docker.version().await?;
        Ok(version.version.unwrap_or_else(|| "unknown".to_string()))
    }

    /// Gets version info with runtime name
    pub async fn version_info(&self) -> DockerResult<String> {
        let version = self.version().await?;
        Ok(format!("{} {}", self.runtime_name(), version))
    }

    /// Pulls a Docker image
    pub async fn pull_image(&self, image: &str) -> DockerResult<()> {
        let options = Some(CreateImageOptions {
            from_image: image,
            ..Default::default()
        });

        let mut stream = self.docker.create_image(options, None, None);

        while let Some(result) = stream.next().await {
            match result {
                Ok(_) => continue,
                Err(e) => return Err(DockerError::ImagePullFailed(e.to_string())),
            }
        }

        Ok(())
    }

    /// Creates a container for agent execution
    pub async fn create_agent_container(
        &self,
        name: &str,
        image: &str,
        working_dir: &str,
        host_path: &str,
    ) -> DockerResult<String> {
        // Validate and canonicalize host path
        let host_path_buf = std::path::Path::new(host_path);
        if !host_path_buf.exists() {
            return Err(DockerError::ExecutionFailed(format!(
                "Host path does not exist: {}",
                host_path
            )));
        }
        let canonical_host_path = host_path_buf.canonicalize().map_err(|e| {
            DockerError::ExecutionFailed(format!(
                "Failed to resolve host path '{}': {}",
                host_path, e
            ))
        })?;

        // Ensure image exists
        if self.docker.inspect_image(image).await.is_err() {
            self.pull_image(image).await?;
        }

        let config = Config {
            image: Some(image.to_string()),
            working_dir: Some(working_dir.to_string()),
            host_config: Some(bollard::service::HostConfig {
                binds: Some(vec![format!(
                    "{}:{}",
                    canonical_host_path.to_string_lossy(),
                    working_dir
                )]),
                auto_remove: Some(true),
                ..Default::default()
            }),
            tty: Some(true),
            ..Default::default()
        };

        let options = CreateContainerOptions {
            name,
            platform: None,
        };

        let response = self.docker.create_container(Some(options), config).await?;
        Ok(response.id)
    }

    /// Starts a container
    pub async fn start_container(&self, id: &str) -> DockerResult<()> {
        self.docker
            .start_container(id, None::<StartContainerOptions<String>>)
            .await?;
        Ok(())
    }

    /// Stops a container
    pub async fn stop_container(&self, id: &str) -> DockerResult<()> {
        let options = StopContainerOptions { t: 10 };
        self.docker.stop_container(id, Some(options)).await?;
        Ok(())
    }

    /// Removes a container
    pub async fn remove_container(&self, id: &str) -> DockerResult<()> {
        self.docker.remove_container(id, None).await?;
        Ok(())
    }

    /// Executes a command in a running container
    pub async fn exec_in_container(&self, id: &str, cmd: Vec<&str>) -> DockerResult<String> {
        use bollard::exec::{CreateExecOptions, StartExecResults};

        let exec = self
            .docker
            .create_exec(
                id,
                CreateExecOptions {
                    cmd: Some(cmd),
                    attach_stdout: Some(true),
                    attach_stderr: Some(true),
                    ..Default::default()
                },
            )
            .await?;

        let mut output = String::new();

        if let StartExecResults::Attached {
            output: mut stream, ..
        } = self.docker.start_exec(&exec.id, None).await?
        {
            while let Some(Ok(msg)) = stream.next().await {
                use bollard::container::LogOutput;
                let text = match msg {
                    LogOutput::StdOut { message } => String::from_utf8_lossy(&message).to_string(),
                    LogOutput::StdErr { message } => String::from_utf8_lossy(&message).to_string(),
                    _ => String::new(),
                };
                output.push_str(&text);
            }
        }

        Ok(output)
    }

    /// Lists all running containers (returns container IDs)
    pub async fn list_containers(&self) -> DockerResult<Vec<String>> {
        use bollard::container::ListContainersOptions;

        let options = ListContainersOptions::<String> {
            all: false,
            ..Default::default()
        };

        let containers = self.docker.list_containers(Some(options)).await?;

        Ok(containers.into_iter().filter_map(|c| c.id).collect())
    }

    /// Lists all running container names (returns container names without
    /// leading slash)
    pub async fn list_running_container_names(&self) -> DockerResult<Vec<String>> {
        use bollard::container::ListContainersOptions;

        let options = ListContainersOptions::<String> {
            all: false, // Only running containers
            ..Default::default()
        };

        let containers = self.docker.list_containers(Some(options)).await?;

        // Extract container names, removing the leading slash that Docker adds
        Ok(containers
            .into_iter()
            .filter_map(|c| c.names)
            .flatten()
            .map(|name| name.trim_start_matches('/').to_string())
            .collect())
    }

    /// Checks if a container with the given name is currently running
    pub async fn is_container_running(&self, name: &str) -> bool {
        use std::collections::HashMap;

        use bollard::container::ListContainersOptions;

        // Filter by container name
        let mut filters = HashMap::new();
        filters.insert("name".to_string(), vec![name.to_string()]);

        let options = ListContainersOptions {
            all: false, // Only running containers
            filters,
            ..Default::default()
        };

        match self.docker.list_containers(Some(options)).await {
            Ok(containers) => !containers.is_empty(),
            Err(_) => false,
        }
    }

    /// Creates a container for agent execution with environment variables
    pub async fn create_agent_container_with_env(
        &self,
        name: &str,
        image: &str,
        working_dir: &str,
        host_path: &str,
        env_vars: Vec<String>,
        claude_config_path: Option<&str>,
    ) -> DockerResult<String> {
        // Validate and canonicalize host path
        let host_path_buf = std::path::Path::new(host_path);
        if !host_path_buf.exists() {
            return Err(DockerError::ExecutionFailed(format!(
                "Host path does not exist: {}",
                host_path
            )));
        }
        let canonical_host_path = host_path_buf.canonicalize().map_err(|e| {
            DockerError::ExecutionFailed(format!(
                "Failed to resolve host path '{}': {}",
                host_path, e
            ))
        })?;

        // Validate and canonicalize claude config path if provided
        let canonical_config_path = if let Some(config_path) = claude_config_path {
            let config_path_buf = std::path::Path::new(config_path);
            if !config_path_buf.exists() {
                return Err(DockerError::ExecutionFailed(format!(
                    "Claude config path does not exist: {}",
                    config_path
                )));
            }
            Some(config_path_buf.canonicalize().map_err(|e| {
                DockerError::ExecutionFailed(format!(
                    "Failed to resolve claude config path '{}': {}",
                    config_path, e
                ))
            })?)
        } else {
            None
        };

        // Ensure image exists
        if self.docker.inspect_image(image).await.is_err() {
            self.pull_image(image).await?;
        }

        // Build volume binds
        // Use :Z for SELinux relabeling (required for Podman rootless mode)
        let mut binds = vec![format!(
            "{}:{}:Z",
            canonical_host_path.to_string_lossy(),
            working_dir
        )];

        // Mount Claude config directory to a separate path (will be copied to HOME
        // later)
        if let Some(config_path) = canonical_config_path {
            binds.push(format!(
                "{}:/tmp/claude-config:ro",
                config_path.to_string_lossy()
            ));
        }

        let config = Config {
            image: Some(image.to_string()),
            working_dir: Some(working_dir.to_string()),
            env: Some(env_vars),
            host_config: Some(bollard::service::HostConfig {
                binds: Some(binds),
                auto_remove: Some(false), // We'll manually remove after getting logs
                // Use keep-id to map host user UID/GID into container (Podman rootless)
                userns_mode: Some("keep-id".to_string()),
                tmpfs: Some(std::collections::HashMap::from([
                    ("/tmp/claude".to_string(), "".to_string()),
                    // Make /workspace writable for HOME directory (Claude Code writes .claude.json
                    // here)
                    ("/workspace".to_string(), "".to_string()),
                ])),
                ..Default::default()
            }),
            tty: Some(true),
            // Keep container running with a long sleep command
            cmd: Some(vec!["sleep".to_string(), "infinity".to_string()]),
            ..Default::default()
        };

        let options = CreateContainerOptions {
            name,
            platform: None,
        };

        let response = self.docker.create_container(Some(options), config).await?;
        Ok(response.id)
    }

    /// Executes a command in a running container with timeout and output
    /// callback
    pub async fn exec_with_callback<F>(
        &self,
        id: &str,
        cmd: Vec<String>,
        timeout_secs: u64,
        mut on_output: F,
    ) -> DockerResult<ExecResult>
    where
        F: FnMut(&str) + Send,
    {
        use bollard::exec::{CreateExecOptions, StartExecResults};

        let exec = self
            .docker
            .create_exec(
                id,
                CreateExecOptions {
                    cmd: Some(cmd.iter().map(|s| s.as_str()).collect()),
                    attach_stdout: Some(true),
                    attach_stderr: Some(true),
                    ..Default::default()
                },
            )
            .await?;

        let mut stdout = String::new();
        let mut stderr = String::new();

        let exec_result = tokio::time::timeout(Duration::from_secs(timeout_secs), async {
            if let StartExecResults::Attached {
                output: mut stream, ..
            } = self.docker.start_exec(&exec.id, None).await?
            {
                while let Some(Ok(msg)) = stream.next().await {
                    use bollard::container::LogOutput;
                    match msg {
                        LogOutput::StdOut { message } => {
                            let text = String::from_utf8_lossy(&message).to_string();
                            on_output(&text);
                            stdout.push_str(&text);
                        }
                        LogOutput::StdErr { message } => {
                            let text = String::from_utf8_lossy(&message).to_string();
                            on_output(&text);
                            stderr.push_str(&text);
                        }
                        _ => {}
                    }
                }
            }
            Ok::<_, DockerError>(())
        })
        .await;

        match exec_result {
            Ok(Ok(())) => {
                // Get exit code
                let inspect = self.docker.inspect_exec(&exec.id).await?;
                Ok(ExecResult {
                    stdout,
                    stderr,
                    exit_code: inspect.exit_code,
                })
            }
            Ok(Err(e)) => Err(e),
            Err(_) => Err(DockerError::Timeout(timeout_secs)),
        }
    }

    /// Executes a command and returns the full result
    pub async fn exec_with_result(
        &self,
        id: &str,
        cmd: Vec<String>,
        timeout_secs: u64,
    ) -> DockerResult<ExecResult> {
        self.exec_with_callback(id, cmd, timeout_secs, |_| {}).await
    }

    /// Waits for a container to exit
    pub async fn wait_container(&self, id: &str) -> DockerResult<i64> {
        use bollard::container::WaitContainerOptions;

        let options = WaitContainerOptions {
            condition: "not-running",
        };

        let mut stream = self.docker.wait_container(id, Some(options));

        if let Some(result) = stream.next().await {
            match result {
                Ok(response) => Ok(response.status_code),
                Err(e) => Err(e.into()),
            }
        } else {
            Ok(0)
        }
    }

    /// Gets container logs
    pub async fn get_container_logs(&self, id: &str) -> DockerResult<String> {
        use bollard::container::LogsOptions;

        let options = LogsOptions::<String> {
            stdout: true,
            stderr: true,
            ..Default::default()
        };

        let mut logs = String::new();
        let mut stream = self.docker.logs(id, Some(options));

        while let Some(Ok(msg)) = stream.next().await {
            use bollard::container::LogOutput;
            match msg {
                LogOutput::StdOut { message } | LogOutput::StdErr { message } => {
                    logs.push_str(&String::from_utf8_lossy(&message));
                }
                _ => {}
            }
        }

        Ok(logs)
    }

    /// Builds an image from a Dockerfile in the specified directory
    ///
    /// # Arguments
    /// * `dockerfile_dir` - Directory containing the Dockerfile
    /// * `image_tag` - Tag for the built image
    /// * `on_output` - Optional callback to receive build output logs
    ///
    /// # Returns
    /// The image ID on success
    pub async fn build_image_from_dockerfile<F>(
        &self,
        dockerfile_dir: &Path,
        image_tag: &str,
        mut on_output: F,
    ) -> DockerResult<String>
    where
        F: FnMut(&str) + Send,
    {
        use tar::Builder;

        // Create a tar archive of the Dockerfile directory
        let mut tar_buffer = Vec::new();
        {
            let mut tar_builder = Builder::new(&mut tar_buffer);

            // Add all files from the directory
            for entry in std::fs::read_dir(dockerfile_dir).map_err(|e| {
                DockerError::ImageBuildFailed(format!("Failed to read directory: {}", e))
            })? {
                let entry = entry.map_err(|e| {
                    DockerError::ImageBuildFailed(format!("Failed to read directory entry: {}", e))
                })?;
                let path = entry.path();
                let file_name = path
                    .file_name()
                    .ok_or_else(|| DockerError::ImageBuildFailed("Invalid file name".to_string()))?
                    .to_string_lossy();

                if path.is_file() {
                    let content = std::fs::read(&path).map_err(|e| {
                        DockerError::ImageBuildFailed(format!("Failed to read file: {}", e))
                    })?;

                    let mut header = tar::Header::new_gnu();
                    header.set_path(&*file_name).map_err(|e| {
                        DockerError::ImageBuildFailed(format!("Failed to set path in tar: {}", e))
                    })?;
                    header.set_size(content.len() as u64);
                    header.set_mode(0o644);
                    header.set_cksum();

                    tar_builder
                        .append(&header, content.as_slice())
                        .map_err(|e| {
                            DockerError::ImageBuildFailed(format!("Failed to append to tar: {}", e))
                        })?;
                }
            }

            tar_builder.finish().map_err(|e| {
                DockerError::ImageBuildFailed(format!("Failed to finish tar: {}", e))
            })?;
        }

        let options = BuildImageOptions {
            dockerfile: "Dockerfile",
            t: image_tag,
            rm: true,
            ..Default::default()
        };

        // Pass empty HashMap for credentials to avoid X-Registry-Config header parsing
        // issues
        let credentials: std::collections::HashMap<String, bollard::auth::DockerCredentials> =
            std::collections::HashMap::new();
        let mut stream =
            self.docker
                .build_image(options, Some(credentials), Some(tar_buffer.into()));

        let mut last_error = None;
        while let Some(result) = stream.next().await {
            match result {
                Ok(info) => {
                    // Stream build output to callback
                    if let Some(stream_msg) = info.stream {
                        on_output(&stream_msg);
                    }
                    // Also handle status messages
                    if let Some(status) = info.status {
                        on_output(&status);
                    }
                    // Check for error in build output
                    if let Some(error) = info.error {
                        last_error = Some(error);
                    }
                }
                Err(e) => {
                    return Err(DockerError::ImageBuildFailed(e.to_string()));
                }
            }
        }

        if let Some(error) = last_error {
            return Err(DockerError::ImageBuildFailed(error));
        }

        // Return the image tag as the image ID
        Ok(image_tag.to_string())
    }

    /// Computes a hash of the Dockerfile directory contents for cache
    /// invalidation
    fn compute_dockerfile_hash(dockerfile_dir: &Path) -> DockerResult<String> {
        use std::collections::BTreeMap;

        let mut files: BTreeMap<String, Vec<u8>> = BTreeMap::new();

        for entry in std::fs::read_dir(dockerfile_dir).map_err(|e| {
            DockerError::ImageBuildFailed(format!("Failed to read directory: {}", e))
        })? {
            let entry = entry.map_err(|e| {
                DockerError::ImageBuildFailed(format!("Failed to read directory entry: {}", e))
            })?;
            let path = entry.path();

            if path.is_file() {
                let file_name = path
                    .file_name()
                    .ok_or_else(|| DockerError::ImageBuildFailed("Invalid file name".to_string()))?
                    .to_string_lossy()
                    .to_string();

                let content = std::fs::read(&path).map_err(|e| {
                    DockerError::ImageBuildFailed(format!("Failed to read file: {}", e))
                })?;

                files.insert(file_name, content);
            }
        }

        // Simple hash: concatenate all file names and contents, then hash
        let mut hasher_input = Vec::new();
        for (name, content) in &files {
            hasher_input.extend(name.as_bytes());
            hasher_input.push(0);
            hasher_input.extend(content);
            hasher_input.push(0);
        }

        // Use a simple hash (sum of bytes with rotation)
        let mut hash: u64 = 0;
        for byte in &hasher_input {
            hash = hash.rotate_left(5).wrapping_add(*byte as u64);
        }

        Ok(format!("{:016x}", hash))
    }

    /// Gets the image to use for a repository
    ///
    /// If .delidev/setup/Dockerfile exists, builds a custom image (or reuses if
    /// unchanged). Otherwise, returns the default image (node:20-slim).
    ///
    /// # Arguments
    /// * `repo_path` - Path to the repository
    /// * `_task_id` - Task ID (unused, kept for API compatibility)
    /// * `on_output` - Callback to receive build output logs
    ///
    /// # Returns
    /// The image name/tag to use
    pub async fn get_or_build_image<F>(
        &self,
        repo_path: &Path,
        _task_id: &str,
        on_output: F,
    ) -> DockerResult<String>
    where
        F: FnMut(&str) + Send,
    {
        let dockerfile_path = repo_path.join(".delidev/setup/Dockerfile");

        if dockerfile_path.exists() {
            let dockerfile_dir = dockerfile_path.parent().ok_or_else(|| {
                DockerError::ImageBuildFailed("Invalid Dockerfile path".to_string())
            })?;

            // Compute hash of Dockerfile contents for cache key
            let content_hash = Self::compute_dockerfile_hash(dockerfile_dir)?;
            let image_tag = format!("delidev-setup:{}", content_hash);

            // Check if image already exists
            if self.docker.inspect_image(&image_tag).await.is_ok() {
                tracing::info!("Reusing existing image: {}", image_tag);
                return Ok(image_tag);
            }

            tracing::info!("Building custom image from {:?}", dockerfile_path);
            self.build_image_from_dockerfile(dockerfile_dir, &image_tag, on_output)
                .await?;

            Ok(image_tag)
        } else {
            // Default image
            let default_image = "node:20-slim";
            tracing::info!(
                "No custom Dockerfile found, using default image: {}",
                default_image
            );
            Ok(default_image.to_string())
        }
    }
}
