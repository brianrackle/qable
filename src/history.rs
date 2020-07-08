use core::time;
use std::borrow::{Borrow, BorrowMut};
use std::fs::{File, OpenOptions};
use std::io::{BufReader, BufWriter};
use std::ops::{Deref, DerefMut};
use std::path::Path;
use std::thread::sleep;

use serde::{Deserialize, Serialize};

use crate::{deluge, plex, rarbg, tmdb};
use crate::config::Config;
use crate::plex::PlexMetadata;
use crate::rarbg::get_rarbg_token;
use crate::tmdb::get_movie_title;

pub struct MediaManager {
    history: History,
    pub config: Config,
    rarbg_token: String,
}

//TODO: Change to map with imdb_id as key
#[derive(Deserialize, Serialize)]
struct History {
    records: Vec<Record>,
}

//TODO: store magnet
#[derive(Deserialize, Serialize)]
struct Record {
    imdb_id: String,
    title: String,
    status: State,
}

#[derive(Deserialize, Serialize, Debug)]
enum State {
    //Movie not found in library
    Missing,
    //Downloading torrent
    Downloading,
    //Movie in library
    Downloaded,
    //Movie in library and cleaned
    Cleaned,
}

enum ChangeOption {
    Upsert,
    Insert,
    Update,
}

//TODO: make history the manager of deluge and plex data
//TODO: remove deluge entry once movie is Downloaded
//TODO: record time with state so stuck downloading movies can eventually be cleared
impl MediaManager {
    pub fn new(config_path: &Path) -> MediaManager {
        let config = match File::open(&config_path) {
            Err(why) => panic!("couldn't open config: {}", why),
            Ok(file) => serde_json::from_reader(BufReader::new(file)).unwrap(),
        };
        let pmds = plex::get_plex_library_guids(&config).expect("Exiting (Plex GUIDs Not Found)");
        let rarbg_token = get_rarbg_token(&config).unwrap();
        //TODO: set the state to broken if plex library does not have the title in History
        // Movie not found in library
        let mut manager = MediaManager {
            config,
            rarbg_token,
            history: History { records: Vec::new() },
        };
        //init is overwriting existing history with plex
        manager.init_history(&pmds);
        manager.save_history();
        manager
    }

    //check if it exists in history and add it as downloading if it doesnt
    pub fn add_torrent(&mut self,
                       imdb_id: &str,
                       save: bool) -> bool {
        let mut err_index = 0usize;
        let errors = [
            format!("Skipping (Title Not Found) {}", &imdb_id),
            format!("Skipping (Already Exists) {}", &imdb_id),
            format!("Skipping (Magnet Not Found) {}", &imdb_id),
        ];

        let title_option = get_movie_title(&self.config, imdb_id);
        if let Some(title) = title_option {
            err_index += 1;
            if !self.history.records.iter().any(|x| x.imdb_id == imdb_id.to_lowercase()) {
                err_index += 1;
                if let Some(magnet) = rarbg::get_rarbg_magnet(&self.config, &self.rarbg_token, &imdb_id) {
                    err_index += 1;
                    deluge::add_torrent(&self.config, &magnet);
                    self.add_record(
                        Record {
                            imdb_id: imdb_id.to_string(),
                            title: title.clone(),
                            status: State::Downloading,
                        },
                        ChangeOption::Insert,
                        |_, _| true);
                    if save {
                        self.save_history();
                    }

                    println!("Downloading {}: \"{}\"", &imdb_id, &title);
                    sleep(time::Duration::from_millis(self.config.list_frequency_millis));
                }
            }
        }

        if err_index < errors.len() {
            println!("{}", errors[err_index])
        }

        err_index >= errors.len()
    }

