//! Remote URL parsing and provider detection.

use entities::VcsProviderType;
use serde::{Deserialize, Serialize};

/// Parsed remote URL information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteUrl {
    /// Original URL.
    pub url: String,
    /// Detected provider type.
    pub provider: Option<VcsProviderType>,
    /// Repository owner/organization.
    pub owner: Option<String>,
    /// Repository name.
    pub name: Option<String>,
    /// Protocol used (https, ssh, git).
    pub protocol: String,
    /// Host name.
    pub host: String,
}

impl RemoteUrl {
    /// Parses a remote URL.
    pub fn parse(url: &str) -> Self {
        let (protocol, host, path) = parse_url_parts(url);
        let provider = detect_provider(&host);
        let (owner, name) = parse_repo_path(&path);

        Self {
            url: url.to_string(),
            provider,
            owner,
            name,
            protocol,
            host,
        }
    }

    /// Returns the full repository path (owner/name).
    pub fn repo_path(&self) -> Option<String> {
        match (&self.owner, &self.name) {
            (Some(owner), Some(name)) => Some(format!("{}/{}", owner, name)),
            _ => None,
        }
    }

    /// Returns the HTTPS URL for this repository.
    pub fn https_url(&self) -> Option<String> {
        self.repo_path()
            .map(|path| format!("https://{}/{}", self.host, path))
    }

    /// Returns the SSH URL for this repository.
    pub fn ssh_url(&self) -> Option<String> {
        self.repo_path()
            .map(|path| format!("git@{}:{}.git", self.host, path))
    }

    /// Returns the web URL for viewing the repository.
    pub fn web_url(&self) -> Option<String> {
        self.https_url()
    }
}

/// Parses a URL into protocol, host, and path components.
fn parse_url_parts(url: &str) -> (String, String, String) {
    // Handle SSH URLs (git@host:path)
    if let Some(rest) = url.strip_prefix("git@") {
        if let Some((host, path)) = rest.split_once(':') {
            return ("ssh".to_string(), host.to_string(), path.to_string());
        }
    }

    // Handle HTTPS/HTTP URLs
    if let Some(rest) = url.strip_prefix("https://") {
        if let Some((host, path)) = rest.split_once('/') {
            return ("https".to_string(), host.to_string(), path.to_string());
        }
        return ("https".to_string(), rest.to_string(), String::new());
    }

    if let Some(rest) = url.strip_prefix("http://") {
        if let Some((host, path)) = rest.split_once('/') {
            return ("http".to_string(), host.to_string(), path.to_string());
        }
        return ("http".to_string(), rest.to_string(), String::new());
    }

    // Handle git:// URLs
    if let Some(rest) = url.strip_prefix("git://") {
        if let Some((host, path)) = rest.split_once('/') {
            return ("git".to_string(), host.to_string(), path.to_string());
        }
        return ("git".to_string(), rest.to_string(), String::new());
    }

    // Unknown format
    ("unknown".to_string(), String::new(), url.to_string())
}

/// Detects the VCS provider from a hostname.
fn detect_provider(host: &str) -> Option<VcsProviderType> {
    let host_lower = host.to_lowercase();
    if host_lower.contains("github.com") {
        Some(VcsProviderType::Github)
    } else if host_lower.contains("gitlab") {
        Some(VcsProviderType::Gitlab)
    } else if host_lower.contains("bitbucket") {
        Some(VcsProviderType::Bitbucket)
    } else {
        None
    }
}

/// Parses a repository path into owner and name.
fn parse_repo_path(path: &str) -> (Option<String>, Option<String>) {
    let path = path.trim_matches('/').trim_end_matches(".git");
    let parts: Vec<&str> = path.split('/').collect();

    match parts.len() {
        0 => (None, None),
        1 => (None, Some(parts[0].to_string())),
        _ => (Some(parts[0].to_string()), Some(parts[1].to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_https_url() {
        let remote = RemoteUrl::parse("https://github.com/user/repo");

        assert_eq!(remote.protocol, "https");
        assert_eq!(remote.host, "github.com");
        assert_eq!(remote.owner, Some("user".to_string()));
        assert_eq!(remote.name, Some("repo".to_string()));
        assert_eq!(remote.provider, Some(VcsProviderType::Github));
    }

    #[test]
    fn test_parse_ssh_url() {
        let remote = RemoteUrl::parse("git@github.com:user/repo.git");

        assert_eq!(remote.protocol, "ssh");
        assert_eq!(remote.host, "github.com");
        assert_eq!(remote.owner, Some("user".to_string()));
        assert_eq!(remote.name, Some("repo".to_string()));
        assert_eq!(remote.provider, Some(VcsProviderType::Github));
    }

    #[test]
    fn test_parse_gitlab_url() {
        let remote = RemoteUrl::parse("https://gitlab.com/group/project");

        assert_eq!(remote.provider, Some(VcsProviderType::Gitlab));
        assert_eq!(remote.owner, Some("group".to_string()));
        assert_eq!(remote.name, Some("project".to_string()));
    }

    #[test]
    fn test_parse_bitbucket_url() {
        let remote = RemoteUrl::parse("https://bitbucket.org/team/repo");

        assert_eq!(remote.provider, Some(VcsProviderType::Bitbucket));
        assert_eq!(remote.owner, Some("team".to_string()));
        assert_eq!(remote.name, Some("repo".to_string()));
    }

    #[test]
    fn test_url_generation() {
        let remote = RemoteUrl::parse("git@github.com:user/repo.git");

        assert_eq!(
            remote.https_url(),
            Some("https://github.com/user/repo".to_string())
        );
        assert_eq!(
            remote.ssh_url(),
            Some("git@github.com:user/repo.git".to_string())
        );
        assert_eq!(remote.repo_path(), Some("user/repo".to_string()));
    }
}
