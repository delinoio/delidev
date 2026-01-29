use serde::{Deserialize, Serialize};

/// A group of repositories that can be used together for tasks
///
/// When name is None, this is a single-repository group and should
/// display the repository name instead of a group name.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryGroup {
    /// Unique identifier
    pub id: String,
    /// Group name (None for single-repository groups)
    pub name: Option<String>,
    /// Parent workspace ID
    pub workspace_id: String,
    /// List of repository IDs in this group
    pub repository_ids: Vec<String>,
    /// Created timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Updated timestamp
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl RepositoryGroup {
    /// Creates a new repository group
    pub fn new(id: String, workspace_id: String) -> Self {
        let now = chrono::Utc::now();
        Self {
            id,
            name: None,
            workspace_id,
            repository_ids: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }

    /// Creates a new named repository group
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Creates a single-repository group
    pub fn single_repo(id: String, workspace_id: String, repository_id: String) -> Self {
        let now = chrono::Utc::now();
        Self {
            id,
            name: None, // Single-repo groups have no name
            workspace_id,
            repository_ids: vec![repository_id],
            created_at: now,
            updated_at: now,
        }
    }

    /// Returns true if this is a single-repository group
    pub fn is_single_repo(&self) -> bool {
        self.name.is_none() && self.repository_ids.len() == 1
    }

    /// Adds a repository to the group
    pub fn add_repository(&mut self, repository_id: String) {
        if !self.repository_ids.contains(&repository_id) {
            self.repository_ids.push(repository_id);
            self.updated_at = chrono::Utc::now();
        }
    }

    /// Removes a repository from the group
    pub fn remove_repository(&mut self, repository_id: &str) -> bool {
        let len_before = self.repository_ids.len();
        self.repository_ids.retain(|id| id != repository_id);
        let removed = self.repository_ids.len() < len_before;
        if removed {
            self.updated_at = chrono::Utc::now();
        }
        removed
    }
}
