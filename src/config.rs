use serde::Deserialize;
use std::fs::File;
use std::io::Read;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub peer_port: u16,
    pub bootstrap_peers: Vec<String>,
    pub storage_path: String,
    pub encryption_key: String,
}

impl Config {
    pub fn load(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let mut file = File::open(path)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        let config: Config = serde_yaml::from_str(&contents)?;
        Ok(config)
    }
}
