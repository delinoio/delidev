//! Application state.

use std::collections::HashMap;
use std::sync::Arc;

use auth::JwtManager;
use task_store::TaskStore;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::config::Config;
use crate::services::worker_registry::WorkerRegistry;

/// Secrets cache for tasks (in-memory storage).
#[derive(Debug, Default)]
pub struct SecretsCache {
    /// Map of task ID to secrets.
    secrets: HashMap<Uuid, Vec<rpc_protocol::Secret>>,
}

impl SecretsCache {
    /// Creates a new secrets cache.
    pub fn new() -> Self {
        Self::default()
    }

    /// Stores secrets for a task.
    pub fn store(&mut self, task_id: Uuid, secrets: Vec<rpc_protocol::Secret>) {
        self.secrets.insert(task_id, secrets);
    }

    /// Gets secrets for a task.
    pub fn get(&self, task_id: &Uuid) -> Option<&Vec<rpc_protocol::Secret>> {
        self.secrets.get(task_id)
    }

    /// Clears secrets for a task.
    pub fn clear(&mut self, task_id: &Uuid) {
        self.secrets.remove(task_id);
    }
}

/// Shared application state.
pub struct AppState<S: TaskStore> {
    /// Server configuration.
    pub config: Config,
    /// Task store.
    pub store: S,
    /// JWT manager (optional, only used in multi-user mode).
    pub jwt_manager: Option<JwtManager>,
    /// Worker registry.
    pub worker_registry: RwLock<WorkerRegistry>,
    /// Secrets cache.
    pub secrets_cache: RwLock<SecretsCache>,
}

impl<S: TaskStore> AppState<S> {
    /// Creates new application state.
    pub fn new(config: Config, store: S, jwt_manager: Option<JwtManager>) -> Self {
        Self {
            config,
            store,
            jwt_manager,
            worker_registry: RwLock::new(WorkerRegistry::new()),
            secrets_cache: RwLock::new(SecretsCache::new()),
        }
    }

    /// Returns true if authentication is enabled.
    pub fn auth_enabled(&self) -> bool {
        self.config.auth_enabled()
    }
}

/// Type alias for shared state.
pub type SharedState<S> = Arc<AppState<S>>;

/// Creates shared state from config and store.
pub fn create_shared_state<S: TaskStore>(
    config: Config,
    store: S,
    jwt_manager: Option<JwtManager>,
) -> SharedState<S> {
    Arc::new(AppState::new(config, store, jwt_manager))
}
