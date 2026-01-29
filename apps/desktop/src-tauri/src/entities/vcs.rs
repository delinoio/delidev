use serde::{Deserialize, Serialize};

/// Version Control System types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum VCSType {
    #[default]
    Git,
}

/// Merge strategy for local merging
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum MergeStrategy {
    /// Standard merge commit (default)
    #[default]
    Merge,
    /// Squash all commits into a single commit
    Squash,
    /// Rebase commits onto target branch
    Rebase,
}

impl MergeStrategy {
    /// Returns the display name for the strategy
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Merge => "Merge commit",
            Self::Squash => "Squash and merge",
            Self::Rebase => "Rebase and merge",
        }
    }

    /// Returns a description of what this strategy does
    pub fn description(&self) -> &'static str {
        match self {
            Self::Merge => "Create a merge commit preserving all commits",
            Self::Squash => "Combine all commits into a single commit",
            Self::Rebase => "Reapply commits on top of the target branch",
        }
    }
}

/// VCS hosting provider types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VCSProviderType {
    #[serde(rename = "github")]
    GitHub,
    #[serde(rename = "gitlab")]
    GitLab,
    #[serde(rename = "bitbucket")]
    Bitbucket,
}

impl VCSProviderType {
    /// Returns the display name for the provider
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::GitHub => "GitHub",
            Self::GitLab => "GitLab",
            Self::Bitbucket => "Bitbucket",
        }
    }

    /// Returns the required scopes for authentication
    pub fn required_scopes(&self) -> &'static [&'static str] {
        match self {
            Self::GitHub => &["repo", "read:user", "workflow"],
            Self::GitLab => &["api", "read_user", "read_repository", "write_repository"],
            Self::Bitbucket => &[
                "repository:read",
                "repository:write",
                "pullrequest:read",
                "pullrequest:write",
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod vcs_provider_type {
        use super::*;

        #[test]
        fn test_display_name_returns_human_readable_names() {
            assert_eq!(VCSProviderType::GitHub.display_name(), "GitHub");
            assert_eq!(VCSProviderType::GitLab.display_name(), "GitLab");
            assert_eq!(VCSProviderType::Bitbucket.display_name(), "Bitbucket");
        }

        #[test]
        fn test_required_scopes_returns_provider_specific_scopes() {
            let github_scopes = VCSProviderType::GitHub.required_scopes();
            let gitlab_scopes = VCSProviderType::GitLab.required_scopes();
            let bitbucket_scopes = VCSProviderType::Bitbucket.required_scopes();

            // GitHub requires repo, read:user, and workflow
            assert!(github_scopes.contains(&"repo"));
            assert!(github_scopes.contains(&"workflow"));

            // GitLab requires api and repository permissions
            assert!(gitlab_scopes.contains(&"api"));
            assert!(gitlab_scopes.contains(&"read_repository"));

            // Bitbucket uses granular permissions
            assert!(bitbucket_scopes.contains(&"repository:read"));
            assert!(bitbucket_scopes.contains(&"pullrequest:write"));
        }

        #[test]
        fn test_serialization_roundtrip_all_providers() {
            let providers = [
                (VCSProviderType::GitHub, "\"github\""),
                (VCSProviderType::GitLab, "\"gitlab\""),
                (VCSProviderType::Bitbucket, "\"bitbucket\""),
            ];

            for (provider, expected_json) in providers {
                let json = serde_json::to_string(&provider).unwrap();
                assert_eq!(json, expected_json);

                let deserialized: VCSProviderType = serde_json::from_str(&json).unwrap();
                assert_eq!(deserialized, provider);
            }
        }
    }

    mod merge_strategy {
        use super::*;

        #[test]
        fn test_display_name_returns_human_readable_names() {
            assert_eq!(MergeStrategy::Merge.display_name(), "Merge commit");
            assert_eq!(MergeStrategy::Squash.display_name(), "Squash and merge");
            assert_eq!(MergeStrategy::Rebase.display_name(), "Rebase and merge");
        }

        #[test]
        fn test_description_returns_strategy_descriptions() {
            assert!(MergeStrategy::Merge.description().contains("merge commit"));
            assert!(MergeStrategy::Squash
                .description()
                .contains("single commit"));
            assert!(MergeStrategy::Rebase.description().contains("top of"));
        }

        #[test]
        fn test_default_is_merge() {
            assert_eq!(MergeStrategy::default(), MergeStrategy::Merge);
        }

        #[test]
        fn test_serialization_roundtrip_all_strategies() {
            let strategies = [
                (MergeStrategy::Merge, "\"merge\""),
                (MergeStrategy::Squash, "\"squash\""),
                (MergeStrategy::Rebase, "\"rebase\""),
            ];

            for (strategy, expected_json) in strategies {
                let json = serde_json::to_string(&strategy).unwrap();
                assert_eq!(json, expected_json);

                let deserialized: MergeStrategy = serde_json::from_str(&json).unwrap();
                assert_eq!(deserialized, strategy);
            }
        }
    }
}
