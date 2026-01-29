use serde::{Deserialize, Serialize};

use super::{VCSProviderType, VCSType};

/// A managed repository
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Repository {
    /// Unique identifier
    pub id: String,
    /// Version control system type
    pub vcs_type: VCSType,
    /// VCS hosting provider type
    pub vcs_provider_type: VCSProviderType,
    /// Remote URL (e.g., https://github.com/user/repo)
    pub remote_url: String,
    /// Repository name
    pub name: String,
    /// Local path to the repository
    pub local_path: String,
    /// Default branch name
    pub default_branch: String,
    /// Created timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl Repository {
    pub fn new(
        id: String,
        name: String,
        local_path: String,
        remote_url: String,
        vcs_provider_type: VCSProviderType,
    ) -> Self {
        Self {
            id,
            vcs_type: VCSType::Git,
            vcs_provider_type,
            remote_url,
            name,
            local_path,
            default_branch: "main".to_string(),
            created_at: chrono::Utc::now(),
        }
    }

    pub fn with_default_branch(mut self, branch: impl Into<String>) -> Self {
        self.default_branch = branch.into();
        self
    }

    /// Detects VCS provider type from remote URL
    pub fn detect_provider_from_url(url: &str) -> Option<VCSProviderType> {
        let url_lower = url.to_lowercase();
        if url_lower.contains("github.com") {
            Some(VCSProviderType::GitHub)
        } else if url_lower.contains("gitlab.com") || url_lower.contains("gitlab") {
            Some(VCSProviderType::GitLab)
        } else if url_lower.contains("bitbucket.org") || url_lower.contains("bitbucket") {
            Some(VCSProviderType::Bitbucket)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod detect_provider_from_url {
        use super::*;

        #[test]
        fn test_detects_github_from_various_url_formats() {
            assert_eq!(
                Repository::detect_provider_from_url("https://github.com/owner/repo.git"),
                Some(VCSProviderType::GitHub)
            );
            assert_eq!(
                Repository::detect_provider_from_url("git@github.com:owner/repo.git"),
                Some(VCSProviderType::GitHub)
            );
        }

        #[test]
        fn test_detects_gitlab_including_self_hosted() {
            assert_eq!(
                Repository::detect_provider_from_url("https://gitlab.com/owner/repo.git"),
                Some(VCSProviderType::GitLab)
            );
            assert_eq!(
                Repository::detect_provider_from_url("https://gitlab.company.com/owner/repo.git"),
                Some(VCSProviderType::GitLab)
            );
        }

        #[test]
        fn test_detects_bitbucket_including_self_hosted() {
            assert_eq!(
                Repository::detect_provider_from_url("https://bitbucket.org/owner/repo.git"),
                Some(VCSProviderType::Bitbucket)
            );
            assert_eq!(
                Repository::detect_provider_from_url(
                    "https://bitbucket.company.com/owner/repo.git"
                ),
                Some(VCSProviderType::Bitbucket)
            );
        }

        #[test]
        fn test_returns_none_for_unknown_providers() {
            assert_eq!(
                Repository::detect_provider_from_url("https://custom-git.example.com/repo.git"),
                None
            );
        }

        #[test]
        fn test_case_insensitive_detection() {
            assert_eq!(
                Repository::detect_provider_from_url("https://GITHUB.COM/owner/repo.git"),
                Some(VCSProviderType::GitHub)
            );
        }
    }
}
