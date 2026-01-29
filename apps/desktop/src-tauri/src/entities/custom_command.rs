use serde::{Deserialize, Serialize};

use super::AIAgentType;

/// Source of a custom command
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandSource {
    /// Project-specific command (e.g., .claude/commands/, .opencode/command/)
    Project,
    /// Global user command (e.g., ~/.claude/commands/,
    /// ~/.config/opencode/command/)
    Global,
}

/// Agent framework that provides the custom command
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandFramework {
    /// Claude Code framework (.claude/commands/)
    ClaudeCode,
    /// OpenCode framework (.opencode/command/)
    OpenCode,
}

impl CommandFramework {
    /// Returns the default agent type for this framework
    pub fn default_agent_type(&self) -> AIAgentType {
        match self {
            Self::ClaudeCode => AIAgentType::ClaudeCode,
            Self::OpenCode => AIAgentType::OpenCode,
        }
    }

    /// Returns the display name for this framework
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::ClaudeCode => "Claude Code",
            Self::OpenCode => "OpenCode",
        }
    }
}

/// Frontmatter metadata for a custom command
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommandFrontmatter {
    /// Brief description of the command
    #[serde(default)]
    pub description: Option<String>,

    /// Agent type to use for this command (overrides framework default)
    #[serde(default)]
    pub agent: Option<String>,

    /// Model to use for this command
    #[serde(default)]
    pub model: Option<String>,

    /// Whether to run as a subtask/fork (OpenCode: subtask, Claude Code:
    /// context: fork)
    #[serde(default)]
    pub subtask: Option<bool>,

    /// Context mode for Claude Code (e.g., "fork")
    #[serde(default)]
    pub context: Option<String>,
}

/// A custom command that can be executed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomCommand {
    /// Unique identifier for the command (derived from filename without
    /// extension)
    pub name: String,

    /// Display name (may include namespace from subdirectories)
    pub display_name: String,

    /// Brief description of what the command does
    pub description: Option<String>,

    /// Agent type to use (derived from framework or frontmatter override)
    pub agent_type: AIAgentType,

    /// Model to use (optional, uses agent default if not specified)
    pub model: Option<String>,

    /// The prompt template content (markdown body after frontmatter)
    pub template: String,

    /// Source of the command (project or global)
    pub source: CommandSource,

    /// Framework that provides this command
    pub framework: CommandFramework,

    /// Relative path to the command file from the commands directory
    pub relative_path: String,

    /// Whether this command should run as a subtask
    pub is_subtask: bool,
}

impl CustomCommand {
    /// Creates a new CustomCommand
    pub fn new(
        name: String,
        template: String,
        source: CommandSource,
        framework: CommandFramework,
        relative_path: String,
    ) -> Self {
        Self {
            display_name: name.clone(),
            name,
            description: None,
            agent_type: framework.default_agent_type(),
            model: None,
            template,
            source,
            framework,
            relative_path,
            is_subtask: false,
        }
    }

    /// Sets the description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Sets the agent type
    pub fn with_agent_type(mut self, agent_type: AIAgentType) -> Self {
        self.agent_type = agent_type;
        self
    }

    /// Sets the model
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Sets the display name
    pub fn with_display_name(mut self, display_name: impl Into<String>) -> Self {
        self.display_name = display_name.into();
        self
    }

    /// Sets whether this is a subtask
    pub fn with_subtask(mut self, is_subtask: bool) -> Self {
        self.is_subtask = is_subtask;
        self
    }

    /// Applies frontmatter metadata to this command
    pub fn with_frontmatter(mut self, frontmatter: CommandFrontmatter) -> Self {
        if let Some(desc) = frontmatter.description {
            self.description = Some(desc);
        }

        // Parse agent type from frontmatter
        if let Some(agent_str) = frontmatter.agent {
            self.agent_type = match agent_str.to_lowercase().as_str() {
                "claude_code" | "claudecode" | "claude-code" => AIAgentType::ClaudeCode,
                "open_code" | "opencode" | "open-code" => AIAgentType::OpenCode,
                "gemini_cli" | "geminicli" | "gemini-cli" => AIAgentType::GeminiCli,
                "codex_cli" | "codexcli" | "codex-cli" => AIAgentType::CodexCli,
                "aider" => AIAgentType::Aider,
                "amp" => AIAgentType::Amp,
                _ => self.agent_type, // Keep default if unrecognized
            };
        }

        if let Some(model) = frontmatter.model {
            self.model = Some(model);
        }

        // Handle subtask flag (OpenCode style)
        if let Some(subtask) = frontmatter.subtask {
            self.is_subtask = subtask;
        }

        // Handle context: fork (Claude Code style)
        if let Some(context) = frontmatter.context {
            if context.to_lowercase() == "fork" {
                self.is_subtask = true;
            }
        }

        self
    }

