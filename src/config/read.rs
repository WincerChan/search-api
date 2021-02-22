use std::fs::read_to_string;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct Config {
    pub blog_source: String,
    pub tantivy_db: String,
    pub listen_addr: String
}

pub fn read_config() -> Config {
    let contents = read_to_string("/etc/search.toml").expect("Something went wrong.");
    toml::from_str(&contents).unwrap()
}