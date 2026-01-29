use std::sync::Arc;

use crate::{
    database::{Database, DatabaseResult, RepositoryGroupRow},
    entities::RepositoryGroup,
};

/// Service for managing repository groups
pub struct RepositoryGroupService {
    db: Arc<Database>,
}

impl RepositoryGroupService {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    /// Lists all repository groups
    pub async fn list(&self) -> DatabaseResult<Vec<RepositoryGroup>> {
        let rows: Vec<RepositoryGroupRow> = sqlx::query_as(
            "SELECT id, name, workspace_id, created_at, updated_at
             FROM repository_groups
             ORDER BY created_at DESC",
        )
        .fetch_all(self.db.pool())
        .await?;

        let mut groups = Vec::new();
        for row in rows {
            let repo_ids = self.get_repository_ids(&row.id).await?;
            groups.push(row.into_repository_group(repo_ids));
        }

        Ok(groups)
    }

    /// Lists repository groups by workspace
    pub async fn list_by_workspace(
        &self,
        workspace_id: &str,
    ) -> DatabaseResult<Vec<RepositoryGroup>> {
        let rows: Vec<RepositoryGroupRow> = sqlx::query_as(
            "SELECT id, name, workspace_id, created_at, updated_at
             FROM repository_groups
             WHERE workspace_id = ?
             ORDER BY created_at DESC",
        )
        .bind(workspace_id)
        .fetch_all(self.db.pool())
        .await?;

        let mut groups = Vec::new();
        for row in rows {
            let repo_ids = self.get_repository_ids(&row.id).await?;
            groups.push(row.into_repository_group(repo_ids));
        }

        Ok(groups)
    }

