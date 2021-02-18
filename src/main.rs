use std::env;
use std::path::Path;

use clap::{App, Arg, ArgMatches};

use plex::refresh_plex_library;

mod history;
mod tmdb;
mod request;
mod config;
mod plex;

fn matches() -> ArgMatches {
    App::new("qable")
        .version("0.1.0")
        .author("Brian Rackle <brian@rackle.me>")
        .about("Queues Torrents")
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

    let config = config::Config::new(&Path::new(env.as_str()));

    if matches.is_present("clean") {
        let _media_manager = history::MediaManager::new(config);
    } else if matches.is_present("refresh") {
        refresh_plex_library(&config);
    }
}

