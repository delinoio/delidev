use serde::{Deserialize, Serialize};

use super::AIAgentType;

/// External editor type for opening files
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum EditorType {
    /// Visual Studio Code
    #[default]
    Vscode,
    /// Cursor (AI-powered VSCode fork)
    Cursor,
    /// Windsurf (Codeium editor)
    Windsurf,
    /// VSCode Insiders
    VscodeInsiders,
    /// VSCodium
    Vscodium,
}

impl EditorType {
    /// Returns the display name for this editor
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Vscode => "Visual Studio Code",
            Self::Cursor => "Cursor",
            Self::Windsurf => "Windsurf",
            Self::VscodeInsiders => "VSCode Insiders",
            Self::Vscodium => "VSCodium",
        }
    }

    /// Returns the command to open this editor
    #[cfg(target_os = "macos")]
    pub fn command(&self) -> &'static str {
        match self {
            Self::Vscode => "code",
            Self::Cursor => "cursor",
            Self::Windsurf => "windsurf",
            Self::VscodeInsiders => "code-insiders",
            Self::Vscodium => "codium",
        }
    }

    #[cfg(target_os = "linux")]
    pub fn command(&self) -> &'static str {
        match self {
            Self::Vscode => "code",
            Self::Cursor => "cursor",
            Self::Windsurf => "windsurf",
            Self::VscodeInsiders => "code-insiders",
            Self::Vscodium => "codium",
        }
    }

    #[cfg(target_os = "windows")]
    pub fn command(&self) -> &'static str {
        match self {
            Self::Vscode => "code.cmd",
            Self::Cursor => "cursor.cmd",
            Self::Windsurf => "windsurf.cmd",
            Self::VscodeInsiders => "code-insiders.cmd",
            Self::Vscodium => "codium.cmd",
        }
    }
}

/// Editor configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EditorConfig {
    /// The preferred editor type
    #[serde(default)]
    pub editor_type: EditorType,
}

/// Container runtime type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum ContainerRuntime {
    /// Docker
    #[default]
    Docker,
    /// Podman
    Podman,
}

impl ContainerRuntime {
    /// Returns the display name
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Docker => "Docker",
            Self::Podman => "Podman",
        }
    }

    /// Returns the default socket path for this runtime
    #[cfg(target_os = "linux")]
    pub fn default_socket_path(&self) -> String {
        match self {
            Self::Docker => "unix:///var/run/docker.sock".to_string(),
            Self::Podman => {
                // Podman uses user-specific socket by default
                if let Ok(xdg_runtime) = std::env::var("XDG_RUNTIME_DIR") {
                    format!("unix://{}/podman/podman.sock", xdg_runtime)
                } else if let Ok(uid) = std::env::var("UID") {
                    format!("unix:///run/user/{}/podman/podman.sock", uid)
                } else {
                    // Fallback to common location
                    "unix:///run/podman/podman.sock".to_string()
                }
            }
        }
    }

    #[cfg(target_os = "macos")]
    pub fn default_socket_path(&self) -> String {
        match self {
            Self::Docker => "unix:///var/run/docker.sock".to_string(),
            Self::Podman => {
                // Podman on macOS uses a VM, socket is typically in ~/.local/share/containers
                if let Some(home) = dirs::home_dir() {
                    format!(
                        "unix://{}/.local/share/containers/podman/machine/podman.sock",
                        home.display()
                    )
                } else {
                    "unix:///var/run/podman/podman.sock".to_string()
                }
            }
        }
    }

    #[cfg(target_os = "windows")]
    pub fn default_socket_path(&self) -> String {
        match self {
            Self::Docker => "npipe:////./pipe/docker_engine".to_string(),
            Self::Podman => "npipe:////./pipe/podman-machine-default".to_string(),
        }
    }

    /// Mobile platforms don't support containers
    #[cfg(any(target_os = "ios", target_os = "android"))]
    pub fn default_socket_path(&self) -> String {
        // Containers are not available on mobile
        String::new()
    }
}

/// Agent configuration for a specific purpose
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    /// AI agent type
    #[serde(rename = "type", default)]
    pub agent_type: AIAgentType,
    /// AI model
    #[serde(default = "default_model")]
    pub model: String,
}

