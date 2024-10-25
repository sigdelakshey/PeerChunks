use clap::{Parser, Subcommand};
use env_logger::Env;
use log::{error, info};
use crate::config::Config;
use crate::peer::discovery::start_peer_discovery;
use crate::ui::cli::run_cli;
use std::error::Error;
use tokio::sync::mpsc;
use std::fs;
use std::path::Path;

mod config;
mod peer;
mod ui;

#[derive(Parser)]
#[command(name = "PeerChunks")]
#[command(about = "A peer-to-peer distributed file sharing system", long_about = None)]
struct Cli {
    #[arg(short, long, default_value = "config.yaml")]
    config: String,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    Upload {
        file_path: String,
    },
    Download {
        file_id: String,
        destination: String,
    },
    Search {
        query: String,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let cli = Cli::parse();

    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    info!("Starting PeerChunks...");

    let config = Config::load(&cli.config).unwrap_or_else(|err| {
        error!("Failed to load configuration from {}: {}", &cli.config, err);
        std::process::exit(1);
    });
    info!("Configuration loaded successfully.");

    if let Some(command) = cli.command {
        match command {
            Commands::Upload { file_path } => {
                info!("Uploading file: {}", file_path);
            }
            Commands::Download { file_id, destination } => {
                info!("Downloading file ID: {} to {}", file_id, destination);
            }
            Commands::Search { query } => {
                info!("Searching for: {}", query);
            }
        }
    } else {
        if !Path::new(&config.storage_path).exists() {
            fs::create_dir_all(&config.storage_path)?;
            info!("Created storage directory at {}", config.storage_path);
        }
        
        let (tx, _rx) = mpsc::channel(100);
        let peer_discovery_handle = tokio::spawn(start_peer_discovery(config.clone(), tx.clone()));
        let cli_handle = tokio::spawn(run_cli(tx.clone()));
        let _ = tokio::join!(peer_discovery_handle, cli_handle);
        
        info!("PeerChunks has stopped.");
    }

    Ok(())
}
