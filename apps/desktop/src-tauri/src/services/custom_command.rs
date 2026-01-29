use std::path::{Path, PathBuf};

use thiserror::Error;

use crate::entities::{
    AIAgentType, CommandFramework, CommandFrontmatter, CommandSource, CustomCommand,
};

#[derive(Error, Debug)]
pub enum CustomCommandError {
    #[error("Failed to read directory: {0}")]
    ReadDir(#[from] std::io::Error),
    #[error("Failed to parse frontmatter: {0}")]
    FrontmatterParse(String),
    #[error("Command not found: {0}")]
    NotFound(String),
}

pub type CustomCommandResult<T> = Result<T, CustomCommandError>;

/// Service for discovering and managing custom commands
pub struct CustomCommandService;

impl CustomCommandService {
    /// Creates a new CustomCommandService
    pub fn new() -> Self {
        Self
    }

    /// Discovers all custom commands for a repository
    ///
    /// This searches for commands in:
    /// - Project commands: `.claude/commands/`, `.opencode/command/`
    /// - Global commands: `~/.claude/commands/`, `~/.config/opencode/command/`
    ///
    /// Project commands take precedence over global commands with the same
    /// name.
    pub fn discover_commands(&self, repo_path: &Path) -> CustomCommandResult<Vec<CustomCommand>> {
        let mut commands: Vec<CustomCommand> = Vec::new();
        let mut seen_names: std::collections::HashSet<String> = std::collections::HashSet::new();

        // Discover project commands first (they take precedence)
        let project_commands = self.discover_project_commands(repo_path)?;
        for cmd in project_commands {
            seen_names.insert(cmd.name.clone());
            commands.push(cmd);
        }

        // Discover global commands (skip if name already exists from project)
        let global_commands = self.discover_global_commands()?;
        for cmd in global_commands {
            if !seen_names.contains(&cmd.name) {
                seen_names.insert(cmd.name.clone());
                commands.push(cmd);
            }
        }

        // Sort by name for consistent ordering
        commands.sort_by(|a, b| a.name.cmp(&b.name));

        Ok(commands)
    }

    /// Discovers project-specific custom commands
    fn discover_project_commands(
        &self,
        repo_path: &Path,
    ) -> CustomCommandResult<Vec<CustomCommand>> {
        let mut commands = Vec::new();

        // Claude Code project commands
        let claude_path = repo_path.join(".claude").join("commands");
        if claude_path.exists() {
            let claude_commands = self.scan_directory(
                &claude_path,
                CommandSource::Project,
                CommandFramework::ClaudeCode,
            )?;
            commands.extend(claude_commands);
        }

        // OpenCode project commands
        let opencode_path = repo_path.join(".opencode").join("command");
        if opencode_path.exists() {
            let opencode_commands = self.scan_directory(
                &opencode_path,
                CommandSource::Project,
                CommandFramework::OpenCode,
            )?;
            commands.extend(opencode_commands);
        }

        Ok(commands)
    }

    /// Discovers global custom commands from user's home directory
    fn discover_global_commands(&self) -> CustomCommandResult<Vec<CustomCommand>> {
        let mut commands = Vec::new();

        if let Some(home) = dirs::home_dir() {
            // Claude Code global commands (~/.claude/commands/)
            let claude_path = home.join(".claude").join("commands");
            if claude_path.exists() {
                let claude_commands = self.scan_directory(
                    &claude_path,
                    CommandSource::Global,
                    CommandFramework::ClaudeCode,
                )?;
                commands.extend(claude_commands);
            }

            // OpenCode global commands (~/.config/opencode/command/)
            let opencode_path = home.join(".config").join("opencode").join("command");
            if opencode_path.exists() {
                let opencode_commands = self.scan_directory(
                    &opencode_path,
                    CommandSource::Global,
                    CommandFramework::OpenCode,
                )?;
                commands.extend(opencode_commands);
            }
        }

        Ok(commands)
    }

    /// Scans a directory for custom command markdown files
    fn scan_directory(
        &self,
        dir_path: &Path,
        source: CommandSource,
        framework: CommandFramework,
    ) -> CustomCommandResult<Vec<CustomCommand>> {
        let mut commands = Vec::new();

        self.scan_directory_recursive(dir_path, dir_path, source, framework, &mut commands)?;

        Ok(commands)
    }

