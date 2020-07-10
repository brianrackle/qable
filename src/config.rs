use serde::Deserialize;

#[derive(Deserialize)]
pub struct Config {
    pub history_file: String,
    pub deluge_url: String,
    pub password: String,
    pub move_completed_path: String,
    pub download_location: String,
    pub plex_url: String,
    pub plex_token: String,
    pub min_file_size: i64,
    pub ideal_file_size: i64,
    pub seeders: Vec<Seeders>,
    pub target_categories: Vec<String>,
    pub retries: i8,
    pub api_backoff_millis: u64,
    pub list_frequency_millis: u64,
    pub min_imdb_rating: i8,
    pub tmdb_v4_api_key: String,
}

//apply additional restrictions if results qualify
//if filter(seeders >= 2).count() >= min_seeders (try next rule, else use previous rule)
#[derive(Deserialize)]
pub struct Seeders {
    pub available_magnets: u8,
    pub min_seeders: u8,
}