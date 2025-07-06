use anyhow::Result;
use clap::Parser;
use tracing::{info, Level};
use tracing_subscriber;

mod cli;
mod core;
mod config;
mod error;

use cli::Cli;
use core::Engine;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();

    info!("Starting Codesworth v{}", env!("CARGO_PKG_VERSION"));

    // Create the core engine with configuration
    let engine = Engine::new(cli.config.as_deref()).await?;

    // Execute the requested command
    cli.execute(engine).await
}