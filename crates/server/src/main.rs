// Main entry point for the collaborative editor server

mod database;
mod document;
mod features;
mod file_store;
mod messages;
mod room;
mod server;
mod secure_channel;

use anyhow::Result;
use std::net::SocketAddr;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};


#[cfg(not(feature = "prod"))]
const IP: &str = "127.0.0.1:9001";

#[cfg(feature = "prod")]
const IP: &str = "34.135.102.212:9001";

#[tokio::main]
async fn main() -> Result<()> {
    println!("ip: {}", IP);

    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,server=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Starting collaborative editor server...");

    // Configuration
    let database_url =
        std::env::var("DATABASE_URL").unwrap_or_else(|_| "mysql:bearshare.db".to_string());
    let file_store_path =
        std::env::var("FILE_STORE_PATH").unwrap_or_else(|_| "./file_store".to_string());
    let addr: SocketAddr = std::env::var("BIND_ADDR")
        .unwrap_or_else(|_| "127.0.0.1:9001".to_string())
        .parse()?;

    // Initialize database
    tracing::info!("Connecting to database: {}", database_url);
    let db = database::Database::new(&database_url).await?;

    // Initialize file store
    tracing::info!("Initializing file store: {}", file_store_path);
    let file_store = file_store::FileStore::new(&file_store_path).await?;

    // Create server state
    let state = server::ServerState::new(db, file_store).await;

    // Start server
    server::create_server(state, addr).await?;

    Ok(())
}
