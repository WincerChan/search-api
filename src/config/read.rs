use serde::Deserialize;
use std::fs::read_to_string;

#[derive(Deserialize)]
pub struct Config {
    pub blog_source: String,
    pub tantivy_db: String,
    pub listen_addr: String,
}

pub fn read_config() -> Config {
    let contents =
        read_to_string("/etc/search.toml").expect("No config file found: /etc/search.toml");
    toml::from_str(&contents).unwrap()
}