    /// Recursively scans a directory and its subdirectories for command files
    fn scan_directory_recursive(
        &self,
        base_path: &Path,
        current_path: &Path,
        source: CommandSource,
        framework: CommandFramework,
        commands: &mut Vec<CustomCommand>,
    ) -> CustomCommandResult<()> {
        let entries = std::fs::read_dir(current_path)?;

        for entry in entries {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                // Recurse into subdirectories
                self.scan_directory_recursive(base_path, &path, source, framework, commands)?;
            } else if path.is_file() {
                // Check for .md extension
                if let Some(ext) = path.extension() {
                    if ext == "md" {
                        if let Some(cmd) =
                            self.parse_command_file(&path, base_path, source, framework)?
                        {
                            commands.push(cmd);
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Parses a markdown file into a CustomCommand
    fn parse_command_file(
        &self,
        file_path: &Path,
        base_path: &Path,
        source: CommandSource,
        framework: CommandFramework,
    ) -> CustomCommandResult<Option<CustomCommand>> {
        let content = std::fs::read_to_string(file_path)?;

        // Get command name from filename (without .md extension)
        let name = file_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        // Get relative path for namespacing
        let relative_path = file_path
            .strip_prefix(base_path)
            .unwrap_or(file_path)
            .to_string_lossy()
            .to_string();

        // Build display name with namespace (subdirectory prefix)
        let display_name = self.build_display_name(&name, &relative_path, source, framework);

        // Parse frontmatter and body
        let (frontmatter, template) = self.parse_frontmatter_and_body(&content)?;

        // Skip empty templates
        if template.trim().is_empty() {
            return Ok(None);
        }

        let mut cmd = CustomCommand::new(name, template, source, framework, relative_path);
        cmd = cmd.with_display_name(display_name);

        if let Some(fm) = frontmatter {
            cmd = cmd.with_frontmatter(fm);
        }

        // If no description from frontmatter, use first line of template
        if cmd.description.is_none() {
            let first_line = cmd.template.lines().next().unwrap_or("").trim();
            if !first_line.is_empty() && first_line.len() <= 100 {
                cmd.description = Some(first_line.to_string());
            }
        }

        Ok(Some(cmd))
    }

    /// Builds the display name for a command, including namespace information
    fn build_display_name(
        &self,
        name: &str,
        relative_path: &str,
        source: CommandSource,
        framework: CommandFramework,
    ) -> String {
        let mut parts = Vec::new();

        // Get parent directory as namespace
        let path = PathBuf::from(relative_path);
        if let Some(parent) = path.parent() {
            let namespace = parent.to_string_lossy();
            if !namespace.is_empty() && namespace != "." {
                // Format: name (source:namespace)
                let source_str = match source {
                    CommandSource::Project => "project",
                    CommandSource::Global => "user",
                };
                parts.push(format!("{}:{}", source_str, namespace));
            } else {
                // Format: name (source)
                let source_str = match source {
                    CommandSource::Project => "project",
                    CommandSource::Global => "user",
                };
                parts.push(source_str.to_string());
            }
        }

        // Add framework indicator if it's not the default for this context
        if matches!(framework, CommandFramework::OpenCode) {
            parts.push("opencode".to_string());
        }

        if parts.is_empty() {
            name.to_string()
        } else {
            format!("{} ({})", name, parts.join(", "))
        }
    }

    /// Parses YAML frontmatter from markdown content
    fn parse_frontmatter_and_body(
        &self,
        content: &str,
    ) -> CustomCommandResult<(Option<CommandFrontmatter>, String)> {
        let content = content.trim();

        // Check if content starts with frontmatter delimiter
        if !content.starts_with("---") {
            return Ok((None, content.to_string()));
        }

        // Find the closing delimiter
        let rest = &content[3..];
        if let Some(end_idx) = rest.find("\n---") {
            let frontmatter_str = &rest[..end_idx].trim();
            let body = &rest[end_idx + 4..].trim();

            // Parse frontmatter as YAML
            let frontmatter: CommandFrontmatter = serde_yaml::from_str(frontmatter_str)
                .map_err(|e| CustomCommandError::FrontmatterParse(e.to_string()))?;

            Ok((Some(frontmatter), body.to_string()))
        } else {
            // No closing delimiter found, treat entire content as body
            Ok((None, content.to_string()))
        }
    }

    /// Gets a specific command by name for a repository
    pub fn get_command(&self, repo_path: &Path, name: &str) -> CustomCommandResult<CustomCommand> {
        let commands = self.discover_commands(repo_path)?;

        commands
            .into_iter()
            .find(|cmd| cmd.name == name)
            .ok_or_else(|| CustomCommandError::NotFound(name.to_string()))
    }

    /// Lists commands filtered by agent type
    pub fn list_commands_by_agent(
        &self,
        repo_path: &Path,
        agent_type: AIAgentType,
    ) -> CustomCommandResult<Vec<CustomCommand>> {
        let commands = self.discover_commands(repo_path)?;

        Ok(commands
            .into_iter()
            .filter(|cmd| cmd.agent_type == agent_type)
            .collect())
    }

    /// Lists commands filtered by framework
    pub fn list_commands_by_framework(
        &self,
        repo_path: &Path,
        framework: CommandFramework,
    ) -> CustomCommandResult<Vec<CustomCommand>> {
        let commands = self.discover_commands(repo_path)?;

        Ok(commands
            .into_iter()
            .filter(|cmd| cmd.framework == framework)
            .collect())
    }
}

impl Default for CustomCommandService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::*;

    fn create_test_command_file(dir: &Path, name: &str, content: &str) {
        fs::create_dir_all(dir).unwrap();
        fs::write(dir.join(format!("{}.md", name)), content).unwrap();
    }

    #[test]
    fn test_discover_project_commands_claude() {
        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path();

        let commands_dir = repo_path.join(".claude").join("commands");
        create_test_command_file(
            &commands_dir,
            "test-cmd",
            "---\ndescription: Test command\n---\nDo something useful",
        );

        let service = CustomCommandService::new();
        let commands = service.discover_commands(repo_path).unwrap();

        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].name, "test-cmd");
        assert_eq!(commands[0].description, Some("Test command".to_string()));
        assert_eq!(commands[0].framework, CommandFramework::ClaudeCode);
        assert_eq!(commands[0].source, CommandSource::Project);
    }

    #[test]
    fn test_discover_project_commands_opencode() {
        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path();

        let commands_dir = repo_path.join(".opencode").join("command");
        create_test_command_file(
            &commands_dir,
            "build",
            "---\ndescription: Build the project\nagent: open_code\n---\nRun the build process",
        );

        let service = CustomCommandService::new();
        let commands = service.discover_commands(repo_path).unwrap();

        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].name, "build");
        assert_eq!(commands[0].framework, CommandFramework::OpenCode);
        assert_eq!(commands[0].agent_type, AIAgentType::OpenCode);
    }

