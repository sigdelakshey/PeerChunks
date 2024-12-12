use clap::{Parser, Subcommand};
use env_logger::Env;
use log::{error, info};
use crate::config::Config;
use crate::peer::discovery::{start_peer_discovery, Peer};
use crate::ui::cli::run_cli;
use crate::indexing::dht::DHT;
use std::error::Error;
use tokio::sync::mpsc;
use std::fs;
use std::path::Path;

mod config;
mod peer;
mod file_manager;
mod indexing;
mod ui;

#[derive(Parser)]
#[command(name = "ShareSphere")]
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
    info!("Starting ShareSphere...");

    let config = Config::load(&cli.config).unwrap_or_else(|err| {
        error!("Failed to load configuration: {}", err);
        std::process::exit(1);
    });
    info!("Configuration loaded successfully.");

    if !Path::new(&config.storage_path).exists() {
        fs::create_dir_all(&config.storage_path)?;
        info!("Created storage directory at {}", config.storage_path);
    }

    let dht = DHT::new();
    let local_peer = Peer {
        address: format!("127.0.0.1:{}", config.peer_port),
    };

    let (tx, rx) = mpsc::channel(100);

    let encryption_key = config.encryption_key.clone();
    let peers = Vec::new();

    let peer_discovery_handle = tokio::spawn(start_peer_discovery(config.clone(), tx.clone(), dht.clone(), local_peer.clone()));
    let cli_handle = tokio::spawn(run_cli(rx, dht, config.storage_path.clone(), peers, encryption_key));

    let _ = tokio::join!(peer_discovery_handle, cli_handle);

    info!("ShareSphere has stopped.");
    Ok(())
}
