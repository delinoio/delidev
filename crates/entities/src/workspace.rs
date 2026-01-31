//! Workspace-related entity definitions.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A workspace groups repositories and tasks together.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workspace {
    /// Unique identifier.
    pub id: Uuid,
    /// Workspace name.
    pub name: String,
    /// Optional description.
    pub description: Option<String>,
    /// Associated user ID (None in single-user mode).
    pub user_id: Option<Uuid>,
    /// When this record was created.
    pub created_at: DateTime<Utc>,
    /// When this record was last updated.
    pub updated_at: DateTime<Utc>,
}

impl Workspace {
    /// Creates a new workspace.
    pub fn new(name: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            description: None,
            user_id: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Sets the description for this workspace.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Sets the user ID for this workspace.
    pub fn with_user_id(mut self, user_id: Uuid) -> Self {
        self.user_id = Some(user_id);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workspace_creation() {
        let workspace = Workspace::new("My Workspace")
            .with_description("A workspace for my projects");

        assert_eq!(workspace.name, "My Workspace");
        assert_eq!(
            workspace.description,
            Some("A workspace for my projects".to_string())
        );
        assert!(workspace.user_id.is_none());
    }

    #[test]
    fn test_workspace_with_user() {
        let user_id = Uuid::new_v4();
        let workspace = Workspace::new("Team Workspace").with_user_id(user_id);

        assert_eq!(workspace.user_id, Some(user_id));
    }
}
