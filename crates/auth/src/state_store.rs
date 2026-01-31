//! Authorization state storage
//!
//! This module provides storage backends for OIDC authorization state.
//! Supports both in-memory (for testing) and database (for production) storage.

use std::collections::HashMap;
use std::sync::RwLock;

use async_trait::async_trait;

use crate::{AuthError, AuthResult, AuthorizationState};

/// Trait for authorization state storage
#[async_trait]
pub trait AuthStateStore: Send + Sync {
    /// Store an authorization state
    async fn store(&self, state: &AuthorizationState) -> AuthResult<()>;

    /// Retrieve and remove an authorization state by state token
    ///
    /// This is an atomic operation - the state is removed upon retrieval
    /// to prevent replay attacks.
    async fn take(&self, state_token: &str) -> AuthResult<Option<AuthorizationState>>;

    /// Remove expired states (cleanup task)
    ///
    /// Returns the number of states removed.
    async fn cleanup_expired(&self, max_age_secs: i64) -> AuthResult<usize>;
}

/// In-memory authorization state store (for testing and single-process mode)
#[derive(Debug, Default)]
pub struct MemoryAuthStateStore {
    states: RwLock<HashMap<String, AuthorizationState>>,
}

impl MemoryAuthStateStore {
    /// Create a new in-memory store
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl AuthStateStore for MemoryAuthStateStore {
    async fn store(&self, state: &AuthorizationState) -> AuthResult<()> {
        let mut states = self
            .states
            .write()
            .map_err(|e| AuthError::Oidc(format!("Lock poisoned: {}", e)))?;
        states.insert(state.state.clone(), state.clone());
        Ok(())
    }

    async fn take(&self, state_token: &str) -> AuthResult<Option<AuthorizationState>> {
        let mut states = self
            .states
            .write()
            .map_err(|e| AuthError::Oidc(format!("Lock poisoned: {}", e)))?;
        Ok(states.remove(state_token))
    }

    async fn cleanup_expired(&self, max_age_secs: i64) -> AuthResult<usize> {
        let mut states = self
            .states
            .write()
            .map_err(|e| AuthError::Oidc(format!("Lock poisoned: {}", e)))?;
        let before_count = states.len();
        states.retain(|_, state| !state.is_expired(max_age_secs));
        Ok(before_count - states.len())
    }
}

#[cfg(feature = "sqlx")]
pub use sqlx_store::*;

#[cfg(feature = "sqlx")]
mod sqlx_store {
    use super::*;
    use sqlx::{Pool, Postgres, Sqlite};

    /// PostgreSQL authorization state store (for multi-user production deployments)
    #[derive(Clone)]
    pub struct PostgresAuthStateStore {
        pool: Pool<Postgres>,
    }

    impl PostgresAuthStateStore {
        /// Create a new PostgreSQL store
        pub fn new(pool: Pool<Postgres>) -> Self {
            Self { pool }
        }

        /// Initialize the database table
        pub async fn init(&self) -> AuthResult<()> {
            sqlx::query(
                r#"
                CREATE TABLE IF NOT EXISTS auth_states (
                    state_token TEXT PRIMARY KEY,
                    nonce TEXT NOT NULL,
                    code_verifier TEXT,
                    created_at BIGINT NOT NULL,
                    redirect_uri TEXT
                )
                "#,
            )
            .execute(&self.pool)
            .await
            .map_err(|e| AuthError::Oidc(format!("Failed to create auth_states table: {}", e)))?;

            // Create index for cleanup queries
            sqlx::query(
                r#"
                CREATE INDEX IF NOT EXISTS idx_auth_states_created_at
                ON auth_states (created_at)
                "#,
            )
            .execute(&self.pool)
            .await
            .map_err(|e| AuthError::Oidc(format!("Failed to create index: {}", e)))?;

            Ok(())
        }
    }

    #[async_trait]
    impl AuthStateStore for PostgresAuthStateStore {
        async fn store(&self, state: &AuthorizationState) -> AuthResult<()> {
            sqlx::query(
                r#"
                INSERT INTO auth_states (state_token, nonce, code_verifier, created_at, redirect_uri)
                VALUES ($1, $2, $3, $4, $5)
                ON CONFLICT (state_token) DO UPDATE SET
                    nonce = EXCLUDED.nonce,
                    code_verifier = EXCLUDED.code_verifier,
                    created_at = EXCLUDED.created_at,
                    redirect_uri = EXCLUDED.redirect_uri
                "#,
            )
            .bind(&state.state)
            .bind(&state.nonce)
            .bind(&state.code_verifier)
            .bind(state.created_at)
            .bind(&state.redirect_uri)
            .execute(&self.pool)
            .await
            .map_err(|e| AuthError::Oidc(format!("Failed to store auth state: {}", e)))?;
            Ok(())
        }

        async fn take(&self, state_token: &str) -> AuthResult<Option<AuthorizationState>> {
            let row: Option<(String, String, Option<String>, i64, Option<String>)> = sqlx::query_as(
                r#"
                DELETE FROM auth_states
                WHERE state_token = $1
                RETURNING state_token, nonce, code_verifier, created_at, redirect_uri
                "#,
            )
            .bind(state_token)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| AuthError::Oidc(format!("Failed to retrieve auth state: {}", e)))?;

            Ok(row.map(|(state, nonce, code_verifier, created_at, redirect_uri)| {
                AuthorizationState {
                    state,
                    nonce,
                    code_verifier,
                    created_at,
                    redirect_uri,
                }
            }))
        }

