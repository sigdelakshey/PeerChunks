use crate::config::Config;
use crate::peer::connection::handle_connection;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc::Sender;
use std::error::Error;
use log::{info, error};

pub async fn start_peer_discovery(config: Config, _tx: Sender<String>) -> Result<(), Box<dyn Error + Send + Sync>> {
    let listener = TcpListener::bind(("0.0.0.0", config.peer_port)).await?;
    info!("Listening for peers on port {}", config.peer_port);

    for peer in config.bootstrap_peers.iter() {
        let peer = peer.clone(); 
        let encryption_key = config.encryption_key.clone();
        match TcpStream::connect(&peer).await {
            Ok(stream) => {
                info!("Connected to bootstrap peer {}", peer);
                tokio::spawn(async move {
                    if let Err(e) = handle_connection(stream, encryption_key).await {
                        error!("Error handling connection with {}: {}", peer, e);
                    }
                });
            },
            Err(e) => {
                error!("Failed to connect to bootstrap peer {}: {}", peer, e);
            }
        }
    }

    loop {
        let (stream, addr) = listener.accept().await?;
        info!("Accepted connection from {}", addr);
        let encryption_key = config.encryption_key.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_connection(stream, encryption_key).await {
                error!("Error handling connection with {}: {}", addr, e);
            }
        });
    }
}
