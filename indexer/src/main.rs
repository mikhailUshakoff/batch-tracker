use anyhow::Error;
use batch_indexer::BatchIndexer;
use config::Config;
mod batch_indexer;
mod config;
mod db;
mod taiko_inbox_binding;

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env()) // reads RUST_LOG
        .init();

    tracing::info!("App started");

    let config = Config::new();
    let mut batch_tracker = BatchIndexer::new(config).await?;
    batch_tracker.run_indexing_loop().await;

    Ok(())
}
