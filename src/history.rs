use std::fs::{File, OpenOptions};
use std::io::{BufReader, BufWriter};

use serde::{Deserialize, Serialize};

use crate::config::Config;
use crate::plex::PlexMetadata;

#[derive(Deserialize, Serialize)]
pub struct History {
    records: Vec<Record>,
}

//TODO: store magnet
#[derive(Deserialize, Serialize)]
struct Record {
    imdb_id: String,
    title: String,
    status: Status,
}

#[derive(Deserialize, Serialize)]
enum Status {
    //Unable to find magnet
    Unfound,
    //Downloading torrent
    Downloading,
    //Movie in library
    Downloaded,
    //Movie in library and cleaned
    Cleaned,
    //Movie in library with no title, or imdb_id
    Broken,
}

pub fn update_history(config: &Config, plex_metadatas: &[PlexMetadata]) -> History {
    if let Ok(file) = File::open(&config.history_file) {
        let reader = BufReader::new(file);
        //if movie doesnt exist add it as downloaded
        let mut temp_history: History = serde_json::from_reader(reader).expect(
            &format!("Unable to deserialize history {}", &config.history_file));

        //add plex metadata that doesnt yet exist in history
        for metadata in plex_metadatas {
            if !temp_history.records.iter().any(|record|
                record.imdb_id == metadata.imdb_guid()) {
                temp_history.records.push(Record {
                    imdb_id: metadata.imdb_guid(),
                    title: metadata.title.clone(),
                    status: Status::Downloaded,
                });
            }
        }
        temp_history
    } else {
        let mut temp_history = History { records: Vec::new() };
        for metadata in plex_metadatas {
            temp_history.records.push(
                Record {
                    imdb_id: metadata.imdb_guid(),
                    title: metadata.title.clone(),
                    status: Status::Downloaded,
                });
        }
        //TODO: should history be written with every operation or at the end of program execution
        let file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&config.history_file)
            .expect(&format!("Unable to create history file {}", &config.history_file));
        serde_json::to_writer(BufWriter::new(file), &temp_history)
            .expect(&format!("Unable to initalize history {}", &config.history_file));

        temp_history
    }
}