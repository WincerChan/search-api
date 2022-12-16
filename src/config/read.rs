use serde::Deserialize;
use std::{fs::read_to_string, path::Path};

#[derive(Deserialize)]
pub struct Database {
    pub atom_url: String,
    pub tantivy_db: String,
    pub update_interval: u64,
}

#[derive(Deserialize)]
pub struct Network {
    pub listen_type: String,
    pub listen_addr: String,
}

#[derive(Deserialize)]
pub struct Config {
    pub database: Database,
    pub network: Network,
}

pub fn read_config(path: String) -> Config {
    // let path = env::var("CONFIG");
    let contents = read_to_string(Path::new(&path)).expect("No config file found.");
    toml::from_str(&contents).unwrap()
}
