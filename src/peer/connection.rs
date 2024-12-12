// src/peer/connection.rs

use crate::peer::encryption::{encrypt, decrypt};
use crate::peer::discovery::Peer;
use crate::file_manager::storage;
use crate::indexing::dht::DHT;
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
    dht: DHT,
    local_peer: Peer,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let peer_addr = stream.peer_addr()?;
    info!("New connection from {}", peer_addr);

    let welcome_message = format!("Welcome to ShareSphere, peer {}", peer_addr);
    let (nonce, encrypted_welcome) = encrypt(welcome_message.as_bytes(), &encryption_key)?;
    let message = format!("{}:{}\n", nonce, encrypted_welcome);
    stream.write_all(message.as_bytes()).await?;

    stream.write_all(b"DHT_REQUEST\n").await?;

    let mut buffer = Vec::new();
    loop {
        let mut temp_buffer = [0u8; 4096];
        let bytes_read = stream.read(&mut temp_buffer).await?;
        if bytes_read == 0 {
            info!("Connection closed by {}", peer_addr);
            break;
        }
        buffer.extend_from_slice(&temp_buffer[..bytes_read]);

        while let Some(pos) = buffer.iter().position(|&b| b == b'\n') {
            let line = buffer.drain(..=pos).collect::<Vec<u8>>();
            let line_str = String::from_utf8_lossy(&line).trim().to_string();

            if line_str.starts_with("DHT_RESPONSE:") {
                // Format: DHT_RESPONSE:<N>
                let parts: Vec<&str> = line_str.split(':').collect();
                if parts.len() == 2 {
                    let n = parts[1].parse::<usize>().unwrap_or(0);
                    let mut entries = Vec::new();
                    for _ in 0..n {
                        let entry_line = read_line(&mut buffer, &mut stream).await?;
                        // FILE_ID:PEER_ADDRESS
                        let eparts: Vec<&str> = entry_line.split(':').collect();
                        if eparts.len() == 2 {
                            if let Ok(fid) = Uuid::parse_str(eparts[0]) {
                                entries.push((fid, eparts[1].to_string()));
                            }
                        }
                    }
                    dht.merge_entries(&entries);

                    let our_entries = dht.all_entries();
                    stream.write_all(format!("DHT_RESPONSE:{}\n", our_entries.len()).as_bytes()).await?;
                    for (fid, addr) in our_entries {
                        stream.write_all(format!("{}:{}\n", fid, addr).as_bytes()).await?;
                    }

                }
            } else if line_str == "DHT_REQUEST" {
                let our_entries = dht.all_entries();
                stream.write_all(format!("DHT_RESPONSE:{}\n", our_entries.len()).as_bytes()).await?;
                for (fid, addr) in our_entries {
                    stream.write_all(format!("{}:{}\n", fid, addr).as_bytes()).await?;
                }
            } else if line_str.starts_with("CHUNK_REQUEST:") {
                // CHUNK_REQUEST:<FILE_ID>:<CHUNK_INDEX>
                let parts: Vec<&str> = line_str.split(':').collect();
                if parts.len() == 3 {
                    let file_id_str = parts[1];
                    let chunk_index_str = parts[2];
                    if let Ok(fid) = Uuid::parse_str(file_id_str) {
                        if let Ok(chunk_index) = chunk_index_str.parse::<usize>() {
                            let storage_dir = Path::new(&storage_root).join(fid.to_string());
                            match storage::get_chunk(&storage_dir, chunk_index) {
                                Ok(data) => {
                                    let response = format!("CHUNK_RESPONSE:{}:{}:{}:", fid, chunk_index, data.len());
                                    stream.write_all(response.as_bytes()).await?;
                                    stream.write_all(&data).await?;
                                }
                                Err(e) => {
                                    error!("Failed to get chunk: {}", e);
                                }
                            }
                        }
                    }
                }
            } else if line_str.starts_with("CHUNK_RESPONSE:") {
                // Already handled chunk requests externally (e.g., in download_file)
                // If handle_connection is also used by the downloading peer, handle it similarly.
                // This is if we do chunk fetch here. For simplicity, chunk fetch logic might be separate.
            } else {
                let parts: Vec<&str> = line_str.split(':').collect();
                if parts.len() == 2 {
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
    }

    Ok(())
}

pub async fn send_chunk_to_peer(
    peer: &Peer,
    storage_dir: &str,
    file_id: &Uuid,
    chunk_index: usize,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let mut stream = TcpStream::connect(&peer.address).await?;
    info!("Connected to peer {}", peer.address);

    let chunk_data = storage::get_chunk(storage_dir, chunk_index)?;

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

async fn read_line(buffer: &mut Vec<u8>, stream: &mut TcpStream) -> Result<String, Box<dyn Error + Send + Sync>> {
    loop {
        if let Some(pos) = buffer.iter().position(|&b| b == b'\n') {
            let line = buffer.drain(..=pos).collect::<Vec<u8>>();
            return Ok(String::from_utf8_lossy(&line).trim().to_string());
        } else {
            let mut temp = [0u8; 1024];
            let bytes_read = stream.read(&mut temp).await?;
            if bytes_read == 0 {
                return Err("Connection closed".into());
            }
            buffer.extend_from_slice(&temp[..bytes_read]);
        }
    }
}
