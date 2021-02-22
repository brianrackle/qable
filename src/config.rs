use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use serde::Deserialize;

#[derive(Deserialize)]
pub struct Config {
    pub plex_url: String,
    pub plex_token: String,
    pub retries: u8,
    pub api_backoff_millis: u64,
    pub tmdb_v4_api_key: String,
}

impl Config {
    pub fn new(config_path: &Path) -> Config {
        match File::open(&config_path) {
            Err(why) => panic!("couldn't open config: {}", why),
            Ok(file) => serde_json::from_reader(BufReader::new(file)).unwrap(),
        }
    }
}