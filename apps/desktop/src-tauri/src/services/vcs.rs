use std::sync::LazyLock;

use regex::Regex;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::entities::{BitbucketCredentials, GitHubCredentials, GitLabCredentials};

/// Sanitizes API error messages to prevent leaking sensitive data like tokens.
/// Truncates to 200 characters and redacts potential tokens.
fn sanitize_api_error(body: &str) -> String {
    // Limit length and remove potential tokens
    let truncated: String = body.chars().take(200).collect();

    // Remove potential bearer tokens, API keys, etc.
    static TOKEN_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?i)(bearer|token|ghp_|glpat-|gho_|github_pat_)[A-Za-z0-9\-_]+")
            .expect("regex is valid")
    });

    TOKEN_RE.replace_all(&truncated, "[REDACTED]").to_string()
}

#[derive(Error, Debug)]
pub enum VCSError {
    #[error("HTTP request failed: {0}")]
    Http(reqwest::Error),
    #[error("Authentication failed: Invalid or expired token")]
    AuthFailed,
    #[error("Rate limit exceeded")]
    RateLimited,
    #[error("Resource not found: {0}")]
    NotFound(String),
    #[error("Permission denied")]
    PermissionDenied,
    #[error("Provider not supported: {0}")]
    UnsupportedProvider(String),
    #[error("API error: {0}")]
    ApiError(String),
    #[error("Parse error: {0}")]
    ParseError(String),
}

pub type VCSResult<T> = Result<T, VCSError>;

/// User info from VCS provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VCSUser {
    pub username: String,
    pub name: Option<String>,
    pub avatar_url: Option<String>,
}

/// Pull request / Merge request info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VCSPullRequest {
    pub id: u64,
    pub number: u64,
    pub title: String,
    pub url: String,
    pub state: String,
    pub author: String,
    pub head_branch: String,
    pub base_branch: String,
}

/// Issue info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VCSIssue {
    pub id: u64,
    pub number: u64,
    pub title: String,
    pub url: String,
    pub state: String,
    pub labels: Vec<String>,
}

/// Service for interacting with VCS providers
pub struct VCSProviderService {
    client: Client,
}

impl VCSProviderService {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    // ========== GitHub Operations ==========

    /// Validates GitHub credentials and returns user info
    pub async fn validate_github(&self, creds: &GitHubCredentials) -> VCSResult<VCSUser> {
        let response = self
            .client
            .get("https://api.github.com/user")
            .header("Authorization", format!("Bearer {}", creds.token))
            .header("User-Agent", "delidev")
            .header("Accept", "application/vnd.github+json")
            .send()
            .await
            .map_err(VCSError::Http)?;

        let status = response.status();

        if status == 401 {
            return Err(VCSError::AuthFailed);
        }

        if status == 403 {
            return Err(VCSError::RateLimited);
        }

        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(VCSError::ApiError(format!(
                "GitHub API returned status {}: {}",
                status,
                sanitize_api_error(&body)
            )));
        }

        let user: GitHubUser = response.json().await.map_err(|e| {
            VCSError::ParseError(format!("Failed to parse GitHub user response: {}", e))
        })?;