fn default_model() -> String {
    "claude-sonnet-4-20250514".to_string()
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            agent_type: AIAgentType::default(),
            model: default_model(),
        }
    }
}

/// Learning settings
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LearningConfig {
    /// Automatically learn from VCS provider reviews
    #[serde(default)]
    pub auto_learn_from_reviews: bool,
}

/// Composite task settings
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CompositeTaskConfig {
    /// Automatically approve composite task plans without user review
    #[serde(default)]
    pub auto_approve: bool,
}

/// Hotkey settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotkeyConfig {
    /// Global hotkey to open chat window
    #[serde(default = "default_open_chat_hotkey")]
    pub open_chat: String,
}

fn default_open_chat_hotkey() -> String {
    "Option+Z".to_string()
}

impl Default for HotkeyConfig {
    fn default() -> Self {
        Self {
            open_chat: default_open_chat_hotkey(),
        }
    }
}

/// Global agent settings
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GlobalAgentConfig {
    /// Agent for CompositeTask planning
    #[serde(default)]
    pub planning: AgentConfig,
    /// Agent for UnitTask execution and auto-fix
    #[serde(default)]
    pub execution: AgentConfig,
    /// Agent for chat interface
    #[serde(default)]
    pub chat: AgentConfig,
}

/// Container settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerConfig {
    /// Container runtime to use
    #[serde(default)]
    pub runtime: ContainerRuntime,
    /// Custom socket path (optional, uses default if not set)
    #[serde(default)]
    pub socket_path: Option<String>,
    /// Whether to use container (Docker/Podman) for agent execution
    /// When false, agents run directly on the host without containerization
    #[serde(default = "default_use_container")]
    pub use_container: bool,
}

fn default_use_container() -> bool {
    true
}

impl Default for ContainerConfig {
    fn default() -> Self {
        Self {
            runtime: ContainerRuntime::default(),
            socket_path: None,
            use_container: true,
        }
    }
}

impl ContainerConfig {
    /// Returns the effective socket path
    pub fn effective_socket_path(&self) -> String {
        self.socket_path
            .clone()
            .unwrap_or_else(|| self.runtime.default_socket_path())
    }
}

/// Concurrency settings
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ConcurrencyConfig {
    /// Maximum number of concurrent agent sessions. None means unlimited.
    /// This is a premium feature that requires a valid license.
    #[serde(default)]
    pub max_concurrent_sessions: Option<u32>,
}

/// Global settings (~/.delidev/config.toml)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GlobalConfig {
    #[serde(default)]
    pub learning: LearningConfig,
    #[serde(default)]
    pub hotkey: HotkeyConfig,
    #[serde(default)]
    pub agent: GlobalAgentConfig,
    #[serde(default)]
    pub container: ContainerConfig,
    #[serde(default)]
    pub editor: EditorConfig,
    #[serde(default)]
    pub composite_task: CompositeTaskConfig,
    #[serde(default)]
    pub concurrency: ConcurrencyConfig,
}

/// Filter for auto-fix review comments
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum AutoFixReviewFilter {
    /// Only apply comments from users with write permission
    #[default]
    WriteAccessOnly,
    /// Apply all comments including bots
    All,
}

/// Docker settings for repository
///
/// The Docker image is built from `.delidev/setup/Dockerfile` if it exists,
/// otherwise a default image (node:20-slim) is used.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DockerConfig {
    // All configuration is now done via .delidev/setup/Dockerfile
}

/// Branch naming settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchConfig {
    /// Branch name template (variables: ${taskId}, ${slug})
    #[serde(default = "default_branch_template")]
    pub template: String,
}

fn default_branch_template() -> String {
    "delidev/${taskId}".to_string()
}

impl Default for BranchConfig {
    fn default() -> Self {
        Self {
            template: default_branch_template(),
        }
    }
}

/// Automation settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AutomationConfig {
    /// Automatically apply review comments
    #[serde(default = "default_true")]
    pub auto_fix_review_comments: bool,
    /// Filter for review comments
    #[serde(default)]
    pub auto_fix_review_comments_filter: AutoFixReviewFilter,
    /// Automatically fix CI failures
    #[serde(default = "default_true")]
    pub auto_fix_ci_failures: bool,
    /// Maximum number of auto-fix attempts
    #[serde(default = "default_max_auto_fix_attempts")]
    pub max_auto_fix_attempts: u32,
}

