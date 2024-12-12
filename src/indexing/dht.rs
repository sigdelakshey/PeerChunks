// src/indexing/dht.rs

use crate::peer::discovery::Peer;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use uuid::Uuid;
use log::info;

#[derive(Clone, Debug)]
pub struct DHT {
    // In a production DHT, you'd have a more complex data structure and routing algorithm.
    inner: Arc<Mutex<HashMap<Uuid, Vec<Peer>>>>,
}

impl DHT {
    pub fn new() -> Self {
        DHT {
            inner: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn register_file_location(&self, file_id: Uuid, peer: Peer) {
        let mut map = self.inner.lock().unwrap();
        map.entry(file_id).or_insert_with(Vec::new);
        let peer_address = peer.address.clone();
    
        if let Some(peers) = map.get_mut(&file_id) {
            if !peers.iter().any(|p| p.address == peer_address) {
                peers.push(peer);
            }
        }
    
        info!("Registered file {} at peer {}", file_id, peer_address);
    }
    

    pub fn get_file_locations(&self, file_id: &Uuid) -> Option<Vec<Peer>> {
        let map = self.inner.lock().unwrap();
        map.get(file_id).cloned()
    }
}
