use std::{path::Path, sync::Arc};

use crate::{
    database::{Database, DatabaseResult, RepositoryRow},
    entities::{Repository, VCSProviderType},
};

/// Service for managing repositories
pub struct RepositoryService {
    db: Arc<Database>,
}

impl RepositoryService {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    /// Lists all registered repositories
    pub async fn list(&self) -> DatabaseResult<Vec<Repository>> {
        let rows: Vec<RepositoryRow> = sqlx::query_as(
            "SELECT id, vcs_type, vcs_provider_type, remote_url, name, local_path, \
             default_branch, created_at
             FROM repositories
             ORDER BY name",
        )
        .fetch_all(self.db.pool())
        .await?;

        Ok(rows.into_iter().map(Repository::from).collect())
    }

    /// Gets a repository by ID
    pub async fn get(&self, id: &str) -> DatabaseResult<Option<Repository>> {
        let row: Option<RepositoryRow> = sqlx::query_as(
            "SELECT id, vcs_type, vcs_provider_type, remote_url, name, local_path, \
             default_branch, created_at
             FROM repositories
             WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(self.db.pool())
        .await?;

        Ok(row.map(Repository::from))
    }

    /// Creates a new repository
    pub async fn create(&self, repo: &Repository) -> DatabaseResult<()> {
        let row = RepositoryRow::from(repo);

        sqlx::query(
            "INSERT INTO repositories (id, vcs_type, vcs_provider_type, remote_url, name, \
             local_path, default_branch, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&row.id)
        .bind(&row.vcs_type)
        .bind(&row.vcs_provider_type)
        .bind(&row.remote_url)
        .bind(&row.name)
        .bind(&row.local_path)
        .bind(&row.default_branch)
        .bind(&row.created_at)
        .execute(self.db.pool())
        .await?;

        Ok(())
    }

    /// Deletes a repository
    pub async fn delete(&self, id: &str) -> DatabaseResult<()> {
        sqlx::query("DELETE FROM repositories WHERE id = ?")
            .bind(id)
            .execute(self.db.pool())
            .await?;
        Ok(())
    }

    /// Updates a repository
    pub async fn update(&self, repo: &Repository) -> DatabaseResult<()> {
        let row = RepositoryRow::from(repo);

        sqlx::query(
            "UPDATE repositories
             SET vcs_type = ?, vcs_provider_type = ?, remote_url = ?, name = ?, local_path = ?, \
             default_branch = ?
             WHERE id = ?",
        )
        .bind(&row.vcs_type)
        .bind(&row.vcs_provider_type)
        .bind(&row.remote_url)
        .bind(&row.name)
        .bind(&row.local_path)
        .bind(&row.default_branch)
        .bind(&row.id)
        .execute(self.db.pool())
        .await?;

        Ok(())
    }

    /// Checks if a local path is already registered
    pub async fn exists_by_path(&self, path: &str) -> DatabaseResult<bool> {
        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM repositories WHERE local_path = ?")
                .bind(path)
                .fetch_one(self.db.pool())
                .await?;

        Ok(count > 0)
    }

    /// Checks if a remote URL is already registered
    pub async fn exists_by_remote_url(&self, remote_url: &str) -> DatabaseResult<bool> {
        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM repositories WHERE remote_url = ?")
                .bind(remote_url)
                .fetch_one(self.db.pool())
                .await?;

        Ok(count > 0)
    }

    /// Detects repository info from local path
    pub fn detect_from_path(path: &Path) -> Option<(String, String, VCSProviderType)> {
        let repo = git2::Repository::open(path).ok()?;
        let remote = repo.find_remote("origin").ok()?;
        let url = remote.url()?;

        let provider = Repository::detect_provider_from_url(url)?;

        // Extract repo name from URL
        let name = url
            .split('/')
            .next_back()
            .map(|s| s.trim_end_matches(".git"))
            .unwrap_or("unknown")
            .to_string();

        Some((url.to_string(), name, provider))
    }

    /// Gets the default branch for a repository (main, master, or from
    /// origin/HEAD)
    pub fn detect_default_branch(path: &Path) -> String {
        if let Ok(repo) = git2::Repository::open(path) {
            // Try common default branch names first
            for branch_name in &["main", "master"] {
                if repo
                    .find_branch(branch_name, git2::BranchType::Local)
                    .is_ok()
                {
                    return branch_name.to_string();
                }
            }

            // Try to get from origin/HEAD
            if let Ok(reference) = repo.find_reference("refs/remotes/origin/HEAD") {
                if let Some(target) = reference.symbolic_target() {
                    if let Some(branch) = target.strip_prefix("refs/remotes/origin/") {
                        return branch.to_string();
                    }
                }
            }
        }
        "main".to_string()
    }
}
