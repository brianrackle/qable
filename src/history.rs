use core::time;
use std::fs::{File, OpenOptions};
use std::io::{BufReader, BufWriter};
use std::path::Path;
use std::thread::sleep;

use serde::{Deserialize, Serialize};

use crate::config::Config;
use crate::deluge;
use crate::plex::{get_plex_library_guids, PlexMetadata};
use crate::rarbg::get_rarbg_magnet;

pub struct MediaManager {
    pub history: History,
    pub config: Config,
}

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
    //Unfound,
    //Downloading torrent
    Downloading,
    //Movie in library
    Downloaded,
    //Movie in library and cleaned
    Cleaned,
    //Movie in library with no title, imdb_id, none or multiple files, files too large or too small
    //Broken,
}

//TODO: make history the manager of deluge and plex data
impl MediaManager {
    pub fn new(config_path: &Path) -> MediaManager {
        let config = match File::open(&config_path) {
            Err(why) => panic!("couldn't open config: {}", why),
            Ok(file) => serde_json::from_reader(BufReader::new(file)).unwrap(),
        };
        let pmds = get_plex_library_guids(&config).expect("Exiting (Plex GUIDs Not Found)");

        let mut manager = MediaManager {
            config,
            history: History { records: Vec::new() },
        };
        //init is overwriting existing history with plex
        manager.init_history(&pmds);
        manager.save_history();
        manager
    }

    //TODO: have save options (upsert, insert if doesnt exist)
    // validate incoming record before saving
    //could simplify !self.history.records.iter().any(|... logic with (insert if doesnt exists)
    fn add_record_and_save_history(&mut self, record: Record) {
        self.upsert_record(record);
        self.save_history()
    }

    //check if it exists in history and add it as downloading if it doesnt
    pub fn add_torrent_and_save(&mut self,
                                token_option: &Option<String>,
                                imdb_id: &str,
                                title_option: Option<String>) {
        let mut err_index = 0usize;
        let errors = [
            format!("Skipping (Title Not Found) {}", &imdb_id),
            format!("Skipping (Unable to Retrieve Token) {}", &imdb_id),
            format!("Skipping (Already Exists) {}", &imdb_id),
            format!("Skipping (Magnet Not Found) {}", &imdb_id),
        ];

        if let Some(title) = title_option {
            err_index += 1;
            if let Some(token) = token_option {
                err_index += 1;
                if !self.history.records.iter().any(|x| x.imdb_id == imdb_id.to_lowercase()) {
                    err_index += 1;
                    if let Some(magnet) = get_rarbg_magnet(&self.config, &token, &imdb_id) {
                        err_index += 1;
                        deluge::add_torrent(&self.config, &magnet);
                        self.add_record_and_save_history(Record {
                            imdb_id: imdb_id.to_string(),
                            title: title.clone(),
                            status: State::Downloading,
                        });
                        println!("Downloading {}: \"{}\"", &imdb_id, &title);
                        sleep(time::Duration::from_millis(self.config.list_frequency_millis));
                    }
                }
            }
        }

        if err_index < errors.len() {
            println!("{}", errors[err_index])
        }
    }

    //Initialize history from file or from scratch and update it with latest state
    fn init_history(&mut self, pmds: &[PlexMetadata]) {
        match File::open(&self.config.history_file) {
            Ok(file) => {
                let reader = BufReader::new(file);

                //TODO: this is fucked up it's overwriting the history data with plex data
                self.history = serde_json::from_reader(reader)
                    .expect(&format!("Unable to deserialize history {}", &self.config.history_file));
                let unique_pmds = self.pmds_to_add_or_update(pmds);
                self.add_pmds_list(unique_pmds.into_iter());
            }
            Err(..) => {
                self.add_pmds_list(pmds.iter());
            }
        }
    }

    fn save_history(&self) {
        let file = OpenOptions::new()
            .write(true).create(true).open(&self.config.history_file)
            .expect(&format!("Unable to create history file {}", &self.config.history_file));

        serde_json::to_writer(BufWriter::new(file), &self.history)
            .expect(&format!("Unable to initalize history {}", &self.config.history_file));
    }

    fn pmds_to_add_or_update<'a>(&self, pmds: &'a [PlexMetadata]) -> Vec<&'a PlexMetadata> {
        pmds.iter().filter(|pmd|
            !self.history.records.iter().any(|history|
                history.imdb_id == pmd.imdb_guid() && (
                    matches!(history.status, State::Downloaded) || matches!(history.status, State::Cleaned)
                )
            )).collect()
    }

    fn add_pmds_list<'a>(&mut self, pmds: impl Iterator<Item=&'a PlexMetadata>) {
        for pmd in pmds {
            let record = Record {
                imdb_id: pmd.imdb_guid(),
                title: pmd.title.clone(),
                status: State::Downloaded,
            };

            self.upsert_record(record);
        }
    }

    fn upsert_record(&mut self, record: Record) {
        match self.history.records.iter_mut().find(
            |r| r.imdb_id == record.imdb_id) {
            Some(_record) => *_record = record,
            None => self.history.records.push(record),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_overwriting_with_add_pmds_list() {
        let mut manager = MediaManager {
            config: Config {
                history_file: "".to_string(),
                deluge_url: "".to_string(),
                password: "".to_string(),
                move_completed_path: "".to_string(),
                download_location: "".to_string(),
                plex_url: "".to_string(),
                plex_token: "".to_string(),
                min_file_size: 0,
                ideal_file_size: 0,
                min_seeders: 0,
                target_categories: vec![],
                retries: 0,
                api_backoff_millis: 0,
                list_frequency_millis: 0,
                min_imdb_rating: 0,
                tmdb_v4_api_key: "".to_string()
            },
            history: History {
                records: vec!(Record {
                    imdb_id: "tt0021749".into(),
                    title: "123".into(),
                    status: State::Downloading,
                })
            }
        };
        manager.add_pmds_list(
            [
                PlexMetadata {
                    title: "abc".into(),
                    ratingKey: "123".into(),
                    guid: "com.plexapp.agents.imdb://tt0021749?lang=en".into(),
                }].iter());

        assert!(matches!(manager.history.records.first().unwrap().status, State::Downloaded));
        assert_eq!(manager.history.records.first().unwrap().title, "abc");
        assert_eq!(manager.history.records.first().unwrap().imdb_id, "tt0021749");
    }
}