fn default_true() -> bool {
    true
}

fn default_max_auto_fix_attempts() -> u32 {
    3
}

impl Default for AutomationConfig {
    fn default() -> Self {
        Self {
            auto_fix_review_comments: true,
            auto_fix_review_comments_filter: AutoFixReviewFilter::default(),
            auto_fix_ci_failures: true,
            max_auto_fix_attempts: 3,
        }
    }
}

/// Repository-specific learning settings (can override global)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RepositoryLearningConfig {
    /// Override global setting for this repository
    pub auto_learn_from_reviews: Option<bool>,
}

/// Repository-specific composite task settings (can override global)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RepositoryCompositeTaskConfig {
    /// Override global auto-approve setting for this repository
    pub auto_approve: Option<bool>,
}

/// Repository settings (.delidev/config.toml)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RepositoryConfig {
    #[serde(default)]
    pub docker: DockerConfig,
    #[serde(default)]
    pub branch: BranchConfig,
    #[serde(default)]
    pub automation: AutomationConfig,
    #[serde(default)]
    pub learning: RepositoryLearningConfig,
    #[serde(default)]
    pub composite_task: RepositoryCompositeTaskConfig,
}

impl RepositoryConfig {
    /// Returns effective auto_learn_from_reviews, considering global config
    pub fn effective_auto_learn(&self, global: &GlobalConfig) -> bool {
        self.learning
            .auto_learn_from_reviews
            .unwrap_or(global.learning.auto_learn_from_reviews)
    }

    /// Returns effective auto_approve for composite tasks, considering global
    /// config
    pub fn effective_composite_task_auto_approve(&self, global: &GlobalConfig) -> bool {
        self.composite_task
            .auto_approve
            .unwrap_or(global.composite_task.auto_approve)
    }

    /// Generates branch name from template
    pub fn generate_branch_name(&self, task_id: &str, slug: &str) -> String {
        self.branch
            .template
            .replace("${taskId}", task_id)
            .replace("${slug}", slug)
    }
}

