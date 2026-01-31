//! DeliDev Main Server binary.

use std::net::SocketAddr;

use main_server::{config::Config, create_app, create_state, init_tracing};
use task_store::MemoryTaskStore;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load environment variables from .env if present
    dotenvy::dotenv().ok();

    // Load configuration
    let config = Config::from_env()?;

    // Initialize tracing
    init_tracing(&config.log_level);

    tracing::info!(
        single_user_mode = config.single_user_mode,
        "Starting DeliDev Main Server"
    );

    // Create task store
    // TODO: Use SQLite or PostgreSQL based on config
    let store = MemoryTaskStore::new();

    // Create application state
    let state = create_state(config.clone(), store);

    // Create application router
    let app = create_app(state);

    // Parse server address
    let addr: SocketAddr = config.server_addr().parse()?;

    tracing::info!(addr = %addr, "Server listening");

    // Start server
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
