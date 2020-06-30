use std::borrow::Borrow;
use std::fs::{File, OpenOptions};
use std::io::{BufReader, BufWriter};

use serde::{Deserialize, Serialize};

use crate::config::Config;
use crate::plex::PlexMetadata;

#[derive(Deserialize, Serialize)]
pub struct History {
    pub records: Vec<Record>,
}

//TODO: store magnet
#[derive(Deserialize, Serialize)]
pub struct Record {
    pub imdb_id: String,
    pub title: String,
    pub status: State,
    //file: Vec<String>, TODO: Add files
}

#[derive(Deserialize, Serialize)]
pub enum State {
    //Unable to find magnet
    Unfound,
    //Downloading torrent
    Downloading,
    //Movie in library
    Downloaded,
    //Movie in library and cleaned
    Cleaned,
    //Movie in library with no title, imdb_id, none or multiple files, files too large or too small
    //Broken,
}

//Initialize history and updated it with new PMDs
//TODO: do a full diff. Item might be in a different state and needs to be upgraded to Downloaded
pub fn update_history(config: &Config, pmds: &[PlexMetadata]) -> History {
    save_history(&config, init_history(&config, pmds))
}

pub fn add_history(config: &Config, record: Record, mut history: History) -> History {
    history.records.push(record);
    save_history(&config, history)
}

//TODO: make history the manager of deluge and plex data
pub fn add_torrent() {
    //check if it exists in history and add it as Downloading if it doesnt
}

//Initialize history from file or from scratch and update it with latest state
fn init_history(config: &Config, pmds: &[PlexMetadata]) -> History {
    if let Ok(file) = File::open(&config.history_file) {
        let reader = BufReader::new(file);
        let mut history: History = serde_json::from_reader(reader)
            .expect(&format!("Unable to deserialize history {}", &config.history_file));

        //if pmds doesnt exist in history add it as downloaded
        let unique_pmds = pmds_not_current(pmds, &history);
        add_pmds_list(unique_pmds.into_iter(), &mut history);
        history
    } else {
        let mut history = History { records: Vec::new() };

        //add all pmds to history
        add_pmds_list(pmds.iter(), &mut history);
        history
    }
}

fn save_history(config: &Config, history: History) -> History {
    let file = OpenOptions::new()
        .write(true).create_new(true).open(&config.history_file)
        .expect(&format!("Unable to create history file {}", &config.history_file));

    serde_json::to_writer(BufWriter::new(file), &history)
        .expect(&format!("Unable to initalize history {}", &config.history_file));
    history
}

fn pmds_not_current<'a>(pmds: &'a [PlexMetadata], histories: &History) -> Vec<&'a PlexMetadata> {
    pmds.iter().filter(|pmd|
        !histories.records.iter().any(|history|
            history.imdb_id == pmd.imdb_guid() && (
                matches!(history.status, State::Downloaded) || matches!(history.status, State::Cleaned)
            )
        )).collect()
}

//TODO: unit test this, lots of complex logic
fn add_pmds_list<'a>(pmds: impl Iterator<Item=&'a PlexMetadata>, history: &mut History) {
    for pmd in pmds {
        let record = Record {
            imdb_id: pmd.imdb_guid(),
            title: pmd.title.clone(),
            status: State::Downloaded,
        };

        match history.records.iter_mut().find(|record| record.imdb_id == pmd.imdb_guid()) {
            Some(_record) => *_record = record,
            None => history.records.push(record),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_overwriting_with_add_pmds_list() {
        let mut history = History {
            records: vec!(Record {
                imdb_id: "tt0021749".into(),
                title: "123".into(),
                status: State::Downloading,
            })
        };
        add_pmds_list(
            [
                PlexMetadata {
                    title: "abc".into(),
                    ratingKey: "123".into(),
                    guid: "com.plexapp.agents.imdb://tt0021749?lang=en".into(),
                }].iter(), &mut history);
        assert!(matches!(history.records.first().unwrap().status, State::Downloaded));
        assert_eq!(history.records.first().unwrap().title, "abc");
        assert_eq!(history.records.first().unwrap().imdb_id, "tt0021749");

    }
}