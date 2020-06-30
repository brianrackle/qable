use std::{env, time};
use std::path::Path;

use clap::{App, Arg, ArgMatches};

use imdb::get_imdb_list;
use plex::refresh_plex_library;
use rarbg::get_rarbg_token;
use tmdb::get_movie_title;

mod history;
mod tmdb;
mod request;
mod config;
mod imdb;
mod deluge;
mod plex;
mod rarbg;

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
            .short('i')
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
    let mut media_manager = history::MediaManager::new(&config_path);

    if let Some(imdb_list_id) = matches.value_of("imdb_list") {
        let token = get_rarbg_token(&media_manager.config);
        for imdb_id in get_imdb_list(imdb_list_id).iter() {
            media_manager.add_torrent_and_save(&token,
                                               &imdb_id,
                                               get_movie_title(&media_manager.config, imdb_id));
        }
    } else if let Some(imdb_id) = matches.value_of("imdb_id") {
        let token = get_rarbg_token(&media_manager.config);
        media_manager.add_torrent_and_save(
                                           &token,
                                           &imdb_id,
                                           get_movie_title(&media_manager.config, imdb_id));
    } else if matches.is_present("clean") {
        media_manager.clean_library();
    } else if matches.is_present("refresh") {
        refresh_plex_library(&media_manager.config);
    }
}

