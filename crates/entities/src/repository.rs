//! Repository-related entity definitions.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{VcsProviderType, VcsType};

/// A repository tracked by DeliDev.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Repository {
    /// Unique identifier.
    pub id: Uuid,
    /// Associated workspace ID.
    pub workspace_id: Uuid,
    /// Repository name.
    pub name: String,
    /// Remote URL.
    pub remote_url: String,
    /// Default branch name.
    pub default_branch: String,
    /// Version control system type.
    pub vcs_type: VcsType,
    /// VCS provider type.
    pub vcs_provider_type: VcsProviderType,
    /// When this record was created.
    pub created_at: DateTime<Utc>,
    /// When this record was last updated.
    pub updated_at: DateTime<Utc>,
}

impl Repository {
    /// Creates a new repository.
    pub fn new(
        workspace_id: Uuid,
        name: impl Into<String>,
        remote_url: impl Into<String>,
        vcs_provider_type: VcsProviderType,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            workspace_id,
            name: name.into(),
            remote_url: remote_url.into(),
            default_branch: "main".to_string(),
            vcs_type: VcsType::Git,
            vcs_provider_type,
            created_at: now,
            updated_at: now,
        }
    }

    /// Sets the default branch.
    pub fn with_default_branch(mut self, branch: impl Into<String>) -> Self {
        self.default_branch = branch.into();
        self
    }

    /// Detects the VCS provider type from a remote URL.
    pub fn detect_provider(remote_url: &str) -> Option<VcsProviderType> {
        let url_lower = remote_url.to_lowercase();
        if url_lower.contains("github.com") {
            Some(VcsProviderType::Github)
        } else if url_lower.contains("gitlab.com") || url_lower.contains("gitlab") {
            Some(VcsProviderType::Gitlab)
        } else if url_lower.contains("bitbucket.org") || url_lower.contains("bitbucket") {
            Some(VcsProviderType::Bitbucket)
        } else {
            None
        }
    }
}

/// A group of repositories that work together.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryGroup {
    /// Unique identifier.
    pub id: Uuid,
    /// Associated workspace ID.
    pub workspace_id: Uuid,
    /// Optional group name (None for single-repo groups).
    pub name: Option<String>,
    /// Repository IDs in this group.
    pub repository_ids: Vec<Uuid>,
    /// When this record was created.
    pub created_at: DateTime<Utc>,
    /// When this record was last updated.
    pub updated_at: DateTime<Utc>,
}

impl RepositoryGroup {
    /// Creates a new repository group.
    pub fn new(workspace_id: Uuid) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            workspace_id,
            name: None,
            repository_ids: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }

    /// Sets the name for this group.
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Adds a repository to this group.
    pub fn add_repository(&mut self, repository_id: Uuid) {
        if !self.repository_ids.contains(&repository_id) {
            self.repository_ids.push(repository_id);
        }
    }

    /// Removes a repository from this group.
    pub fn remove_repository(&mut self, repository_id: Uuid) {
        self.repository_ids.retain(|id| *id != repository_id);
    }

    /// Returns true if this is a single-repository group.
    pub fn is_single_repo(&self) -> bool {
        self.repository_ids.len() == 1 && self.name.is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_repository_creation() {
        let workspace_id = Uuid::new_v4();
        let repo = Repository::new(
            workspace_id,
            "my-repo",
            "https://github.com/user/my-repo",
            VcsProviderType::Github,
        )
        .with_default_branch("develop");

        assert_eq!(repo.workspace_id, workspace_id);
        assert_eq!(repo.name, "my-repo");
        assert_eq!(repo.remote_url, "https://github.com/user/my-repo");
        assert_eq!(repo.default_branch, "develop");
        assert_eq!(repo.vcs_type, VcsType::Git);
        assert_eq!(repo.vcs_provider_type, VcsProviderType::Github);
    }

    #[test]
    fn test_detect_provider() {
        assert_eq!(
            Repository::detect_provider("https://github.com/user/repo"),
            Some(VcsProviderType::Github)
        );
        assert_eq!(
            Repository::detect_provider("git@github.com:user/repo.git"),
            Some(VcsProviderType::Github)
        );
        assert_eq!(
            Repository::detect_provider("https://gitlab.com/user/repo"),
            Some(VcsProviderType::Gitlab)
        );
        assert_eq!(
            Repository::detect_provider("https://bitbucket.org/user/repo"),
            Some(VcsProviderType::Bitbucket)
        );
        assert_eq!(
            Repository::detect_provider("https://custom.server.com/repo"),
            None
        );
    }

    #[test]
    fn test_repository_group() {
        let workspace_id = Uuid::new_v4();
        let mut group = RepositoryGroup::new(workspace_id).with_name("Backend Services");

        let repo1_id = Uuid::new_v4();
        let repo2_id = Uuid::new_v4();

        group.add_repository(repo1_id);
        group.add_repository(repo2_id);
        group.add_repository(repo1_id); // Duplicate should not be added

        assert_eq!(group.repository_ids.len(), 2);
        assert!(!group.is_single_repo());

        group.remove_repository(repo2_id);
        assert_eq!(group.repository_ids.len(), 1);
    }
}
