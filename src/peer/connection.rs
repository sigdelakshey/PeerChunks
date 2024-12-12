// src/peer/connection.rs

use crate::peer::encryption::{encrypt, decrypt};
use crate::peer::discovery::Peer;
use crate::file_manager::storage;
use crate::file_manager::replication::replicate_chunks;
use tokio::net::TcpStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::error::Error;
use std::path::Path;
use log::{info, error};
use uuid::Uuid;

pub async fn handle_connection(
    mut stream: TcpStream,
    encryption_key: String,
    storage_root: String,
    peers: Vec<Peer>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let peer_addr = stream.peer_addr()?;
    info!("New connection from {}", peer_addr);

    // Send a welcome message
    let welcome_message = format!("Welcome to ShareSphere, peer {}", peer_addr);
    let (nonce, encrypted_welcome) = encrypt(welcome_message.as_bytes(), &encryption_key)
        .map_err(|e| Box::new(e) as Box<dyn Error + Send + Sync>)?;
    let message = format!("{}:{}\n", nonce, encrypted_welcome);
    stream.write_all(message.as_bytes()).await?;

    let mut buffer = Vec::new();
    loop {
        let mut temp_buffer = [0u8; 1024];
        let bytes_read = stream.read(&mut temp_buffer).await?;
        if bytes_read == 0 {
            info!("Connection closed by {}", peer_addr);
            break;
        }
        buffer.extend_from_slice(&temp_buffer[..bytes_read]);

        // Check if the buffer contains a chunk transfer
        if let Some(header_end) = buffer.iter().position(|&b| b == b':') {
            // Attempt to parse as a chunk transfer
            let header = buffer[..header_end].to_vec(); // Changed from .clone() to .to_vec()
            let header_str = String::from_utf8_lossy(&header);
            let parts: Vec<&str> = header_str.splitn(4, ':').collect();
            if parts.len() < 4 {
                continue; // Not enough data yet
            }
            let file_id_str = parts[0];
            let chunk_index_str = parts[1];
            let chunk_size_str = parts[2];
            let chunk_size: usize = match chunk_size_str.parse() {
                Ok(size) => size,
                Err(_) => {
                    error!("Invalid chunk size from {}", peer_addr);
                    continue;
                }
            };

            let expected_length = header_end + 1 + chunk_size;
            if buffer.len() < expected_length {
                continue; // Wait for more data
            }

            let chunk_data = buffer[header_end + 1..expected_length].to_vec();
            buffer.drain(..expected_length);

            let file_id = match Uuid::parse_str(file_id_str) {
                Ok(id) => id,
                Err(_) => {
                    error!("Invalid file ID from {}", peer_addr);
                    continue;
                }
            };
            let chunk_index = match chunk_index_str.parse::<usize>() {
                Ok(index) => index,
                Err(_) => {
                    error!("Invalid chunk index from {}", peer_addr);
                    continue;
                }
            };

            let storage_dir = storage_root.clone();
            let storage_path = Path::new(&storage_dir).join(file_id.to_string());

            // Save the chunk with error conversion
            storage::save_chunk(
                &storage_path,
                &crate::file_manager::chunker::ChunkMetadata::new(file_id, chunk_index, chunk_size, 0),
                &chunk_data,
            )
            .map_err(|e| Box::new(e) as Box<dyn Error + Send + Sync>)?;

            // Send acknowledgment
            stream.write_all(b"OK").await?;

            info!("Received and saved chunk {} of file {}", chunk_index, file_id);
            
            // Initiate replication
            replicate_chunks(&peers, &storage_dir, &file_id).await?;
        } else {
            // Handle encrypted messages or other protocols
            while let Some(pos) = buffer.iter().position(|&b| b == b'\n') {
                let line = buffer.drain(..=pos).collect::<Vec<u8>>();
                let line_str = String::from_utf8_lossy(&line);
                let parts: Vec<&str> = line_str.trim().split(':').collect();
                if parts.len() != 2 {
                    error!("Invalid message format from {}", peer_addr);
                    continue;
                }
                let nonce = parts[0];
                let ciphertext = parts[1];
                match decrypt(nonce, ciphertext, &encryption_key) {
                    Ok(decrypted_data) => {
                        let message = String::from_utf8_lossy(&decrypted_data);
                        info!("Received from {}: {}", peer_addr, message);
                    },
                    Err(e) => {
                        error!("Failed to decrypt message from {}: {}", peer_addr, e);
                    }
                }
            }
        }
    }

    Ok(())
}

/// Sends a specific chunk to a peer.
/// 
/// # Arguments
/// 
/// * `peer` - The target peer to send the chunk to.
/// * `storage_dir` - Directory where chunks are stored.
/// * `file_id` - Unique identifier of the file.
/// * `chunk_index` - Index of the chunk to send.
pub async fn send_chunk_to_peer(
    peer: &Peer,
    storage_dir: &str,
    file_id: &Uuid,
    chunk_index: usize,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let mut stream = TcpStream::connect(&peer.address).await?;
    info!("Connected to peer {}", peer.address);

    let chunk_data = storage::get_chunk(storage_dir, chunk_index)
        .map_err(|e| Box::new(e) as Box<dyn Error + Send + Sync>)?;

    let message = format!(
        "{}:{}:{}:",
        file_id,
        chunk_index,
        chunk_data.len()
    );

    stream.write_all(message.as_bytes()).await?;
    stream.write_all(&chunk_data).await?;

    let mut ack = [0u8; 2];
    stream.read_exact(&mut ack).await?;
    if &ack != b"OK" {
        return Err("Failed to receive acknowledgment from peer".into());
    }

    Ok(())
}