    /// Gets a repository group by ID
    pub async fn get(&self, id: &str) -> DatabaseResult<Option<RepositoryGroup>> {
        let row: Option<RepositoryGroupRow> = sqlx::query_as(
            "SELECT id, name, workspace_id, created_at, updated_at
             FROM repository_groups
             WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(self.db.pool())
        .await?;

        match row {
            Some(r) => {
                let repo_ids = self.get_repository_ids(&r.id).await?;
                Ok(Some(r.into_repository_group(repo_ids)))
            }
            None => Ok(None),
        }
    }

    /// Creates a new repository group
    pub async fn create(&self, group: &RepositoryGroup) -> DatabaseResult<()> {
        let row = RepositoryGroupRow::from(group);

        sqlx::query(
            "INSERT INTO repository_groups (id, name, workspace_id, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(&row.id)
        .bind(&row.name)
        .bind(&row.workspace_id)
        .bind(&row.created_at)
        .bind(&row.updated_at)
        .execute(self.db.pool())
        .await?;

        // Insert repository members
        for repo_id in &group.repository_ids {
            self.add_repository_internal(&group.id, repo_id).await?;
        }

        Ok(())
    }

    /// Updates a repository group
    pub async fn update(&self, group: &RepositoryGroup) -> DatabaseResult<()> {
        let now = chrono::Utc::now().to_rfc3339();

        sqlx::query(
            "UPDATE repository_groups SET name = ?, updated_at = ?
             WHERE id = ?",
        )
        .bind(&group.name)
        .bind(&now)
        .bind(&group.id)
        .execute(self.db.pool())
        .await?;

        Ok(())
    }

    /// Deletes a repository group
    pub async fn delete(&self, id: &str) -> DatabaseResult<()> {
        sqlx::query("DELETE FROM repository_groups WHERE id = ?")
            .bind(id)
            .execute(self.db.pool())
            .await?;
        Ok(())
    }

    /// Adds a repository to a group
    pub async fn add_repository(&self, group_id: &str, repository_id: &str) -> DatabaseResult<()> {
        self.add_repository_internal(group_id, repository_id)
            .await?;

        // Update the group's updated_at timestamp
        let now = chrono::Utc::now().to_rfc3339();
        sqlx::query("UPDATE repository_groups SET updated_at = ? WHERE id = ?")
            .bind(&now)
            .bind(group_id)
            .execute(self.db.pool())
            .await?;

        Ok(())
    }

    async fn add_repository_internal(
        &self,
        group_id: &str,
        repository_id: &str,
    ) -> DatabaseResult<()> {
        sqlx::query(
            "INSERT OR IGNORE INTO repository_group_members (repository_group_id, repository_id)
             VALUES (?, ?)",
        )
        .bind(group_id)
        .bind(repository_id)
        .execute(self.db.pool())
        .await?;

        Ok(())
    }

    /// Removes a repository from a group
    pub async fn remove_repository(
        &self,
        group_id: &str,
        repository_id: &str,
    ) -> DatabaseResult<()> {
        sqlx::query(
            "DELETE FROM repository_group_members
             WHERE repository_group_id = ? AND repository_id = ?",
        )
        .bind(group_id)
        .bind(repository_id)
        .execute(self.db.pool())
        .await?;

        // Update the group's updated_at timestamp
        let now = chrono::Utc::now().to_rfc3339();
        sqlx::query("UPDATE repository_groups SET updated_at = ? WHERE id = ?")
            .bind(&now)
            .bind(group_id)
            .execute(self.db.pool())
            .await?;

        Ok(())
    }

    /// Gets repository IDs in a group
    pub async fn get_repository_ids(&self, group_id: &str) -> DatabaseResult<Vec<String>> {
        let rows: Vec<(String,)> = sqlx::query_as(
            "SELECT repository_id FROM repository_group_members
             WHERE repository_group_id = ?",
        )
        .bind(group_id)
        .fetch_all(self.db.pool())
        .await?;

        Ok(rows.into_iter().map(|(id,)| id).collect())
    }

    /// Creates a single-repository group for a repository
    /// Returns the group ID
    pub async fn create_single_repo_group(
        &self,
        workspace_id: &str,
        repository_id: &str,
    ) -> DatabaseResult<String> {
        let group_id = uuid::Uuid::new_v4().to_string();
        let group = RepositoryGroup::single_repo(
            group_id.clone(),
            workspace_id.to_string(),
            repository_id.to_string(),
        );
        self.create(&group).await?;

        tracing::info!(
            "Created single-repo group {} for repository {}",
            group_id,
            repository_id
        );

        Ok(group_id)
    }

    /// Gets or creates a single-repository group for a repository
    /// If a single-repo group already exists for this repository, returns its
    /// ID Otherwise creates a new one
    pub async fn get_or_create_single_repo_group(
        &self,
        workspace_id: &str,
        repository_id: &str,
    ) -> DatabaseResult<String> {
        // Check if a single-repo group already exists for this repository
        let existing: Option<(String,)> = sqlx::query_as(
            "SELECT rg.id FROM repository_groups rg
             JOIN repository_group_members rgm ON rg.id = rgm.repository_group_id
             WHERE rg.name IS NULL
             AND rgm.repository_id = ?
             AND (SELECT COUNT(*) FROM repository_group_members WHERE repository_group_id = rg.id) \
             = 1
             LIMIT 1",
        )
        .bind(repository_id)
        .fetch_optional(self.db.pool())
        .await?;

        if let Some((group_id,)) = existing {
            return Ok(group_id);
        }

        // Create a new single-repo group
        self.create_single_repo_group(workspace_id, repository_id)
            .await
    }

    /// Finds a single-repo group for a given repository ID
    /// Returns None if no single-repo group exists
    pub async fn find_single_repo_group(
        &self,
        repository_id: &str,
    ) -> DatabaseResult<Option<String>> {
        let result: Option<(String,)> = sqlx::query_as(
            "SELECT rg.id FROM repository_groups rg
             JOIN repository_group_members rgm ON rg.id = rgm.repository_group_id
             WHERE rg.name IS NULL
             AND rgm.repository_id = ?
             AND (SELECT COUNT(*) FROM repository_group_members WHERE repository_group_id = rg.id) \
             = 1
             LIMIT 1",
        )
        .bind(repository_id)
        .fetch_optional(self.db.pool())
        .await?;

        Ok(result.map(|(id,)| id))
    }
}