/// VCS provider credentials
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VCSCredentials {
    #[serde(default)]
    pub github: Option<GitHubCredentials>,
    #[serde(default)]
    pub gitlab: Option<GitLabCredentials>,
    #[serde(default)]
    pub bitbucket: Option<BitbucketCredentials>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubCredentials {
    pub token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitLabCredentials {
    pub token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BitbucketCredentials {
    pub username: String,
    pub app_password: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    mod editor_type {
        use super::*;

        #[test]
        fn test_display_name() {
            assert_eq!(EditorType::Vscode.display_name(), "Visual Studio Code");
            assert_eq!(EditorType::Cursor.display_name(), "Cursor");
            assert_eq!(EditorType::Windsurf.display_name(), "Windsurf");
            assert_eq!(EditorType::VscodeInsiders.display_name(), "VSCode Insiders");
            assert_eq!(EditorType::Vscodium.display_name(), "VSCodium");
        }

        #[test]
        fn test_command_returns_non_empty_string() {
            assert!(!EditorType::Vscode.command().is_empty());
            assert!(!EditorType::Cursor.command().is_empty());
            assert!(!EditorType::Windsurf.command().is_empty());
            assert!(!EditorType::VscodeInsiders.command().is_empty());
            assert!(!EditorType::Vscodium.command().is_empty());
        }

        #[test]
        fn test_default_is_vscode() {
            assert_eq!(EditorType::default(), EditorType::Vscode);
        }

        #[test]
        fn test_serialization() {
            let editor = EditorType::Cursor;
            let json = serde_json::to_string(&editor).unwrap();
            assert_eq!(json, "\"cursor\"");

            let editor = EditorType::VscodeInsiders;
            let json = serde_json::to_string(&editor).unwrap();
            assert_eq!(json, "\"vscode_insiders\"");
        }

        #[test]
        fn test_deserialization() {
            let editor: EditorType = serde_json::from_str("\"windsurf\"").unwrap();
            assert_eq!(editor, EditorType::Windsurf);
        }
    }

    mod container_runtime {
        use super::*;

        #[test]
        fn test_display_name() {
            assert_eq!(ContainerRuntime::Docker.display_name(), "Docker");
            assert_eq!(ContainerRuntime::Podman.display_name(), "Podman");
        }

        #[test]
        fn test_default_socket_path_returns_platform_specific_path() {
            let docker_path = ContainerRuntime::Docker.default_socket_path();
            let podman_path = ContainerRuntime::Podman.default_socket_path();

            // Verify paths are non-empty and different between runtimes
            assert!(!docker_path.is_empty());
            assert!(!podman_path.is_empty());
            assert_ne!(docker_path, podman_path);
        }
    }

    mod agent_config {
        use super::*;

        #[test]
        fn test_serialization_renames_agent_type_to_type() {
            let config = AgentConfig::default();
            let json = serde_json::to_string(&config).unwrap();
            assert!(json.contains("\"type\""));
            assert!(!json.contains("\"agent_type\""));
        }
    }

    mod container_config {
        use super::*;

        #[test]
        fn test_effective_socket_path_uses_custom_when_set() {
            let config = ContainerConfig {
                runtime: ContainerRuntime::Docker,
                socket_path: Some("/custom/path.sock".to_string()),
                use_container: true,
            };
            assert_eq!(config.effective_socket_path(), "/custom/path.sock");
        }

        #[test]
        fn test_effective_socket_path_falls_back_to_runtime_default() {
            let config = ContainerConfig::default();
            let path = config.effective_socket_path();
            // Should return the runtime's default socket path
            assert_eq!(path, config.runtime.default_socket_path());
        }
    }

    mod repository_config {
        use super::*;

        #[test]
        fn test_effective_auto_learn_prefers_repository_override() {
            let global = GlobalConfig {
                learning: LearningConfig {
                    auto_learn_from_reviews: true,
                },
                ..Default::default()
            };

            let repo = RepositoryConfig {
                learning: RepositoryLearningConfig {
                    auto_learn_from_reviews: Some(false),
                },
                ..Default::default()
            };

            assert!(!repo.effective_auto_learn(&global));
        }

        #[test]
        fn test_effective_auto_learn_falls_back_to_global_when_not_set() {
            let global = GlobalConfig {
                learning: LearningConfig {
                    auto_learn_from_reviews: true,
                },
                ..Default::default()
            };

            let repo = RepositoryConfig {
                learning: RepositoryLearningConfig {
                    auto_learn_from_reviews: None,
                },
                ..Default::default()
            };

            assert!(repo.effective_auto_learn(&global));
        }

        #[test]
        fn test_generate_branch_name_replaces_template_variables() {
            let config = RepositoryConfig {
                branch: BranchConfig {
                    template: "feature/${slug}-${taskId}".to_string(),
                },
                ..Default::default()
            };
            let branch = config.generate_branch_name("456", "add-feature");
            assert_eq!(branch, "feature/add-feature-456");
        }

        #[test]
        fn test_effective_composite_task_auto_approve_prefers_repository_override() {
            let global = GlobalConfig {
                composite_task: CompositeTaskConfig {
                    auto_approve: false,
                },
                ..Default::default()
            };

            let repo = RepositoryConfig {
                composite_task: RepositoryCompositeTaskConfig {
                    auto_approve: Some(true),
                },
                ..Default::default()
            };

            assert!(repo.effective_composite_task_auto_approve(&global));
        }

        #[test]
        fn test_effective_composite_task_auto_approve_falls_back_to_global_when_not_set() {
            let global = GlobalConfig {
                composite_task: CompositeTaskConfig { auto_approve: true },
                ..Default::default()
            };

            let repo = RepositoryConfig {
                composite_task: RepositoryCompositeTaskConfig { auto_approve: None },
                ..Default::default()
            };

            assert!(repo.effective_composite_task_auto_approve(&global));
        }

        #[test]
        fn test_effective_composite_task_auto_approve_defaults_to_false() {
            let global = GlobalConfig::default();
            let repo = RepositoryConfig::default();

            assert!(!repo.effective_composite_task_auto_approve(&global));
        }
    }
}
