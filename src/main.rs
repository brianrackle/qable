use std::{env, time};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::thread::sleep;

use clap::{App, Arg, ArgMatches};

use config::Config;
use deluge::add_torrent;
use imdb::get_imdb_list;
use plex::get_plex_library_guids;
use rarbg::{get_rarbg_magnet, get_rarbg_token};
use tmdb::get_movie_title;

mod tmdb;
mod request;
mod config;
mod imdb;
mod deluge;
mod plex;
mod rarbg;


fn add_torrent_by_imdb_id(config: &Config, token_option: &Option<String>, plex_guids_option: &Option<Vec<String>>, imdb_id: &str, has_title: bool) {
    let mut err_index = 0usize;
    let errors = [
        format!("Skipping (Title Not Found) {}", &imdb_id),
        format!("Skipping (Unable to Retrieve Token) {}", &imdb_id),
        format!("Skipping (Plex GUIDs Not Found) {}", &imdb_id),
        format!("Skipping (Already Exists) {}", &imdb_id),
        format!("Skipping (Magnet Not Found) {}", &imdb_id),
    ];

    //TODO: change err_index to some/none to make error logic better
    if has_title {
        err_index += 1;
        if let Some(token) = token_option {
            err_index += 1;
            if let Some(plex_guids) = plex_guids_option {
                err_index += 1;
                if !plex_guids.contains(&imdb_id.to_lowercase()) {
                    err_index += 1;
                    if let Some(magnet) = get_rarbg_magnet(&config, &token, &imdb_id) {
                        err_index += 1;
                        add_torrent(&config, &magnet);
                        println!("Downloading {}", &imdb_id);
                        sleep(time::Duration::from_millis(config.list_frequency_millis));
                    }
                }
            }
        }
    }

    if err_index < errors.len() {
        println!("{}", errors[err_index])
    }
}

fn matches() -> ArgMatches {
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
    //TODO: split into individual tools joined by a single application
    //TODO: check for currently downloading and queued by qable
    //Create database (file) of Pending, Downloading, Downloaded, Existing
    //search existing plex library
    //TODO: -v verbose mode, -l log file location
    //TODO: -l can accept multiple lists
    //TODO: -d delete duplicate movies from library
    //TODO: -c clean names in plex library
    // from plex list "ratingKey": "1641", and guid imdb key (see existing plex list)
    // then put_plex_movie_metadata
    //TODO: add minimum imdb rating to config

    //TODO: add ability to compare plex display name with imdb name and fix
    //TODO: add restart option that will pick up list download from last spot
    let matches = matches();
    let env = match env::var("QABLE") {
        Err(_) => env::var("HOME").expect("$HOME not defined") + "/.qable/config.json",
        Ok(e) => e,
    };

    let config_path = Path::new(env.as_str());
    let config: Config = match File::open(&config_path) {
        Err(why) => panic!("couldn't open config {}: {}", env, why),
        Ok(file) => serde_json::from_reader(BufReader::new(file)).unwrap(),
    };

    let plex_guids: Option<Vec<String>> = Some(Vec::new()); //get_plex_library_guids(&config);
    if let Some(imdb_list_id) = matches.value_of("imdb_list") {
        let token = get_rarbg_token(&config);
        //TODO: check if title is a movie using get_move_title
        for imdb_id in get_imdb_list(imdb_list_id).iter() {
            add_torrent_by_imdb_id(&config,
                                   &token,
                                   &plex_guids,
                                   &imdb_id,
                                   get_movie_title(&config, imdb_id).is_some());
        }
    } else if let Some(imdb_id) = matches.value_of("imdb_id") {
        let token = get_rarbg_token(&config);
        add_torrent_by_imdb_id(&config,
                               &token,
                               &plex_guids,
                               &imdb_id,
                               get_movie_title(&config, imdb_id).is_some());
    } else if let Some(magnet) = matches.value_of("magnet") {
        add_torrent(&config, &magnet);
    }
}

