// src/indexing/dht.rs

use crate::peer::discovery::Peer;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use uuid::Uuid;
use log::info;

#[derive(Clone, Debug)]
pub struct DHT {
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
        if let Some(peers) = map.get_mut(&file_id) {
            if !peers.iter().any(|p| p.address == peer.address) {
                peers.push(peer.clone());
            }
        }
        info!("Registered file {} at peer {}", file_id, peer.address);
    }

    pub fn get_file_locations(&self, file_id: &Uuid) -> Option<Vec<Peer>> {
        let map = self.inner.lock().unwrap();
        map.get(file_id).cloned()
    }

    pub fn all_entries(&self) -> Vec<(Uuid, String)> {
        let map = self.inner.lock().unwrap();
        let mut entries = Vec::new();
        for (file_id, peers) in map.iter() {
            for p in peers {
                entries.push((*file_id, p.address.clone()));
            }
        }
        entries
    }

    pub fn merge_entries(&self, entries: &[(Uuid, String)]) {
        for (file_id, address) in entries {
            let peer = Peer { address: address.clone() };
            self.register_file_location(*file_id, peer);
        }
    }
}
