use serde::Deserialize;
use std::{env, fs::read_to_string, path::Path, process::exit};

#[derive(Deserialize)]
pub struct Database {
    pub blog_source: String,
    pub tantivy_db: String,
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

pub fn read_config() -> Config {
    let path = env::var("CONFIG");
    match path {
        Ok(val) => {
            let contents = read_to_string(Path::new(&val).join("search.toml"))
                .expect("No config file found: search.toml");
            toml::from_str(&contents).unwrap()
        }
        Err(_) => {
            println!("Please set environment variable `CONFIG`");
            exit(1)
        }
    }
}