        Ok(VCSUser {
            username: user.login,
            name: user.name,
            avatar_url: user.avatar_url,
        })
    }

    /// Creates a pull request on GitHub
    #[allow(clippy::too_many_arguments)]
    pub async fn create_github_pr(
        &self,
        creds: &GitHubCredentials,
        owner: &str,
        repo: &str,
        title: &str,
        body: &str,
        head: &str,
        base: &str,
    ) -> VCSResult<VCSPullRequest> {
        let url = format!("https://api.github.com/repos/{}/{}/pulls", owner, repo);

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", creds.token))
            .header("User-Agent", "delidev")
            .header("Accept", "application/vnd.github+json")
            .json(&serde_json::json!({
                "title": title,
                "body": body,
                "head": head,
                "base": base,
            }))
            .send()
            .await
            .map_err(VCSError::Http)?;

        let status = response.status();

        if status == 401 {
            return Err(VCSError::AuthFailed);
        }

        if status == 403 {
            return Err(VCSError::PermissionDenied);
        }

        if status == 404 {
            return Err(VCSError::NotFound(format!("{}/{}", owner, repo)));
        }

        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(VCSError::ApiError(format!(
                "GitHub API returned status {}: {}",
                status,
                sanitize_api_error(&body)
            )));
        }

        let pr: GitHubPR = response.json().await.map_err(|e| {
            VCSError::ParseError(format!("Failed to parse GitHub PR response: {}", e))
        })?;

        Ok(VCSPullRequest {
            id: pr.id,
            number: pr.number,
            title: pr.title,
            url: pr.html_url,
            state: pr.state,
            author: pr.user.login,
            head_branch: pr.head.ref_field,
            base_branch: pr.base.ref_field,
        })
    }

    /// Finds an existing open PR by head branch name on GitHub
    /// Returns the PR URL if found, None otherwise
    pub async fn find_github_pr_by_branch(
        &self,
        creds: &GitHubCredentials,
        owner: &str,
        repo: &str,
        head_branch: &str,
    ) -> VCSResult<Option<VCSPullRequest>> {
        let url = format!("https://api.github.com/repos/{}/{}/pulls", owner, repo);

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", creds.token))
            .header("User-Agent", "delidev")
            .header("Accept", "application/vnd.github+json")
            .query(&[
                ("state", "open"),
                ("head", &format!("{}:{}", owner, head_branch)),
            ])
            .send()
            .await
            .map_err(VCSError::Http)?;

        let status = response.status();

        if status == 401 {
            return Err(VCSError::AuthFailed);
        }

        if status == 403 {
            return Err(VCSError::PermissionDenied);
        }

        if status == 404 {
            return Err(VCSError::NotFound(format!("{}/{}", owner, repo)));
        }

        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(VCSError::ApiError(format!(
                "GitHub API returned status {}: {}",
                status,
                sanitize_api_error(&body)
            )));
        }

        let prs: Vec<GitHubPR> = response.json().await.map_err(|e| {
            VCSError::ParseError(format!("Failed to parse GitHub PRs response: {}", e))
        })?;

        // Find the PR with matching head branch
        let matching_pr = prs.into_iter().find(|pr| pr.head.ref_field == head_branch);

        Ok(matching_pr.map(|pr| VCSPullRequest {
            id: pr.id,
            number: pr.number,
            title: pr.title,
            url: pr.html_url,
            state: pr.state,
            author: pr.user.login,
            head_branch: pr.head.ref_field,
            base_branch: pr.base.ref_field,
        }))
    }

    /// Gets open issues from GitHub repository
    pub async fn list_github_issues(
        &self,
        creds: &GitHubCredentials,
        owner: &str,
        repo: &str,
    ) -> VCSResult<Vec<VCSIssue>> {
        let url = format!("https://api.github.com/repos/{}/{}/issues", owner, repo);

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", creds.token))
            .header("User-Agent", "delidev")
            .header("Accept", "application/vnd.github+json")
            .query(&[("state", "open"), ("per_page", "100")])
            .send()
            .await
            .map_err(VCSError::Http)?;

        let status = response.status();

        if status == 401 {
            return Err(VCSError::AuthFailed);
        }

        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(VCSError::ApiError(format!(
                "GitHub API returned status {}: {}",
                status,
                sanitize_api_error(&body)
            )));
        }

        let issues: Vec<GitHubIssue> = response.json().await.map_err(|e| {
            VCSError::ParseError(format!("Failed to parse GitHub issues response: {}", e))
        })?;

        Ok(issues
            .into_iter()
            .filter(|i| i.pull_request.is_none()) // Filter out PRs
            .map(|i| VCSIssue {
                id: i.id,
                number: i.number,
                title: i.title,
                url: i.html_url,
                state: i.state,
                labels: i.labels.into_iter().map(|l| l.name).collect(),
            })
            .collect())
    }

    // ========== GitLab Operations ==========

    /// Validates GitLab credentials and returns user info
    pub async fn validate_gitlab(&self, creds: &GitLabCredentials) -> VCSResult<VCSUser> {
        let response = self
            .client
            .get("https://gitlab.com/api/v4/user")
            .header("PRIVATE-TOKEN", &creds.token)
            .send()
            .await
            .map_err(VCSError::Http)?;

        let status = response.status();

        if status == 401 {
            return Err(VCSError::AuthFailed);
        }

        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(VCSError::ApiError(format!(
                "GitLab API returned status {}: {}",
                status,
                sanitize_api_error(&body)
            )));
        }

        let user: GitLabUser = response.json().await.map_err(|e| {
            VCSError::ParseError(format!("Failed to parse GitLab user response: {}", e))
        })?;

        Ok(VCSUser {
            username: user.username,
            name: Some(user.name),
            avatar_url: user.avatar_url,
        })
    }

    // ========== Bitbucket Operations ==========

    /// Validates Bitbucket credentials and returns user info
    pub async fn validate_bitbucket(&self, creds: &BitbucketCredentials) -> VCSResult<VCSUser> {
        let response = self
            .client
            .get("https://api.bitbucket.org/2.0/user")
            .basic_auth(&creds.username, Some(&creds.app_password))
            .send()
            .await
            .map_err(VCSError::Http)?;

        let status = response.status();

        if status == 401 {
            return Err(VCSError::AuthFailed);
        }

        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(VCSError::ApiError(format!(
                "Bitbucket API returned status {}: {}",
                status,
                sanitize_api_error(&body)
            )));
        }

        let user: BitbucketUser = response.json().await.map_err(|e| {
            VCSError::ParseError(format!("Failed to parse Bitbucket user response: {}", e))
        })?;

        Ok(VCSUser {
            username: user.username,
            name: Some(user.display_name),
            avatar_url: user.links.avatar.href,
        })
    }
}

