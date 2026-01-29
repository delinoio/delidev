use std::sync::Arc;

use crate::{
    database::{Database, DatabaseResult, WorkspaceRow},
    entities::Workspace,
};

/// Service for managing workspaces
pub struct WorkspaceService {
    db: Arc<Database>,
}

impl WorkspaceService {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    /// Lists all workspaces
    pub async fn list(&self) -> DatabaseResult<Vec<Workspace>> {
        let rows: Vec<WorkspaceRow> = sqlx::query_as(
            "SELECT id, name, description, created_at, updated_at
             FROM workspaces
             ORDER BY name",
        )
        .fetch_all(self.db.pool())
        .await?;

        Ok(rows.into_iter().map(Workspace::from).collect())
    }

    /// Gets a workspace by ID
    pub async fn get(&self, id: &str) -> DatabaseResult<Option<Workspace>> {
        let row: Option<WorkspaceRow> = sqlx::query_as(
            "SELECT id, name, description, created_at, updated_at
             FROM workspaces
             WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(self.db.pool())
        .await?;

        Ok(row.map(Workspace::from))
    }

    /// Creates a new workspace
    pub async fn create(&self, workspace: &Workspace) -> DatabaseResult<()> {
        let row = WorkspaceRow::from(workspace);

        sqlx::query(
            "INSERT INTO workspaces (id, name, description, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(&row.id)
        .bind(&row.name)
        .bind(&row.description)
        .bind(&row.created_at)
        .bind(&row.updated_at)
        .execute(self.db.pool())
        .await?;

        Ok(())
    }

    /// Updates a workspace
    pub async fn update(&self, workspace: &Workspace) -> DatabaseResult<()> {
        let now = chrono::Utc::now().to_rfc3339();

        sqlx::query(
            "UPDATE workspaces SET name = ?, description = ?, updated_at = ?
             WHERE id = ?",
        )
        .bind(&workspace.name)
        .bind(&workspace.description)
        .bind(&now)
        .bind(&workspace.id)
        .execute(self.db.pool())
        .await?;

        Ok(())
    }

    /// Deletes a workspace
    pub async fn delete(&self, id: &str) -> DatabaseResult<()> {
        sqlx::query("DELETE FROM workspaces WHERE id = ?")
            .bind(id)
            .execute(self.db.pool())
            .await?;
        Ok(())
    }

    /// Adds a repository to a workspace
    pub async fn add_repository(
        &self,
        workspace_id: &str,
        repository_id: &str,
    ) -> DatabaseResult<()> {
        sqlx::query(
            "INSERT OR IGNORE INTO workspace_repositories (workspace_id, repository_id)
             VALUES (?, ?)",
        )
        .bind(workspace_id)
        .bind(repository_id)
        .execute(self.db.pool())
        .await?;

        Ok(())
    }

    /// Removes a repository from a workspace
    pub async fn remove_repository(
        &self,
        workspace_id: &str,
        repository_id: &str,
    ) -> DatabaseResult<()> {
        sqlx::query(
            "DELETE FROM workspace_repositories
             WHERE workspace_id = ? AND repository_id = ?",
        )
        .bind(workspace_id)
        .bind(repository_id)
        .execute(self.db.pool())
        .await?;

        Ok(())
    }

    /// Lists repository IDs in a workspace
    pub async fn list_repository_ids(&self, workspace_id: &str) -> DatabaseResult<Vec<String>> {
        let rows: Vec<(String,)> = sqlx::query_as(
            "SELECT repository_id FROM workspace_repositories
             WHERE workspace_id = ?",
        )
        .bind(workspace_id)
        .fetch_all(self.db.pool())
        .await?;

        Ok(rows.into_iter().map(|(id,)| id).collect())
    }

    /// Gets or creates the default workspace
    /// This is called during app initialization to ensure a default workspace
    /// exists
    pub async fn get_or_create_default(&self) -> DatabaseResult<Workspace> {
        // Check if any workspace exists
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM workspaces")
            .fetch_one(self.db.pool())
            .await?;

        if count > 0 {
            // Return the first workspace as default
            let row: WorkspaceRow = sqlx::query_as(
                "SELECT id, name, description, created_at, updated_at
                 FROM workspaces
                 ORDER BY created_at ASC
                 LIMIT 1",
            )
            .fetch_one(self.db.pool())
            .await?;

            return Ok(Workspace::from(row));
        }

        // Create default workspace
        let workspace = Workspace::default_workspace();
        self.create(&workspace).await?;

        tracing::info!("Created default workspace: {}", workspace.id);

        Ok(workspace)
    }
}