    #[test]
    fn test_parse_frontmatter() {
        let service = CustomCommandService::new();

        let content = r#"---
description: Test description
model: gpt-4o
subtask: true
---
This is the template body"#;

        let (frontmatter, body) = service.parse_frontmatter_and_body(content).unwrap();

        assert!(frontmatter.is_some());
        let fm = frontmatter.unwrap();
        assert_eq!(fm.description, Some("Test description".to_string()));
        assert_eq!(fm.model, Some("gpt-4o".to_string()));
        assert_eq!(fm.subtask, Some(true));
        assert_eq!(body, "This is the template body");
    }

    #[test]
    fn test_parse_no_frontmatter() {
        let service = CustomCommandService::new();

        let content = "Just a simple template without frontmatter";

        let (frontmatter, body) = service.parse_frontmatter_and_body(content).unwrap();

        assert!(frontmatter.is_none());
        assert_eq!(body, content);
    }

    #[test]
    fn test_project_commands_override_global() {
        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path();

        // Create a project command
        let commands_dir = repo_path.join(".claude").join("commands");
        create_test_command_file(
            &commands_dir,
            "shared-cmd",
            "---\ndescription: Project version\n---\nProject template",
        );

        let service = CustomCommandService::new();
        let commands = service.discover_commands(repo_path).unwrap();

        // Should have at least the project command
        let cmd = commands.iter().find(|c| c.name == "shared-cmd").unwrap();
        assert_eq!(cmd.source, CommandSource::Project);
        assert_eq!(cmd.description, Some("Project version".to_string()));
    }

    #[test]
    fn test_subdirectory_namespacing() {
        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path();

        // Create a command in a subdirectory
        let commands_dir = repo_path.join(".claude").join("commands").join("frontend");
        create_test_command_file(
            &commands_dir,
            "component",
            "---\ndescription: Create component\n---\nCreate a new component",
        );

        let service = CustomCommandService::new();
        let commands = service.discover_commands(repo_path).unwrap();

        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].name, "component");
        assert!(commands[0].display_name.contains("frontend"));
    }

    #[test]
    fn test_empty_template_skipped() {
        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path();

        let commands_dir = repo_path.join(".claude").join("commands");
        create_test_command_file(
            &commands_dir,
            "empty",
            "---\ndescription: Empty command\n---\n   ",
        );

        let service = CustomCommandService::new();
        let commands = service.discover_commands(repo_path).unwrap();

        assert!(commands.is_empty());
    }

    #[test]
    fn test_list_commands_by_agent() {
        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path();

        // Create Claude Code command
        let claude_dir = repo_path.join(".claude").join("commands");
        create_test_command_file(&claude_dir, "claude-cmd", "Claude command template");

        // Create OpenCode command
        let opencode_dir = repo_path.join(".opencode").join("command");
        create_test_command_file(&opencode_dir, "opencode-cmd", "OpenCode command template");

        let service = CustomCommandService::new();

        let claude_commands = service
            .list_commands_by_agent(repo_path, AIAgentType::ClaudeCode)
            .unwrap();
        assert_eq!(claude_commands.len(), 1);
        assert_eq!(claude_commands[0].name, "claude-cmd");

        let opencode_commands = service
            .list_commands_by_agent(repo_path, AIAgentType::OpenCode)
            .unwrap();
        assert_eq!(opencode_commands.len(), 1);
        assert_eq!(opencode_commands[0].name, "opencode-cmd");
    }
}
