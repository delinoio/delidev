//! Application state

use std::sync::Arc;
use std::time::Duration;

use auth::{
    AuthStateStore, JwtAuth, MemoryAuthStateStore, OidcAuth, OidcConfig, PostgresAuthStateStore,
    SqliteAuthStateStore,
};
use task_store::{MemoryStore, TaskStore};
use tokio::sync::RwLock;

use crate::{
    config::ServerConfig,
    log_broadcaster::LogBroadcaster,
    worker_registry::WorkerRegistry,
};

/// Default timeout for OIDC metadata discovery (30 seconds)
const OIDC_DISCOVERY_TIMEOUT_SECS: u64 = 30;

/// Application state shared across handlers
#[derive(Clone)]
pub struct AppState {
    /// Task store
    pub store: Arc<dyn TaskStore>,

    /// JWT authentication (None in single-user mode)
    pub auth: Option<Arc<JwtAuth>>,

    /// OIDC authentication client (None if not configured)
    pub oidc: Option<Arc<OidcAuth>>,

    /// Auth state store for OIDC flow (database-backed for production)
    pub auth_state_store: Arc<dyn AuthStateStore>,

    /// Worker registry
    pub worker_registry: Arc<RwLock<WorkerRegistry>>,

    /// Log broadcaster for real-time streaming
    pub log_broadcaster: Arc<LogBroadcaster>,

    /// Server configuration
    pub config: Arc<ServerConfig>,
}

impl AppState {
    /// Create a new application state
    pub async fn new(config: ServerConfig) -> Result<Self, StateError> {
        // Initialize store based on mode
        let store: Arc<dyn TaskStore> = if config.single_user_mode {
            // Use in-memory store for now (SQLite can be added later)
            Arc::new(MemoryStore::new())
        } else {
            // In multi-user mode, would use PostgreSQL
            // For now, use memory store as a placeholder
            tracing::warn!("PostgreSQL store not yet implemented, using in-memory store");
            Arc::new(MemoryStore::new())
        };

        // Initialize JWT auth (skip in single-user mode)
        let auth = if config.single_user_mode {
            None
        } else {
            let jwt_auth = JwtAuth::new_hs256(config.jwt_secret.as_bytes())
                .with_issuer(&config.jwt_issuer);
            Some(Arc::new(jwt_auth))
        };

        // Initialize OIDC auth if configured
        let oidc = if let Some(ref oidc_config) = config.oidc {
            let oidc_cfg = OidcConfig::new(
                &oidc_config.issuer_url,
                &oidc_config.client_id,
                &oidc_config.client_secret,
                &oidc_config.redirect_url,
            )
            .with_scopes(oidc_config.scopes.clone());

            let mut oidc_auth = OidcAuth::new(oidc_cfg);

            // Try to discover OIDC provider metadata with timeout
            match discover_oidc_metadata_with_timeout(
                &oidc_auth,
                Duration::from_secs(OIDC_DISCOVERY_TIMEOUT_SECS),
            )
            .await
            {
                Ok(metadata) => {
                    tracing::info!(
                        issuer = %metadata.issuer,
                        "OIDC provider metadata discovered"
                    );
                    oidc_auth = oidc_auth.with_metadata(metadata);
                }
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        "Failed to discover OIDC metadata, OIDC auth may not work"
                    );
                }
            }

            Some(Arc::new(oidc_auth))
        } else {
            None
        };

        // Initialize auth state store based on configuration
        let auth_state_store: Arc<dyn AuthStateStore> = if config.single_user_mode {
            // Single-user mode: use SQLite if database path is configured
            match create_sqlite_auth_store(&config).await {
                Ok(store) => {
                    tracing::info!("Using SQLite for auth state storage");
                    Arc::new(store)
                }
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        "Failed to initialize SQLite auth store, falling back to in-memory"
                    );
                    Arc::new(MemoryAuthStateStore::new())
                }
            }
        } else {
            // Multi-user mode: use PostgreSQL
            match config.database_url.as_ref() {
                Some(database_url) => match create_postgres_auth_store(database_url).await {
                    Ok(store) => {
                        tracing::info!("Using PostgreSQL for auth state storage");
                        Arc::new(store)
                    }
                    Err(e) => {
                        tracing::error!(
                            error = %e,
                            "Failed to initialize PostgreSQL auth store"
                        );
                        return Err(StateError::Database(format!(
                            "Failed to initialize auth state database: {}",
                            e
                        )));
                    }
                },
                None => {
                    return Err(StateError::Database(
                        "DATABASE_URL is required for multi-user mode".to_string(),
                    ));
                }
            }
        };

        // Initialize worker registry
        let worker_registry = Arc::new(RwLock::new(WorkerRegistry::new(
            config.worker_heartbeat_timeout_secs,
        )));

        // Initialize log broadcaster
        let log_broadcaster = Arc::new(LogBroadcaster::new());

        Ok(Self {
            store,
            auth,
            oidc,
            auth_state_store,
            worker_registry,
            log_broadcaster,
            config: Arc::new(config),
        })
    }
}

/// Create a SQLite-backed auth state store
async fn create_sqlite_auth_store(config: &ServerConfig) -> Result<SqliteAuthStateStore, String> {
    // Ensure the database directory exists
    if let Some(parent) = config.database_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create database directory: {}", e))?;
    }

    let database_url = format!("sqlite:{}?mode=rwc", config.database_path.display());
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .map_err(|e| format!("Failed to connect to SQLite: {}", e))?;

    let store = SqliteAuthStateStore::new(pool);
    store
        .init()
        .await
        .map_err(|e| format!("Failed to initialize auth state table: {}", e))?;

    Ok(store)
}

/// Create a PostgreSQL-backed auth state store
async fn create_postgres_auth_store(database_url: &str) -> Result<PostgresAuthStateStore, String> {
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(10)
        .connect(database_url)
        .await
        .map_err(|e| format!("Failed to connect to PostgreSQL: {}", e))?;

    let store = PostgresAuthStateStore::new(pool);
    store
        .init()
        .await
        .map_err(|e| format!("Failed to initialize auth state table: {}", e))?;

    Ok(store)
}

/// Discover OIDC provider metadata with a timeout
async fn discover_oidc_metadata_with_timeout(
    oidc: &OidcAuth,
    timeout: Duration,
) -> Result<auth::OidcProviderMetadata, Box<dyn std::error::Error + Send + Sync>> {
    tokio::time::timeout(timeout, discover_oidc_metadata(oidc))
        .await
        .map_err(|_| "OIDC metadata discovery timed out".to_string())?
}

/// Discover OIDC provider metadata
async fn discover_oidc_metadata(
    oidc: &OidcAuth,
) -> Result<auth::OidcProviderMetadata, Box<dyn std::error::Error + Send + Sync>> {
    let discovery_url = oidc.discovery_url();
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .connect_timeout(Duration::from_secs(10))
        .build()?;

    let response = client.get(&discovery_url).send().await?;

    if !response.status().is_success() {
        return Err(format!(
            "OIDC discovery failed with status: {}",
            response.status()
        )
        .into());
    }

    let metadata: auth::OidcProviderMetadata = response.json().await?;
    Ok(metadata)
}

/// State initialization errors
#[derive(Debug, thiserror::Error)]
pub enum StateError {
    #[error("Failed to initialize database: {0}")]
    Database(String),

    #[allow(dead_code)]
    #[error("Failed to initialize auth: {0}")]
    Auth(String),
}
