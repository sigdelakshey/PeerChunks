// src/indexing/search.rs

use crate::indexing::dht::DHT;
use uuid::Uuid;
use log::{info, error};

pub fn search_file(dht: &DHT, query: &str) -> Vec<String> {
    match Uuid::parse_str(query) {
        Ok(file_id) => {
            if let Some(peers) = dht.get_file_locations(&file_id) {
                let addresses: Vec<String> = peers.into_iter().map(|p| p.address).collect();
                info!("Found file {} at peers: {:?}", file_id, addresses);
                addresses
            } else {
                info!("No peers found for file {}", file_id);
                Vec::new()
            }
        }
        Err(_) => {
            error!("Invalid file_id format in query: {}", query);
            Vec::new()
        }
    }
}
