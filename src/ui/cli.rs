// src/ui/cli.rs

use clap::{Parser, Subcommand};
use log::{info, error};
use std::error::Error;
use crate::file_manager::chunker::split_file_into_chunks;
use crate::file_manager::storage::{initialize_storage, save_chunk, get_chunk};
use crate::file_manager::replication::replicate_chunks;
use crate::indexing::search::search_file;
use crate::indexing::dht::DHT;
use crate::peer::discovery::Peer;
use tokio::sync::mpsc::Receiver;
use uuid::Uuid;
use std::fs::OpenOptions;
use std::io::Write;
use tokio::runtime::Runtime;

#[derive(Parser)]
#[command(name = "ShareSphere CLI")]
#[command(about = "Interact with the ShareSphere P2P network", long_about = None)]
pub struct Cli {
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
    Exit,
}

pub async fn run_cli(
    mut rx: Receiver<String>,
    dht: DHT,
    storage_root: String,
    peers: Vec<Peer>,
    encryption_key: String,
) {
    let rt = Runtime::new().unwrap(); 
    loop {
        println!("Enter command (upload/download/search/exit): ");
        let cmd = match rx.recv().await {
            Some(line) => line.trim().to_string(),
            None => {
                println!("No more commands. Exiting CLI.");
                break;
            }
        };

        let args = cmd.split_whitespace().collect::<Vec<_>>();
        if args.is_empty() {
            continue;
        }

        match args[0].to_lowercase().as_str() {
            "upload" => {
                if args.len() < 2 {
                    error!("Usage: upload <file_path>");
                    continue;
                }
                let file_path = args[1];
                match rt.block_on(upload_file(file_path, &storage_root, &peers, &dht)) {
                    Ok(file_id) => info!("Uploaded file {} with file_id {}", file_path, file_id),
                    Err(e) => error!("Upload failed: {}", e),
                }
            }
            "download" => {
                if args.len() < 3 {
                    error!("Usage: download <file_id> <destination>");
                    continue;
                }
                let file_id = args[1];
                let destination = args[2];
                match rt.block_on(download_file(file_id, destination, &storage_root, &dht, &peers)) {
                    Ok(_) => info!("Downloaded file {} to {}", file_id, destination),
                    Err(e) => error!("Download failed: {}", e),
                }
            }
            "search" => {
                if args.len() < 2 {
                    error!("Usage: search <file_id>");
                    continue;
                }
                let query = args[1];
                let results = search_file(&dht, query);
                if results.is_empty() {
                    println!("No peers found for file_id {}", query);
                } else {
                    println!("Peers storing file {}:", query);
                    for addr in results {
                        println!("- {}", addr);
                    }
                }
            }
            "exit" => {
                println!("Exiting ShareSphere CLI.");
                break;
            }
            _ => {
                error!("Unknown command. Available commands: upload, download, search, exit");
            }
        }
    }
}

async fn upload_file(
    file_path: &str,
    storage_root: &str,
    peers: &[Peer],
    dht: &DHT,
) -> Result<Uuid, Box<dyn Error + Send + Sync>> {
    let chunk_size = 1024; // Example chunk size
    let (file_id, chunks) = split_file_into_chunks(file_path, chunk_size)?;

    let storage_dir = initialize_storage(storage_root, file_id)?;
    for (metadata, data) in &chunks {
        save_chunk(&storage_dir, metadata, data)?;
    }

    let local_peer = Peer { address: "127.0.0.1:8080".to_string() }; // Assume local peer address known
    dht.register_file_location(file_id, local_peer.clone());

    replicate_chunks(peers, storage_root, &file_id).await?;

    Ok(file_id)
}

async fn download_file(
    file_id_str: &str,
    destination: &str,
    storage_root: &str,
    dht: &DHT,
    peers: &[Peer],
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let file_id = Uuid::parse_str(file_id_str)?;
    let _peer_addresses = dht.get_file_locations(&file_id).ok_or("File not found in DHT")?;

    let storage_dir = std::path::Path::new(storage_root).join(file_id.to_string());
    let chunk_indices = crate::file_manager::storage::list_chunks(&storage_dir)?;

    if chunk_indices.is_empty() {
        return Err("No chunks found locally, remote fetching not implemented".into());
    }

    let mut output = OpenOptions::new().create(true).write(true).open(destination)?;
    for i in 0..chunk_indices.len() {
        let data = get_chunk(&storage_dir, i)?;
        output.write_all(&data)?;
    }

    Ok(())
}
