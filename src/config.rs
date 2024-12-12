// src/config.rs

use serde::Deserialize;
use std::fs;
use std::error::Error;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub peer_port: u16,
    pub bootstrap_peers: Vec<String>,
    pub storage_path: String,
    pub encryption_key: String,
}

impl Config {
    pub fn load<P: AsRef<std::path::Path>>(path: P) -> Result<Self, Box<dyn Error>> {
        let contents = fs::read_to_string(path)?;
        let config: Config = serde_yaml::from_str(&contents)?;
        Ok(config)
    }
}