    /// Renders the template with the given arguments
    ///
    /// Supports the following placeholders:
    /// - `$ARGUMENTS` - All arguments as a single string
    /// - `$1`, `$2`, etc. - Individual positional arguments
    pub fn render(&self, arguments: &str) -> String {
        let mut result = self.template.clone();

        // Replace $ARGUMENTS with all arguments
        result = result.replace("$ARGUMENTS", arguments);

        // Parse arguments into positional parts
        let parts: Vec<&str> = arguments.split_whitespace().collect();

        // Replace positional arguments $1, $2, etc.
        for (i, part) in parts.iter().enumerate() {
            let placeholder = format!("${}", i + 1);
            result = result.replace(&placeholder, part);
        }

        // Clear any remaining positional placeholders that weren't filled
        for i in parts.len()..10 {
            let placeholder = format!("${}", i + 1);
            result = result.replace(&placeholder, "");
        }

        result
    }

    /// Returns the effective model, falling back to agent type default
    pub fn effective_model(&self) -> String {
        self.model
            .clone()
            .unwrap_or_else(|| self.agent_type.default_model().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_framework_default_agent_type() {
        assert_eq!(
            CommandFramework::ClaudeCode.default_agent_type(),
            AIAgentType::ClaudeCode
        );
        assert_eq!(
            CommandFramework::OpenCode.default_agent_type(),
            AIAgentType::OpenCode
        );
    }

    #[test]
    fn test_custom_command_new() {
        let cmd = CustomCommand::new(
            "test".to_string(),
            "Do something".to_string(),
            CommandSource::Project,
            CommandFramework::ClaudeCode,
            "test.md".to_string(),
        );

        assert_eq!(cmd.name, "test");
        assert_eq!(cmd.display_name, "test");
        assert_eq!(cmd.agent_type, AIAgentType::ClaudeCode);
        assert_eq!(cmd.template, "Do something");
        assert_eq!(cmd.source, CommandSource::Project);
        assert!(!cmd.is_subtask);
    }

    #[test]
    fn test_render_with_arguments() {
        let cmd = CustomCommand::new(
            "test".to_string(),
            "Fix issue #$ARGUMENTS following our standards".to_string(),
            CommandSource::Project,
            CommandFramework::ClaudeCode,
            "test.md".to_string(),
        );

        let result = cmd.render("123 high-priority");
        assert_eq!(
            result,
            "Fix issue #123 high-priority following our standards"
        );
    }

    #[test]
    fn test_render_with_positional_arguments() {
        let cmd = CustomCommand::new(
            "test".to_string(),
            "Review PR #$1 with priority $2".to_string(),
            CommandSource::Project,
            CommandFramework::ClaudeCode,
            "test.md".to_string(),
        );

        let result = cmd.render("456 high");
        assert_eq!(result, "Review PR #456 with priority high");
    }

    #[test]
    fn test_render_clears_unfilled_placeholders() {
        let cmd = CustomCommand::new(
            "test".to_string(),
            "Task $1 and $2 and $3".to_string(),
            CommandSource::Project,
            CommandFramework::ClaudeCode,
            "test.md".to_string(),
        );

        let result = cmd.render("first");
        assert_eq!(result, "Task first and  and ");
    }

    #[test]
    fn test_with_frontmatter() {
        let cmd = CustomCommand::new(
            "test".to_string(),
            "Do something".to_string(),
            CommandSource::Project,
            CommandFramework::ClaudeCode,
            "test.md".to_string(),
        );

        let frontmatter = CommandFrontmatter {
            description: Some("Test command".to_string()),
            agent: Some("open_code".to_string()),
            model: Some("gpt-4o".to_string()),
            subtask: Some(true),
            context: None,
        };

        let cmd = cmd.with_frontmatter(frontmatter);

        assert_eq!(cmd.description, Some("Test command".to_string()));
        assert_eq!(cmd.agent_type, AIAgentType::OpenCode);
        assert_eq!(cmd.model, Some("gpt-4o".to_string()));
        assert!(cmd.is_subtask);
    }

    #[test]
    fn test_context_fork_sets_subtask() {
        let cmd = CustomCommand::new(
            "test".to_string(),
            "Do something".to_string(),
            CommandSource::Project,
            CommandFramework::ClaudeCode,
            "test.md".to_string(),
        );

        let frontmatter = CommandFrontmatter {
            context: Some("fork".to_string()),
            ..Default::default()
        };

        let cmd = cmd.with_frontmatter(frontmatter);
        assert!(cmd.is_subtask);
    }

    #[test]
    fn test_effective_model() {
        let cmd = CustomCommand::new(
            "test".to_string(),
            "Do something".to_string(),
            CommandSource::Project,
            CommandFramework::ClaudeCode,
            "test.md".to_string(),
        );

        // Should use default model from agent type
        assert_eq!(cmd.effective_model(), "claude-sonnet-4-20250514");

        // With custom model
        let cmd = cmd.with_model("claude-opus-4");
        assert_eq!(cmd.effective_model(), "claude-opus-4");
    }
}
