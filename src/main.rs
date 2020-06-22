use std::env;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use clap::{App, Arg, ArgMatches};
use serde::Deserialize;

use deluge::add_torrent;
use imdb::get_imdb_list;
use plex::get_plex_library_guids;
use rarbg::get_rarbg_magnet;

mod imdb;
mod deluge;
mod plex;
mod rarbg;

#[derive(Deserialize)]
struct Config {
    path: String,
    password: String,
    move_completed_path: String,
    download_location: String,
    plex_server_library: String,
    plex_token: String,
    min_file_size: i64,
    ideal_file_size: i64,
    min_seeders: i32,
    target_categories: Vec<String>,
    api_backoff: i32,
    download_delay: i32,
}

fn add_torrent_by_imdb_id(plex_guids: &Vec<String>, imdb_id: &str, config: &Config) {
    if !plex_guids.contains(&imdb_id.to_lowercase()) {
        if let Some(magnet) = get_rarbg_magnet(&imdb_id) {
            add_torrent(&config.path, &config.password, &magnet, &config.download_location, &config.move_completed_path);
            println!("Downloading {}", &imdb_id);
        } else {
            println!("Skipping {}", &imdb_id);
        }
    }
}

fn init_args() -> ArgMatches {
    App::new("qable")
        .version("0.1.0")
        .author("Brian Rackle <brian@rackle.me>")
        .about("Queues Torrents")
        .arg(Arg::with_name("magnet")
            .short('m')
            .long("magnet")
            .takes_value(true)
            .conflicts_with("imdb_list")
            .conflicts_with("imdb_id")
            .about("Magnet link"))
        .arg(Arg::with_name("imdb_id")
            .short('i')
            .long("imdb_id")
            .takes_value(true)
            .conflicts_with("magnet")
            .conflicts_with("imdb_list")
            .about("imdb guid"))
        .arg(Arg::with_name("imdb_list")
            .short('l')
            .long("imdb_list")
            .takes_value(true)
            .conflicts_with("magnet")
            .conflicts_with("imdb_id")
            .about("imdb list"))
        .get_matches()
}

fn main() {
    let matches = init_args();
    let env = match env::var("QABLE") {
        Err(_) => env::var("HOME").expect("$HOME not defined") + "/.qable/config.json",
        Ok(e) => e,
    };

    let config_path = Path::new(env.as_str());
    let config: Config = match File::open(&config_path) {
        Err(why) => panic!("couldn't open config {}: {}", env, why),
        Ok(file) => serde_json::from_reader(BufReader::new(file)).unwrap(),
    };

    let plex_guids = get_plex_library_guids(&config.plex_server_library, &config.plex_token);

    if let Some(imdb_list_id) = matches.value_of("imdb_list") {
        for imdb_id in get_imdb_list(imdb_list_id) {
            add_torrent_by_imdb_id(&plex_guids, &imdb_id, &config);
        }
    } else if let Some(imdb_id) = matches.value_of("imdb_id") {
        add_torrent_by_imdb_id(&plex_guids, &imdb_id, &config);
    } else if let Some(magnet) = matches.value_of("magnet") {
        add_torrent(&config.path, &config.password, &magnet, &config.download_location, &config.move_completed_path);
    }
}