    pub fn clean_library(&mut self, save: bool) {
        let mut updates = Vec::<Record>::new();
        let missing_ids: Vec<String> = self.history.records.iter()
            .filter(|record| matches!(record.status, State::Missing))
            .map(|record| record.imdb_id.clone())
            .collect();

        for missing_id in missing_ids {
            let position = self.history.records.iter()
                .position(|record| record.imdb_id == missing_id).unwrap();
            self.history.records.remove(position);
            self.add_torrent(&missing_id, false);
        }

        // TODO: panic will cause unsaved history
        for record in self.history.records.iter()
            .filter(|r| matches!(r.status, State::Downloaded)) {
            if let Some(tmdb_title) = tmdb::get_movie_title(&self.config, &record.imdb_id) {
                let updated_record = Record {
                    imdb_id: record.imdb_id.clone(),
                    title: tmdb_title.clone(),
                    status: State::Cleaned,
                };

                if record.title != tmdb_title {
                    match plex::find_key_by_imdb_id(&self.config, &record.imdb_id) {
                        Some(key) => {
                            plex::put_plex_movie_metadata(&self.config, &key, &tmdb_title);
                            updates.push(updated_record);
                            println!("Updating imdb_id {} from \"{}\" to \"{}\"",
                                     record.imdb_id,
                                     record.title,
                                     tmdb_title);
                        }
                        None => println!("Unable to update imdb_id (No rating key found) {}", record.imdb_id),
                    }
                } else {
                    updates.push(updated_record);
                }
            }
        }

        for update in updates {
            self.add_record(update,
                            ChangeOption::Update,
                            |_, _| true);
        }
        if save {
            self.save_history();
        }
    }

    // Initialize history from file or from scratch and update it with latest state
    fn init_history(&mut self, pmds: &[plex::PlexMetadata]) {
        match File::open(&self.config.history_file) {
            Ok(file) => {
                let reader = BufReader::new(file);
                self.history = serde_json::from_reader(reader)
                    .expect(&format!("Unable to deserialize history {}", &self.config.history_file));

                self.set_missing_records(pmds);
                self.pmds_to_upsert(pmds)
                    .iter()
                    .copied()
                    .map(MediaManager::pmd_to_record)
                    .for_each(|record| self.upsert_record(record));
            }
            Err(..) => {
                pmds
                    .iter()
                    .map(MediaManager::pmd_to_record)
                    .for_each(|record| self.upsert_record(record));
            }
        }
    }

    //Make State Missing if item is in History but not Plex library
    fn set_missing_records(&mut self, pmds: &[plex::PlexMetadata]) {
        self.history.records.iter_mut()
            .filter(
                |record| {
                    !pmds.iter().any(|pmd| pmd.imdb_guid() == record.imdb_id)
                })
            .for_each(|r| r.status = State::Missing);
    }

    fn add_record(&mut self,
                  record: Record,
                  change_option: ChangeOption,
                  predicate: impl Fn(&History, &Record) -> bool) {
        match change_option {
            ChangeOption::Upsert => {
                self.upsert_record_with_predicate(record, predicate);
            }
            ChangeOption::Insert => {
                if !self.history.records.iter().any(|r| r.imdb_id == record.imdb_id) {
                    self.upsert_record_with_predicate(record, predicate);
                }
            }
            ChangeOption::Update => {
                if self.history.records.iter().any(|r| r.imdb_id == record.imdb_id) {
                    self.upsert_record_with_predicate(record, predicate);
                }
            }
        }
    }

    pub fn save_history(&self) {
        let file = OpenOptions::new()
            .write(true).create(true).truncate(true).open(&self.config.history_file)
            .expect(&format!("Unable to create history file {}", &self.config.history_file));

        serde_json::to_writer(BufWriter::new(file), &self.history)
            .expect(&format!("Unable to initalize history {}", &self.config.history_file));
    }

