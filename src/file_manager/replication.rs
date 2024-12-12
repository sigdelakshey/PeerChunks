use crate::peer::discovery::Peer;
use crate::peer::connection::send_chunk_to_peer;
use std::{error::Error, path::Path};
use log::{info, error};

const REPLICATION_FACTOR: usize = 2;

pub async fn replicate_chunks(
    peers: &[Peer],
    storage_dir: &str,
    file_id: &uuid::Uuid,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    for chunk_index in 0..get_total_chunks(storage_dir, file_id)? {
        let peers_to_replicate = select_peers_for_replication(peers, file_id, chunk_index)?;

        for peer in peers_to_replicate {
            if let Err(e) = send_chunk_to_peer(peer, storage_dir, file_id, chunk_index).await {
                error!("Failed to replicate chunk {} to peer {}: {}", chunk_index, peer.address, e);
            } else {
                info!("Replicated chunk {} to peer {}", chunk_index, peer.address);
            }
        }
    }
    Ok(())
}

fn get_total_chunks(storage_dir: &str, file_id: &uuid::Uuid) -> Result<usize, Box<dyn Error + Send + Sync>> {
    use crate::file_manager::storage::list_chunks;
    let storage_path = Path::new(storage_dir).join(file_id.to_string());
    let chunks = list_chunks(&storage_path)?;
    Ok(chunks.len())
}

fn select_peers_for_replication<'a>(
    peers: &'a [Peer],
    file_id: &'a uuid::Uuid,
    chunk_index: usize,
) -> Result<Vec<&'a Peer>, Box<dyn Error + Send + Sync>> {
    let available_peers: Vec<&Peer> = peers.iter()
        .filter(|peer| !peer.address.contains(&file_id.to_string())) // Avoid self-replication
        .collect();

    if available_peers.len() < REPLICATION_FACTOR {
        return Err(format!(
            "Not enough peers to replicate chunk {}. Required: {}, Available: {}",
            chunk_index,
            REPLICATION_FACTOR,
            available_peers.len()
        ).into());
    }

    Ok(available_peers[..REPLICATION_FACTOR].to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::file_manager::chunker::ChunkMetadata;
    use crate::peer::discovery::Peer;
    use uuid::Uuid;
    use std::fs;
    use tempfile::TempDir;

    async fn mock_send_chunk_to_peer(
        peer: &Peer,
        storage_dir: &str,
        file_id: &Uuid,
        chunk_index: usize,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        Ok(())
    }

    #[tokio::test]
    async fn test_replicate_chunks_success() {
        let temp_dir = TempDir::new().unwrap();
        let storage_root = temp_dir.path();

        let file_id = Uuid::new_v4();
        let storage_dir = storage_root.join(file_id.to_string());
        fs::create_dir_all(&storage_dir).unwrap();

        for i in 0..3 {
            let metadata = ChunkMetadata::new(file_id, i, 5, 3);
            let data = format!("Chunk{}", i).into_bytes();
            crate::file_manager::storage::save_chunk(&storage_dir, &metadata, &data).unwrap();
        }

        let peers = vec![
            Peer { address: "127.0.0.1:8081".to_string() },
            Peer { address: "127.0.0.1:8082".to_string() },
            Peer { address: "127.0.0.1:8083".to_string() },
        ];

        let result = replicate_chunks(&peers, storage_dir.to_str().unwrap(), &file_id).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_replicate_chunks_insufficient_peers() {
        let temp_dir = TempDir::new().unwrap();
        let storage_root = temp_dir.path();

        let file_id = Uuid::new_v4();
        let storage_dir = storage_root.join(file_id.to_string());
        fs::create_dir_all(&storage_dir).unwrap();

        for i in 0..2 {
            let metadata = ChunkMetadata::new(file_id, i, 5, 2);
            let data = format!("Chunk{}", i).into_bytes();
            crate::file_manager::storage::save_chunk(&storage_dir, &metadata, &data).unwrap();
        }

        let peers = vec![
            Peer { address: "127.0.0.1:8081".to_string() },
        ];

        let result = replicate_chunks(&peers, storage_dir.to_str().unwrap(), &file_id).await;
        assert!(result.is_err());
    }
}
