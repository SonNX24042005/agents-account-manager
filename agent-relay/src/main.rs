mod cli;
mod config;
mod device;
mod models;
mod oauth;
mod proxy;
mod storage;

use cli::Cli;
use config::Config;
use proxy::{Server, TokenManager};
use storage::AccountStore;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main(worker_threads = 2)]
async fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if let Some(res) = Cli::handle_args(&args) {
        return res;
    }
    // 1. Initialize colorful terminal logging subscriber
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info,agent_relay=debug,antigravity_relay=debug".into()),
        ))
        .with(tracing_subscriber::fmt::layer().with_thread_ids(false))
        .init();

    tracing::info!("=====================================================");
    tracing::info!("   Agent Relay Daemon Engine v1.0.3");
    tracing::info!("=====================================================");

    // 2. Load configuration & create data directories
    let config = Config::default();
    config.ensure_directories()?;
    tracing::info!("[Config] Data directory: {:?}", config.data_dir);

    // 3. Initialize Account Storage & Token Pool
    let store = AccountStore::new(config.accounts_dir());
    let token_manager = TokenManager::new(store, config.data_dir.clone())?;

    // 4. Start Axum Web Server & Account Manager
    Server::run(config, token_manager).await?;

    Ok(())
}