        async fn cleanup_expired(&self, max_age_secs: i64) -> AuthResult<usize> {
            let cutoff = chrono::Utc::now().timestamp() - max_age_secs;
            let result = sqlx::query(
                r#"
                DELETE FROM auth_states
                WHERE created_at < $1
                "#,
            )
            .bind(cutoff)
            .execute(&self.pool)
            .await
            .map_err(|e| AuthError::Oidc(format!("Failed to cleanup expired states: {}", e)))?;

            Ok(result.rows_affected() as usize)
        }
    }

    /// SQLite authorization state store (for single-user mode)
    #[derive(Clone)]
    pub struct SqliteAuthStateStore {
        pool: Pool<Sqlite>,
    }

    impl SqliteAuthStateStore {
        /// Create a new SQLite store
        pub fn new(pool: Pool<Sqlite>) -> Self {
            Self { pool }
        }

        /// Initialize the database table
        pub async fn init(&self) -> AuthResult<()> {
            sqlx::query(
                r#"
                CREATE TABLE IF NOT EXISTS auth_states (
                    state_token TEXT PRIMARY KEY,
                    nonce TEXT NOT NULL,
                    code_verifier TEXT,
                    created_at INTEGER NOT NULL,
                    redirect_uri TEXT
                )
                "#,
            )
            .execute(&self.pool)
            .await
            .map_err(|e| AuthError::Oidc(format!("Failed to create auth_states table: {}", e)))?;

            // Create index for cleanup queries
            sqlx::query(
                r#"
                CREATE INDEX IF NOT EXISTS idx_auth_states_created_at
                ON auth_states (created_at)
                "#,
            )
            .execute(&self.pool)
            .await
            .map_err(|e| AuthError::Oidc(format!("Failed to create index: {}", e)))?;

            Ok(())
        }
    }

    #[async_trait]
    impl AuthStateStore for SqliteAuthStateStore {
        async fn store(&self, state: &AuthorizationState) -> AuthResult<()> {
            sqlx::query(
                r#"
                INSERT OR REPLACE INTO auth_states (state_token, nonce, code_verifier, created_at, redirect_uri)
                VALUES (?, ?, ?, ?, ?)
                "#,
            )
            .bind(&state.state)
            .bind(&state.nonce)
            .bind(&state.code_verifier)
            .bind(state.created_at)
            .bind(&state.redirect_uri)
            .execute(&self.pool)
            .await
            .map_err(|e| AuthError::Oidc(format!("Failed to store auth state: {}", e)))?;
            Ok(())
        }

        async fn take(&self, state_token: &str) -> AuthResult<Option<AuthorizationState>> {
            // SQLite doesn't support RETURNING in DELETE, so we need two queries in a transaction
            let mut tx = self
                .pool
                .begin()
                .await
                .map_err(|e| AuthError::Oidc(format!("Failed to begin transaction: {}", e)))?;

            let row: Option<(String, String, Option<String>, i64, Option<String>)> = sqlx::query_as(
                r#"
                SELECT state_token, nonce, code_verifier, created_at, redirect_uri
                FROM auth_states
                WHERE state_token = ?
                "#,
            )
            .bind(state_token)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|e| AuthError::Oidc(format!("Failed to retrieve auth state: {}", e)))?;

            if row.is_some() {
                sqlx::query(
                    r#"
                    DELETE FROM auth_states
                    WHERE state_token = ?
                    "#,
                )
                .bind(state_token)
                .execute(&mut *tx)
                .await
                .map_err(|e| AuthError::Oidc(format!("Failed to delete auth state: {}", e)))?;
            }

            tx.commit()
                .await
                .map_err(|e| AuthError::Oidc(format!("Failed to commit transaction: {}", e)))?;

            Ok(row.map(|(state, nonce, code_verifier, created_at, redirect_uri)| {
                AuthorizationState {
                    state,
                    nonce,
                    code_verifier,
                    created_at,
                    redirect_uri,
                }
            }))
        }

        async fn cleanup_expired(&self, max_age_secs: i64) -> AuthResult<usize> {
            let cutoff = chrono::Utc::now().timestamp() - max_age_secs;
            let result = sqlx::query(
                r#"
                DELETE FROM auth_states
                WHERE created_at < ?
                "#,
            )
            .bind(cutoff)
            .execute(&self.pool)
            .await
            .map_err(|e| AuthError::Oidc(format!("Failed to cleanup expired states: {}", e)))?;

            Ok(result.rows_affected() as usize)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_memory_store_basic_operations() {
        let store = MemoryAuthStateStore::new();
        let state = AuthorizationState::new();
        let state_token = state.state.clone();

        // Store the state
        store.store(&state).await.unwrap();

        // Take the state (should succeed)
        let retrieved = store.take(&state_token).await.unwrap();
        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.state, state_token);

        // Take again (should be None - already removed)
        let retrieved = store.take(&state_token).await.unwrap();
        assert!(retrieved.is_none());
    }

    #[tokio::test]
    async fn test_memory_store_cleanup() {
        let store = MemoryAuthStateStore::new();

        // Create an old state
        let mut old_state = AuthorizationState::new();
        old_state.created_at = chrono::Utc::now().timestamp() - 1000;

        // Create a fresh state
        let fresh_state = AuthorizationState::new();

        store.store(&old_state).await.unwrap();
        store.store(&fresh_state).await.unwrap();

        // Cleanup states older than 600 seconds
        let removed = store.cleanup_expired(600).await.unwrap();
        assert_eq!(removed, 1);

        // Old state should be gone
        assert!(store.take(&old_state.state).await.unwrap().is_none());

        // Fresh state should still be there
        assert!(store.take(&fresh_state.state).await.unwrap().is_some());
    }
}
