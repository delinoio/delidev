use serde::{Deserialize, Serialize};

/// A workspace containing repositories and repository groups
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workspace {
    /// Unique identifier
    pub id: String,
    /// Workspace name
    pub name: String,
    /// Optional description
    pub description: Option<String>,
    /// Created timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Updated timestamp
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl Workspace {
    /// Creates a new workspace
    pub fn new(id: String, name: String) -> Self {
        let now = chrono::Utc::now();
        Self {
            id,
            name,
            description: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Creates a new workspace with description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Creates the default workspace
    pub fn default_workspace() -> Self {
        Self::new(uuid::Uuid::new_v4().to_string(), "Default".to_string())
            .with_description("Default workspace")
    }
}