    // returns pmds which have an imdb_id
    // if the imdb_id matches an existing then entry should have Downloaded or Cleaned status
    fn pmds_to_upsert<'a>(&self, pmds: &'a [plex::PlexMetadata]) -> Vec<&'a plex::PlexMetadata> {
        pmds.iter().filter(|pmd|
            !pmd.imdb_guid().is_empty() &&
                !self.history.records.iter().any(|history|
                    history.imdb_id == pmd.imdb_guid() && (
                        matches!(history.status, State::Downloaded) || matches!(history.status, State::Cleaned)
                    )
                )).collect()
    }

    //convert a pmd to a movie record. If the pmd exists as downloading, convert it to cleaned
    fn pmd_to_record(pmd: &PlexMetadata) -> Record {
        //TODO: create functions to do basic operations like any
        //        Record {
        //             imdb_id: pmd.imdb_guid(),
        //             title: pmd.title.clone(),
        //             status: match self
        //                 .history.records.iter().any(|record| record.imdb_id == pmd.imdb_guid() && matches!(record.status, State::Downloading)) {
        //                 true => matched,
        //                 false => unmatched,
        //             },
        Record {
            imdb_id: pmd.imdb_guid(),
            title: pmd.title.clone(),
            status: State::Downloaded,
        }
    }

    // Upserts if predicate returns true, or is None
    fn upsert_record_with_predicate(&mut self,
                                    record: Record,
                                    predicate: impl Fn(&History, &Record) -> bool) {
        if predicate(&self.history, &record) {
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

    fn media_manager() -> MediaManager {
        MediaManager {
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
                tmdb_v4_api_key: "".to_string(),
            },
            history: History { records: vec!(
                    Record { imdb_id: "tt0021749".into(), title: "in pmds".into(), status: State::Cleaned},
                    Record { imdb_id: "tt0597723552".into(), title: "not in pmds and no magnet".into(), status: State::Cleaned},
                    Record { imdb_id: "tt0023984212".into(), title: "not in pmds and no magnet and missing".into(), status: State::Missing})
            },
            rarbg_token: "".into(),
        }
    }

    fn plex_metadata() -> [PlexMetadata; 3] {
        [
            plex::PlexMetadata {
                title: "in history".into(),
                ratingKey: "53462".into(),
                guid: "com.plexapp.agents.imdb://tt0021749?lang=en".into(),
            },
            plex::PlexMetadata {
                title: "not in history".into(),
                ratingKey: "09823".into(),
                guid: "com.plexapp.agents.imdb://tt423090134?lang=en".into(),
            },
            plex::PlexMetadata {
                title: "no guid".into(),
                ratingKey: "2342145".into(),
                guid: "local://3149".into(),
            }, ]
    }

    #[test]
    fn test_missing_records() {
        let mut manager = media_manager();
        let pmds = plex_metadata();
        manager.set_missing_records(&pmds);
        assert!(matches!(manager.history.records[0].status, State::Downloading));
        assert!(matches!(manager.history.records[1].status, State::Missing));
    }

    #[test]
    fn test_pmds_to_upsert() {
        let manager = media_manager();
        let pmds = plex_metadata();
        let upserts = manager.pmds_to_upsert(&pmds);
        assert_eq!(upserts.len(), 2);
    }

    #[test]
    fn test_missing_guid() {
        let pmds = plex_metadata();
        assert_eq!(pmds[2].imdb_guid(), "");
    }

    #[test]
    fn test_clean() {
        let mut manager = media_manager();
        let pmds = plex_metadata();
        manager.pmds_to_upsert(&pmds)
            .iter()
            .copied()
            .map(MediaManager::pmd_to_record)
            .for_each(|record| manager.upsert_record(record));

        manager.clean_library(false);
        let expected_records = [
            Record{imdb_id:"tt0021749".into(), title: "in pmds".into(), status: State::Cleaned},
            Record{imdb_id: "tt0597723552".into(), title: "not in pmds and no magnet".into(), status: State::Cleaned},
            Record{imdb_id: "tt423090134".into(), title: "not in history".into(), status: State::Missing},
        ];
        for (actual, expected) in expected_records.iter().zip(&manager.history.records) {
            assert_eq!(actual.imdb_id, expected.imdb_id);
            assert_eq!(actual.title, expected.title);
            if actual.imdb_id == "tt423090134" {
                assert!(matches!(actual.status, State::Missing));
            } else {
                assert!(matches!(actual.status, State::Cleaned));
            }
        }
    }
}