//! Environment variable injection for secrets

use std::collections::HashMap;

use crate::known_keys;

/// Injects secrets into environment variables for agent execution
///
/// This function takes a map of secrets and returns environment variables
/// that should be set for the agent process.
pub fn inject_secrets_to_env(secrets: HashMap<String, String>) -> HashMap<String, String> {
    let mut env = HashMap::new();

    for (key, value) in secrets {
        match key.as_str() {
            known_keys::CLAUDE_CODE_OAUTH_TOKEN => {
                // Claude Code needs both the token and a flag to use OAuth
                env.insert("CLAUDE_CODE_USE_OAUTH".to_string(), "1".to_string());
                env.insert(known_keys::CLAUDE_CODE_OAUTH_TOKEN.to_string(), value);
            }
            known_keys::ANTHROPIC_API_KEY => {
                env.insert(known_keys::ANTHROPIC_API_KEY.to_string(), value);
            }
            known_keys::OPENAI_API_KEY => {
                env.insert(known_keys::OPENAI_API_KEY.to_string(), value);
            }
            known_keys::GOOGLE_AI_API_KEY => {
                env.insert(known_keys::GOOGLE_AI_API_KEY.to_string(), value);
                // Also set GEMINI_API_KEY for Gemini CLI
                if let Some(v) = env.get(known_keys::GOOGLE_AI_API_KEY) {
                    env.insert("GEMINI_API_KEY".to_string(), v.clone());
                }
            }
            known_keys::GITHUB_TOKEN => {
                env.insert(known_keys::GITHUB_TOKEN.to_string(), value.clone());
                // Also set GH_TOKEN for GitHub CLI
                env.insert("GH_TOKEN".to_string(), value);
            }
            _ => {
                // Pass through unknown secrets
                env.insert(key, value);
            }
        }
    }

    env
}

/// Filters secrets to only include those relevant to a specific agent
pub fn filter_secrets_for_agent(
    secrets: &HashMap<String, String>,
    agent_type: &str,
) -> HashMap<String, String> {
    let mut filtered = HashMap::new();

    let relevant_keys: &[&str] = match agent_type {
        "claude_code" => &[
            known_keys::CLAUDE_CODE_OAUTH_TOKEN,
            known_keys::ANTHROPIC_API_KEY,
            known_keys::GITHUB_TOKEN,
        ],
        "open_code" => &[
            known_keys::ANTHROPIC_API_KEY,
            known_keys::OPENAI_API_KEY,
            known_keys::GITHUB_TOKEN,
        ],
        "aider" => &[
            known_keys::ANTHROPIC_API_KEY,
            known_keys::OPENAI_API_KEY,
            known_keys::GITHUB_TOKEN,
        ],
        "gemini_cli" => &[known_keys::GOOGLE_AI_API_KEY, known_keys::GITHUB_TOKEN],
        "codex_cli" => &[known_keys::OPENAI_API_KEY, known_keys::GITHUB_TOKEN],
        "amp" => &[known_keys::ANTHROPIC_API_KEY, known_keys::GITHUB_TOKEN],
        _ => known_keys::ALL,
    };

    for key in relevant_keys {
        if let Some(value) = secrets.get(*key) {
            filtered.insert(key.to_string(), value.clone());
        }
    }

    filtered
}

/// Validates that required secrets are present for an agent
pub fn validate_secrets_for_agent(
    secrets: &HashMap<String, String>,
    agent_type: &str,
) -> Result<(), Vec<String>> {
    let required_keys: &[&str] = match agent_type {
        "claude_code" => &[known_keys::ANTHROPIC_API_KEY], // Or CLAUDE_CODE_OAUTH_TOKEN
        "open_code" => &[],                                // Can use various backends
        "aider" => &[],                                    // Can use various backends
        "gemini_cli" => &[known_keys::GOOGLE_AI_API_KEY],
        "codex_cli" => &[known_keys::OPENAI_API_KEY],
        "amp" => &[known_keys::ANTHROPIC_API_KEY],
        _ => &[],
    };

    let missing: Vec<String> = required_keys
        .iter()
        .filter(|key| !secrets.contains_key(**key))
        .map(|k| k.to_string())
        .collect();

    // Special case: Claude Code can use either OAuth or API key
    if agent_type == "claude_code"
        && !missing.is_empty()
        && secrets.contains_key(known_keys::CLAUDE_CODE_OAUTH_TOKEN)
    {
        return Ok(());
    }

    if missing.is_empty() {
        Ok(())
    } else {
        Err(missing)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inject_secrets_to_env() {
        let mut secrets = HashMap::new();
        secrets.insert(
            known_keys::CLAUDE_CODE_OAUTH_TOKEN.to_string(),
            "oauth-token".to_string(),
        );
        secrets.insert(known_keys::GITHUB_TOKEN.to_string(), "gh-token".to_string());

        let env = inject_secrets_to_env(secrets);

        assert_eq!(env.get("CLAUDE_CODE_USE_OAUTH"), Some(&"1".to_string()));
        assert_eq!(
            env.get(known_keys::CLAUDE_CODE_OAUTH_TOKEN),
            Some(&"oauth-token".to_string())
        );
        assert_eq!(env.get("GH_TOKEN"), Some(&"gh-token".to_string()));
    }

    #[test]
    fn test_filter_secrets_for_agent() {
        let mut secrets = HashMap::new();
        secrets.insert(
            known_keys::ANTHROPIC_API_KEY.to_string(),
            "ant-key".to_string(),
        );
        secrets.insert(
            known_keys::OPENAI_API_KEY.to_string(),
            "oai-key".to_string(),
        );
        secrets.insert(
            known_keys::GOOGLE_AI_API_KEY.to_string(),
            "google-key".to_string(),
        );

        let claude_secrets = filter_secrets_for_agent(&secrets, "claude_code");
        assert!(claude_secrets.contains_key(known_keys::ANTHROPIC_API_KEY));
        assert!(!claude_secrets.contains_key(known_keys::GOOGLE_AI_API_KEY));

        let gemini_secrets = filter_secrets_for_agent(&secrets, "gemini_cli");
        assert!(gemini_secrets.contains_key(known_keys::GOOGLE_AI_API_KEY));
        assert!(!gemini_secrets.contains_key(known_keys::ANTHROPIC_API_KEY));
    }

    #[test]
    fn test_validate_secrets_for_agent() {
        let mut secrets = HashMap::new();
        secrets.insert(known_keys::ANTHROPIC_API_KEY.to_string(), "key".to_string());

        assert!(validate_secrets_for_agent(&secrets, "claude_code").is_ok());
        assert!(validate_secrets_for_agent(&secrets, "gemini_cli").is_err());

        // Test OAuth fallback for Claude Code
        let mut oauth_secrets = HashMap::new();
        oauth_secrets.insert(
            known_keys::CLAUDE_CODE_OAUTH_TOKEN.to_string(),
            "token".to_string(),
        );
        assert!(validate_secrets_for_agent(&oauth_secrets, "claude_code").is_ok());
    }
}
