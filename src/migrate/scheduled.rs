use std::{
    thread::{self, sleep},
    time::Duration,
};

use super::init::init_schema;

pub fn scheduled_load_schema(path: &str, source: String, interval: u64) {
    let p_s = path.to_owned();
    thread::spawn(move || loop {
        init_schema(&p_s, &source);
        sleep(Duration::from_secs(interval));
    });
}
