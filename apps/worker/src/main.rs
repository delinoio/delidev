//! DeliDev Worker Server
//!
//! The worker server handles:
//! - AI agent execution in Docker containers
//! - Task processing and result reporting
//! - Log streaming to the main server

use std::sync::Arc;

use tokio::signal;
use tokio::sync::watch;
use tracing::{error, info, Level};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod config;
mod executor;
mod heartbeat;
mod server_client;

use config::WorkerConfig;
use executor::TaskExecutor;
use heartbeat::{get_system_capacity, HeartbeatService};
use server_client::MainServerClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load configuration
    let config = WorkerConfig::load().expect("Failed to load configuration");
    let config = Arc::new(config);

    // Initialize tracing
    let log_level = match config.log_level.as_str() {
        "trace" => Level::TRACE,
        "debug" => Level::DEBUG,
        "info" => Level::INFO,
        "warn" => Level::WARN,
        "error" => Level::ERROR,
        _ => Level::INFO,
    };

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                format!("delidev_worker={},tower_http=debug", log_level).into()
            }),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!(
        version = env!("CARGO_PKG_VERSION"),
        worker_id = %config.worker_id(),
        server_url = %config.main_server_url,
        "Starting DeliDev Worker"
    );

    // Create shutdown signal
    let (shutdown_tx, shutdown_rx) = watch::channel(false);

    // Create server client
    let client = Arc::new(MainServerClient::new(&config.main_server_url));

    // Wait for server to be available
    info!("Connecting to main server...");
    let mut retries = 0;
    loop {
        match client.health_check().await {
            Ok(_) => {
                info!("Connected to main server");
                break;
            }
            Err(e) => {
                retries += 1;
                if retries > 30 {
                    error!("Failed to connect to main server after 30 attempts");
                    return Err(e.into());
                }
                info!(
                    attempt = retries,
                    "Main server not available, retrying in 2 seconds..."
                );
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            }
        }
    }

    // Register with main server
    let capacity = get_system_capacity();
    let capacity = rpc_protocol::WorkerCapacity {
        max_concurrent_tasks: config.max_concurrent_tasks,
        ..capacity
    };

    match client
        .register_worker(config.worker_id(), &config.bind_address, capacity)
        .await
    {
        Ok(response) => {
            if response.registered {
                info!(worker_id = %response.worker_id, "Registered with main server");
            } else {
                error!("Failed to register with main server");
                return Err("Registration failed".into());
            }
        }
        Err(e) => {
            error!(error = %e, "Failed to register with main server");
            return Err(e.into());
        }
    }

    // Create task executor
    let executor = Arc::new(TaskExecutor::new(config.clone(), client.clone()));

    // Start heartbeat service
    let heartbeat_service = HeartbeatService::new(
        config.clone(),
        client.clone(),
        executor.clone(),
        shutdown_rx.clone(),
    );
    let heartbeat_handle = tokio::spawn(heartbeat_service.run());

    // Wait for shutdown signal
    info!("Worker ready and waiting for tasks");
    shutdown_signal().await;

    // Signal shutdown
    info!("Shutting down worker...");
    let _ = shutdown_tx.send(true);

    // Wait for heartbeat to stop
    let _ = heartbeat_handle.await;

    info!("Worker shutdown complete");
    Ok(())
}

/// Graceful shutdown signal handler
async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            info!("Received Ctrl+C, shutting down");
        }
        _ = terminate => {
            info!("Received terminate signal, shutting down");
        }
    }
}
