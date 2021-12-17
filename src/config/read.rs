use serde::Deserialize;
use std::{fs::read_to_string, path::Path};

#[derive(Deserialize)]
pub struct Database {
    pub atom_url: String,
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

pub fn read_config(path: String) -> Config {
    let abs_path = Path::new(&path);
    if !abs_path.exists() {
        panic!("Config file not found: {}", path);
    }
    let contents = read_to_string(abs_path).expect("Failed to read config file");
    toml::from_str(&contents).expect("Failed to parse config file, check format of file")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{remove_file, File};
    use std::io::Write;
    static TMP_PATH: &str = "/tmp/config.toml";
    fn create_config_file() {
        let config_file = r#"
            [database]
            atom_url = "http://localhost/"
            tantivy_db = "./tantivy_db"
            [network]
            listen_type = 'tcp'
            listen_addr = '127.0.0.1:8834'
        "#;
        let mut file = File::create(TMP_PATH).unwrap();
        file.write_all(config_file.as_bytes()).unwrap();
    }
    fn create_wrong_config_file() {
        let config_file = r#"
            [network]
            listen_type = 'tcp'
            listen_addr = '127.0.0.1:8834'
        "#;
        let mut file = File::create(TMP_PATH).unwrap();
        file.write_all(config_file.as_bytes()).unwrap();
    }

    fn delete_config_file() {
        remove_file(TMP_PATH).unwrap();
    }
    #[test]
    fn test_read_config_succ() {
        create_config_file();
        let config = read_config(TMP_PATH.to_string());
        delete_config_file();
        assert_eq!(config.database.atom_url, "http://localhost/".to_string());
        assert_eq!(config.database.tantivy_db, "./tantivy_db".to_string());
        assert_eq!(config.network.listen_type, "tcp".to_string());
        assert_eq!(config.network.listen_addr, "127.0.0.1:8834".to_string());
    }
    #[test]
    #[should_panic]
    fn test_read_wrong_config() {
        create_wrong_config_file();
        read_config(TMP_PATH.to_string());
        delete_config_file();
    }
    #[test]
    #[should_panic]
    fn test_read_config_fail() {
        read_config("/tmp/config.toml".to_string());
    }
}
