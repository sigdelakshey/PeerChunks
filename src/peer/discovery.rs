
use crate::indexing::dht::DHT;
use crate::peer::connection::handle_connection;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc::Sender;
use std::error::Error;
use log::{info, error};

#[derive(Debug, Clone)]
pub struct Peer {
    pub address: String,
}

pub async fn start_peer_discovery(
    config: crate::config::Config,
    _tx: Sender<String>,
    dht: DHT,
    local_peer: Peer,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let listener = TcpListener::bind(("0.0.0.0", config.peer_port)).await?;
    info!("Listening for peers on port {}", config.peer_port);

    let mut peers = Vec::new();
    for peer_addr in config.bootstrap_peers.iter() {
        let peer = Peer { address: peer_addr.clone() };
        peers.push(peer.clone());
        let encryption_key = config.encryption_key.clone();
        let storage_root = config.storage_path.clone();
        let peers_clone = peers.clone();
        let dht_clone = dht.clone();
        let local_peer_clone = local_peer.clone();

        tokio::spawn(async move {
            match TcpStream::connect(&peer.address).await {
                Ok(stream) => {
                    info!("Connected to bootstrap peer {}", peer.address);
                    if let Err(e) = handle_connection(
                        stream, 
                        encryption_key, 
                        storage_root.clone(), 
                        peers_clone, 
                        dht_clone.clone(), 
                        local_peer_clone.clone()
                    ).await {
                        error!("Error handling connection with {}: {}", peer.address, e);
                    }
                },
                Err(e) => {
                    error!("Failed to connect to bootstrap peer {}: {}", peer.address, e);
                }
            }
        });
    }

    loop {
        let (stream, addr) = listener.accept().await?;
        info!("Accepted connection from {}", addr);
        let encryption_key = config.encryption_key.clone();
        let storage_root = config.storage_path.clone();
        let peers_clone = peers.clone();
        let dht_clone = dht.clone();
        let local_peer_clone = local_peer.clone();

        tokio::spawn(async move {
            if let Err(e) = handle_connection(
                stream, 
                encryption_key, 
                storage_root, 
                peers_clone, 
                dht_clone, 
                local_peer_clone
            ).await {
                error!("Error handling connection with {}: {}", addr, e);
            }
        });
    }
}
