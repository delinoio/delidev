//! Application state

#![allow(dead_code)]

use std::sync::Arc;

use auth::{JwtAuth, OidcAuth, OidcConfig};
use task_store::{MemoryStore, TaskStore};
use tokio::sync::RwLock;

use crate::{
    auth_routes::AuthStateStore,
    config::ServerConfig,
    log_broadcaster::LogBroadcaster,
    worker_registry::WorkerRegistry,
};

/// Application state shared across handlers
#[derive(Clone)]
pub struct AppState {
    /// Task store
    pub store: Arc<dyn TaskStore>,

    /// JWT authentication (None in single-user mode)
    pub auth: Option<Arc<JwtAuth>>,

    /// OIDC authentication client (None if not configured)
    pub oidc: Option<Arc<OidcAuth>>,

    /// Auth state store for OIDC flow
    pub auth_state_store: AuthStateStore,

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

            // Try to discover OIDC provider metadata
            match discover_oidc_metadata(&oidc_auth).await {
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

        // Initialize auth state store
        let auth_state_store = crate::auth_routes::create_auth_state_store();

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

/// Discover OIDC provider metadata
async fn discover_oidc_metadata(
    oidc: &OidcAuth,
) -> Result<auth::OidcProviderMetadata, Box<dyn std::error::Error + Send + Sync>> {
    let discovery_url = oidc.discovery_url();
    let client = reqwest::Client::new();

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

    #[error("Failed to initialize auth: {0}")]
    Auth(String),
}
