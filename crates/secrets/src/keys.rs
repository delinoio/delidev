//! Known secret key definitions.

use serde::{Deserialize, Serialize};

/// Known secret keys used by DeliDev and its supported AI agents.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SecretKey {
    /// Claude Code OAuth token.
    ClaudeCodeOAuthToken,
    /// Anthropic API key.
    AnthropicApiKey,
    /// OpenAI API key.
    OpenAiApiKey,
    /// Google AI API key.
    GoogleAiApiKey,
    /// GitHub access token.
    GithubToken,
    /// GitLab access token.
    GitlabToken,
    /// Bitbucket app password.
    BitbucketAppPassword,
}

impl SecretKey {
    /// Returns the storage key name.
    pub fn key_name(&self) -> &'static str {
        match self {
            Self::ClaudeCodeOAuthToken => "CLAUDE_CODE_OAUTH_TOKEN",
            Self::AnthropicApiKey => "ANTHROPIC_API_KEY",
            Self::OpenAiApiKey => "OPENAI_API_KEY",
            Self::GoogleAiApiKey => "GOOGLE_AI_API_KEY",
            Self::GithubToken => "GITHUB_TOKEN",
            Self::GitlabToken => "GITLAB_TOKEN",
            Self::BitbucketAppPassword => "BITBUCKET_APP_PASSWORD",
        }
    }

    /// Returns the environment variable name for this secret.
    pub fn env_var_name(&self) -> &'static str {
        match self {
            Self::ClaudeCodeOAuthToken => "CLAUDE_CODE_OAUTH_TOKEN",
            Self::AnthropicApiKey => "ANTHROPIC_API_KEY",
            Self::OpenAiApiKey => "OPENAI_API_KEY",
            Self::GoogleAiApiKey => "GOOGLE_AI_API_KEY",
            Self::GithubToken => "GITHUB_TOKEN",
            Self::GitlabToken => "GITLAB_TOKEN",
            Self::BitbucketAppPassword => "BITBUCKET_APP_PASSWORD",
        }
    }

    /// Returns all known secret keys.
    pub fn all() -> &'static [SecretKey] {
        &[
            Self::ClaudeCodeOAuthToken,
            Self::AnthropicApiKey,
            Self::OpenAiApiKey,
            Self::GoogleAiApiKey,
            Self::GithubToken,
            Self::GitlabToken,
            Self::BitbucketAppPassword,
        ]
    }

    /// Returns a description of this secret.
    pub fn description(&self) -> &'static str {
        match self {
            Self::ClaudeCodeOAuthToken => "Claude Code OAuth token",
            Self::AnthropicApiKey => "Anthropic API key for Claude models",
            Self::OpenAiApiKey => "OpenAI API key for GPT models",
            Self::GoogleAiApiKey => "Google AI API key for Gemini models",
            Self::GithubToken => "GitHub personal access token",
            Self::GitlabToken => "GitLab personal access token",
            Self::BitbucketAppPassword => "Bitbucket app password",
        }
    }

    /// Returns the AI agents that use this secret.
    pub fn used_by(&self) -> &'static [&'static str] {
        match self {
            Self::ClaudeCodeOAuthToken => &["Claude Code"],
            Self::AnthropicApiKey => &["Claude Code", "Amp"],
            Self::OpenAiApiKey => &["OpenCode", "Aider", "Codex CLI"],
            Self::GoogleAiApiKey => &["Gemini CLI"],
            Self::GithubToken => &["All agents (for GitHub operations)"],
            Self::GitlabToken => &["All agents (for GitLab operations)"],
            Self::BitbucketAppPassword => &["All agents (for Bitbucket operations)"],
        }
    }
}

impl std::fmt::Display for SecretKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.key_name())
    }
}

impl TryFrom<&str> for SecretKey {
    type Error = String;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s {
            "CLAUDE_CODE_OAUTH_TOKEN" => Ok(Self::ClaudeCodeOAuthToken),
            "ANTHROPIC_API_KEY" => Ok(Self::AnthropicApiKey),
            "OPENAI_API_KEY" => Ok(Self::OpenAiApiKey),
            "GOOGLE_AI_API_KEY" => Ok(Self::GoogleAiApiKey),
            "GITHUB_TOKEN" => Ok(Self::GithubToken),
            "GITLAB_TOKEN" => Ok(Self::GitlabToken),
            "BITBUCKET_APP_PASSWORD" => Ok(Self::BitbucketAppPassword),
            _ => Err(format!("Unknown secret key: {}", s)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_secret_key_names() {
        assert_eq!(
            SecretKey::ClaudeCodeOAuthToken.key_name(),
            "CLAUDE_CODE_OAUTH_TOKEN"
        );
        assert_eq!(SecretKey::AnthropicApiKey.key_name(), "ANTHROPIC_API_KEY");
        assert_eq!(SecretKey::OpenAiApiKey.key_name(), "OPENAI_API_KEY");
    }

    #[test]
    fn test_secret_key_from_string() {
        let key: SecretKey = "ANTHROPIC_API_KEY".try_into().unwrap();
        assert_eq!(key, SecretKey::AnthropicApiKey);

        let result: Result<SecretKey, _> = "UNKNOWN_KEY".try_into();
        assert!(result.is_err());
    }

    #[test]
    fn test_all_keys() {
        let keys = SecretKey::all();
        assert!(keys.len() >= 7);
    }
}
