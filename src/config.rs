use serde::Deserialize;

#[derive(Deserialize)]
pub struct Config {
    pub path: String,
    pub password: String,
    pub move_completed_path: String,
    pub download_location: String,
    pub plex_server_library: String,
    pub plex_token: String,
    pub min_file_size: i64,
    pub ideal_file_size: i64,
    pub min_seeders: i32,
    pub target_categories: Vec<String>,
    pub retries: i32,
    pub api_backoff_millis: u64,
    pub list_frequency_millis: u64,
}