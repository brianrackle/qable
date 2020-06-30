use std::{env, time};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::thread::sleep;

use clap::{App, Arg, ArgMatches};

use config::Config;
use deluge::add_torrent;
use imdb::get_imdb_list;
use plex::{get_plex_library_guids, PlexMetadata, refresh_plex_library};
use rarbg::{get_rarbg_magnet, get_rarbg_token};
use tmdb::get_movie_title;
use crate::history::update_history;

mod history;

mod tmdb;
mod request;
mod config;
mod imdb;
mod deluge;
mod plex;
mod rarbg;

//TODO: refactor this into a function which retuns a tuple of all unwrapped data?
fn add_torrent_by_imdb_id(config: &Config,
                          token_option: &Option<String>,
                          plex_metadata: &[PlexMetadata],
                          imdb_id: &str,
                          title_option: Option<String>) {
    let mut err_index = 0usize;
    let errors = [
        format!("Skipping (Title Not Found) {}", &imdb_id),
        format!("Skipping (Unable to Retrieve Token) {}", &imdb_id),
        format!("Skipping (Already Exists) {}", &imdb_id),
        format!("Skipping (Magnet Not Found) {}", &imdb_id),
    ];

    //TODO: change err_index to some/none to make error logic better
    if let Some(title) = title_option {
        err_index += 1;
        if let Some(token) = token_option {
            err_index += 1;
            if !plex_metadata.iter().any(|x| x.imdb_guid() == imdb_id.to_lowercase()) {
                err_index += 1;
                if let Some(magnet) = get_rarbg_magnet(&config, &token, &imdb_id) {
                    err_index += 1;
                    add_torrent(&config, &magnet);
                    println!("Downloading {}: \"{}\"", &imdb_id, &title);
                    sleep(time::Duration::from_millis(config.list_frequency_millis));
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
            .about("download torrent using magnet link"))
        .arg(Arg::with_name("imdb_id")
            .short('d')
            .long("imdb_id")
            .takes_value(true)
            .about("download torrent using imdb guid"))
        .arg(Arg::with_name("imdb_list")
            .short('l')
            .long("imdb_list")
            .takes_value(true)
            .about("download torrents using imdb list id"))
        .arg(Arg::with_name("clean")
            .short('c')
            .long("clean")
            .takes_value(false)
            .about("clean plex media library"))
        .arg(Arg::with_name("export")
            .short('e')
            .long("export")
            .takes_value(false)
            .about("export plex imdb ids"))
        .arg(Arg::with_name("import")
            .short('i')
            .long("import")
            .takes_value(false)
            .about("download torrents using imdb id file"))
        .arg(Arg::with_name("refresh")
            .short('r')
            .long("refresh")
            .takes_value(false)
            .about("refresh plex library and movie database"))
        .get_matches()
}

fn main() {
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

    let plex_metadata = get_plex_library_guids(&config)
        .expect("Exiting (Plex GUIDs Not Found)");
    let history = update_history(&config, &plex_metadata);

    if let Some(imdb_id_file) = matches.value_of("import") {
        unimplemented!();
    } else if let Some(imdb_list_id) = matches.value_of("imdb_list") {
        let token = get_rarbg_token(&config);
        for imdb_id in get_imdb_list(imdb_list_id).iter() {
            add_torrent_by_imdb_id(&config,
                                   &token,
                                   &plex_metadata,
                                   &imdb_id,
                                   get_movie_title(&config, imdb_id));
        }
    } else if let Some(imdb_id) = matches.value_of("imdb_id") {
        let token = get_rarbg_token(&config);
        add_torrent_by_imdb_id(&config,
                               &token,
                               &plex_metadata,
                               &imdb_id,
                               get_movie_title(&config, imdb_id));
    } else if let Some(magnet) = matches.value_of("magnet") {
        add_torrent(&config, &magnet);
    } else if matches.is_present("clean") {
        for item_metadata in plex_metadata {
            if let Some(tmdb_title) = get_movie_title(&config, &item_metadata.imdb_guid()) {
                if item_metadata.title != tmdb_title {
                    println!("Updating ratingKey {} imdb_id {} from \"{}\" to \"{}\"",
                             item_metadata.ratingKey,
                             item_metadata.imdb_guid(),
                             item_metadata.title,
                             tmdb_title);
                    plex::put_plex_movie_metadata(&config, &item_metadata.ratingKey, &tmdb_title);
                }
            }
        }
    } else if matches.is_present("export") {
        for item_metadata in plex_metadata {
            println!("{}", item_metadata.imdb_guid());
        }
    } else if matches.is_present("refresh") {
        refresh_plex_library(&config);
    }
}

