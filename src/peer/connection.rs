use crate::peer::encryption::{encrypt, decrypt, EncryptionError};
use tokio::net::TcpStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::error::Error;
use log::{info, error};

pub async fn handle_connection(mut stream: TcpStream, encryption_key: String) -> Result<(), Box<dyn Error + Send + Sync>> {
    let peer_addr = stream.peer_addr()?;
    info!("New connection from {}", peer_addr);

    let welcome_message = format!("Welcome to PeerChunks, peer {}", peer_addr);
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
    
    Ok(())
}