impl Default for VCSProviderService {
    fn default() -> Self {
        Self::new()
    }
}

// ========== GitHub API Types ==========

#[derive(Deserialize)]
struct GitHubUser {
    login: String,
    name: Option<String>,
    avatar_url: Option<String>,
}

#[derive(Deserialize)]
struct GitHubPR {
    id: u64,
    number: u64,
    title: String,
    html_url: String,
    state: String,
    user: GitHubUser,
    head: GitHubRef,
    base: GitHubRef,
}

#[derive(Deserialize)]
struct GitHubRef {
    #[serde(rename = "ref")]
    ref_field: String,
}

#[derive(Deserialize)]
struct GitHubIssue {
    id: u64,
    number: u64,
    title: String,
    html_url: String,
    state: String,
    labels: Vec<GitHubLabel>,
    pull_request: Option<serde_json::Value>,
}

#[derive(Deserialize)]
struct GitHubLabel {
    name: String,
}

// ========== GitLab API Types ==========

#[derive(Deserialize)]
struct GitLabUser {
    username: String,
    name: String,
    avatar_url: Option<String>,
}

// ========== Bitbucket API Types ==========

#[derive(Deserialize)]
struct BitbucketUser {
    username: String,
    display_name: String,
    links: BitbucketLinks,
}

#[derive(Deserialize)]
struct BitbucketLinks {
    avatar: BitbucketLink,
}

#[derive(Deserialize)]
struct BitbucketLink {
    href: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    mod sanitize_api_error {
        use super::*;

        #[test]
        fn test_truncates_messages_over_200_characters() {
            let long_message = "a".repeat(300);
            let sanitized = sanitize_api_error(&long_message);
            assert_eq!(sanitized.len(), 200);
        }

        #[test]
        fn test_redacts_various_token_formats() {
            let test_cases = [
                ("Authorization: Bearer ghp_1234567890abcdef failed", "ghp_"),
                ("Token github_pat_abcdef123456 is invalid", "github_pat_"),
                ("Auth failed with glpat-abcdef123456", "glpat-"),
                ("gho_abcdef123456 is expired", "gho_"),
                ("BEARER token123abc is invalid", "token123abc"),
            ];

            for (msg, token_prefix) in test_cases {
                let sanitized = sanitize_api_error(msg);
                assert!(
                    sanitized.contains("[REDACTED]"),
                    "Should redact tokens starting with {}",
                    token_prefix
                );
            }
        }

        #[test]
        fn test_preserves_safe_messages_without_tokens() {
            let msg = "Rate limit exceeded, please try again later";
            let sanitized = sanitize_api_error(msg);
            assert_eq!(sanitized, msg);
        }
    }

    mod vcs_error {
        use super::*;

        #[test]
        fn test_all_error_variants_display_correctly() {
            let test_cases = [
                (
                    VCSError::AuthFailed,
                    "Authentication failed: Invalid or expired token",
                ),
                (VCSError::RateLimited, "Rate limit exceeded"),
                (
                    VCSError::NotFound("owner/repo".to_string()),
                    "Resource not found: owner/repo",
                ),
                (VCSError::PermissionDenied, "Permission denied"),
                (
                    VCSError::UnsupportedProvider("Gitea".to_string()),
                    "Provider not supported: Gitea",
                ),
                (
                    VCSError::ApiError("Something went wrong".to_string()),
                    "API error: Something went wrong",
                ),
                (
                    VCSError::ParseError("Invalid JSON".to_string()),
                    "Parse error: Invalid JSON",
                ),
            ];

            for (error, expected_message) in test_cases {
                assert_eq!(format!("{}", error), expected_message);
            }
        }
    }
}
