use serde::Deserialize;
use std::{env, fs::read_to_string, path::Path, process::exit};

#[derive(Deserialize)]
pub struct Config {
    pub blog_source: String,
    pub tantivy_db: String,
    pub listen_addr: String,
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
