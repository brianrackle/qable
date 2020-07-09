use core::time;
use std::borrow::{Borrow, BorrowMut};
use std::fs::{File, OpenOptions};
use std::io::{BufReader, BufWriter};
use std::iter::{Filter, Map};
use std::ops::{Deref, DerefMut};
use std::path::Path;
use std::slice::Iter;
use std::thread::sleep;

use serde::{Deserialize, Serialize};

use crate::{deluge, plex, rarbg, tmdb};
use crate::config::Config;
use crate::plex::PlexMetadata;
use crate::rarbg::get_rarbg_token;
use crate::tmdb::get_movie_title;
use std::collections::HashMap;

pub struct MediaManager {
    history: History,
    pub config: Config,
    rarbg_token: String,
}

#[derive(Deserialize, Serialize)]
struct History {
    records: HashMap<String, Record>,
}

#[derive(Deserialize, Serialize, Clone)]
struct Record {
    imdb_id: String,
    title: String,
    status: State,
}

#[derive(Deserialize, Serialize, Debug, Copy, Clone)]
enum State {
    //Movie not found in library
    Missing,
    //Downloading torrent
    Downloading,
    //Movie in library
    Downloaded,
    //Movie in library and cleaned
    Cleaned,
    //TODO: get rid of downloaded state Missing or Downloading should straight to Downloaded
    //Movie found in library that didn't exist in history
    //Found,
}

#[derive(Copy, Clone)]
enum ChangeOption {
    Upsert,
    Insert,
    Update,
}

//TODO: use enum to return values as they were entered into history or not
#[derive(Clone)]
enum Operation {
    Modified(Record),
    Existing(Record),
    Invalid,
}

impl History {
    pub fn get_records(&self, imdb_ids: &[String], predicate: impl Fn(&Record) -> bool) -> Vec<Option<&Record>> {
        imdb_ids.iter().map(|imdb_id| self.get_record(imdb_id, &predicate)).collect()
    }

    pub fn is_record_in_state(&self, imdb_id: &String, status: State, predicate: impl Fn(&Record) -> bool) -> bool {
        match self.get_record(&imdb_id, predicate) {
            Some(record) => std::mem::discriminant(&record.status) == std::mem::discriminant(&status),
            None => false,
        }
    }

    pub fn get_record(&self, imdb_id: &String, predicate: impl Fn(&Record) -> bool) -> Option<&Record> {
        let mut found = None;
        for (key, value) in self.records.iter() {
            if key == imdb_id {
                if predicate(value) { found = Some(value); } else { break; }
            }
        }
        found
    }

}

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
        let mut manager = MediaManager {
            config,
            rarbg_token,
            history: History { records: HashMap::new() },
        };
        //init is overwriting existing history with plex
        manager.init_history(&pmds);
        manager.save_history();
        manager
    }

    pub fn save_history(&self) {
        let file = OpenOptions::new()
            .write(true).create(true).truncate(true).open(&self.config.history_file)
            .expect(&format!("Unable to create history file {}", &self.config.history_file));

        serde_json::to_writer(BufWriter::new(file), &self.history)
            .expect(&format!("Unable to initalize history {}", &self.config.history_file));
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
            if self.history.get_record(&imdb_id.to_lowercase(), |_| true).is_none() {
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
                        ChangeOption::Upsert,
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
        //
        //imdb_id [history record, pmd]
        //
        //
        //


        let missing_ids: Vec<String> = self.history.records.iter()
            .filter(|(key, value)| matches!(value.status, State::Missing))
            .map(|(key, value)| key.clone())
            .collect();

        for missing_id in missing_ids {
            self.history.records.remove(&missing_id);
            self.add_torrent(&missing_id, false);
        }

        let mut updates = Vec::<Record>::new();
        for (key, value) in self.history.records.iter()
            .filter(|(k, v)| matches!(v.status, State::Downloaded)) {
            if let Some(tmdb_title) = tmdb::get_movie_title(&self.config, &key) {
                //TODO: modify in place instead
                let updated_record = Record {
                    imdb_id: key.clone(),
                    title: tmdb_title.clone(),
                    status: State::Cleaned,
                };

                if value.title != tmdb_title {
                    if let Some(key) = plex::find_key_by_imdb_id(&self.config, key) {
                        plex::put_plex_movie_metadata(&self.config, &key, &tmdb_title);
                        updates.push(updated_record);
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

    fn build_history_mapping(&self, pmds: &[plex::PlexMetadata]) {
        let mut map :HashMap<String, (Option<&Record>, Option<&plex::PlexMetadata>)> = HashMap::new();
        for (key, value) in self.history.records.iter() {
            map.insert(key.clone(), (Some(value), None));
        }
        for pmd in pmds {
            if !pmd.imdb_guid().is_empty() {
                if let Some(value) = map.get_mut(&pmd.imdb_guid()) {
                    value.1 = Some(pmd);
                } else {
                    map.insert(pmd.imdb_guid(), (None, Some(pmd)));
                }
            }
        }
    }

    //TODO: clean should be automatic during init
    // Initialize history from file or from scratch and update it with latest state
    fn init_history(&mut self, pmds: &[plex::PlexMetadata]) {
        match File::open(&self.config.history_file) {
            Ok(file) => {
                let reader = BufReader::new(file);
                self.history = serde_json::from_reader(reader).expect(&format!("Unable to deserialize history {}", &self.config.history_file));

                self.set_missing_records(pmds);

                //Update non-cleaned pmds as downloaded  which then allows it to be cleaned next
                //TODO: MAINTENANCE Can convert this to upsert...
                self.add_pmds(pmds,
                              ChangeOption::Update,
                              |history, record| {
                                  !history.is_record_in_state(&record.imdb_id, State::Cleaned, |_| true)
                              });

                //Insert new pmds as downloaded
                self.add_pmds(pmds, ChangeOption::Insert, |_, _| true);
            }
            Err(..) => {
                self.add_pmds(pmds, ChangeOption::Insert, |_, _| true);
            }
        }
    }

    fn add_record(&mut self,
                  record: Record,
                  change_option: ChangeOption,
                  predicate: impl Fn(&History, &Record) -> bool) -> bool {
        let mut change = false;
        if !record.imdb_id.is_empty() {
            change = match change_option {
                ChangeOption::Upsert => {
                    self.upsert_record(record, predicate)
                }
                ChangeOption::Insert => {
                    if !self.history.records.iter().any(|(key,value)| *key == record.imdb_id) {
                        self.upsert_record(record, predicate)
                    } else {
                        false
                    }
                }
                ChangeOption::Update => {
                    if self.history.records.iter().any(|(key,value)| *key == record.imdb_id) {
                        self.upsert_record(record, predicate)
                    } else {
                        false
                    }
                }
            }
        }
        change
    }

    // Upserts if predicate returns true, or is None
    fn upsert_record(&mut self,
                     record: Record,
                     predicate: impl Fn(&History, &Record) -> bool) -> bool {
        if predicate(&self.history, &record) {
            match self.history.records.get_mut(&record.imdb_id) {
                Some(_value) => *_value = record,
                None => {self.history.records.insert(record.imdb_id.clone(), record);},
            }
            true
        } else {
            false
        }
    }
    ///////////////////
    //Utility Functions
    ///////////////////

    //adds pmds and returns the ones that were not added/updated
    fn add_pmds(&mut self, pmds: &[plex::PlexMetadata], change_option: ChangeOption, predicate: impl Fn(&History, &Record) -> bool) -> Vec<Record> {
        pmds.iter()
            .map(MediaManager::pmd_to_record)
            .filter(|record| {
                !self.add_record(record.clone(), change_option, &predicate)
            }).collect()
    }

    //Make State Missing if item is in History but not Plex library
    fn set_missing_records(&mut self, pmds: &[plex::PlexMetadata]) {
        self.history.records.iter_mut()
            .filter(
                |(key, value)| {
                    !pmds.iter().any(|pmd| pmd.imdb_guid() == **key)
                })
            .for_each(|(key, value)| value.status = State::Missing);
    }

    //convert a pmd to a movie record. If the pmd exists as downloading, convert it to cleaned
    fn pmd_to_record(pmd: &PlexMetadata) -> Record {
        Record {
            imdb_id: pmd.imdb_guid(),
            title: pmd.title.clone(),
            status: State::Downloaded,
        }
    }
}

// #[cfg(test)]
// mod test {
//     use super::*;
//
//     fn media_manager() -> MediaManager {
//         MediaManager {
//             config: Config {
//                 history_file: "".to_string(),
//                 deluge_url: "".to_string(),
//                 password: "".to_string(),
//                 move_completed_path: "".to_string(),
//                 download_location: "".to_string(),
//                 plex_url: "".to_string(),
//                 plex_token: "".to_string(),
//                 min_file_size: 0,
//                 ideal_file_size: 0,
//                 min_seeders: 0,
//                 target_categories: vec![],
//                 retries: 0,
//                 api_backoff_millis: 0,
//                 list_frequency_millis: 0,
//                 min_imdb_rating: 0,
//                 tmdb_v4_api_key: "".to_string(),
//             },
//             history: History {
//                 records: vec!(
//                     Record { imdb_id: "tt0021749".into(), title: "in pmds".into(), status: State::Cleaned },
//                     Record { imdb_id: "tt0597723552".into(), title: "not in pmds and no magnet".into(), status: State::Cleaned },
//                     Record { imdb_id: "tt0023984212".into(), title: "not in pmds and no magnet and missing".into(), status: State::Missing })
//             },
//             rarbg_token: "".into(),
//         }
//     }
//
//     fn plex_metadata() -> [PlexMetadata; 3] {
//         [
//             plex::PlexMetadata {
//                 title: "in history".into(),
//                 ratingKey: "53462".into(),
//                 guid: "com.plexapp.agents.imdb://tt0021749?lang=en".into(),
//             },
//             plex::PlexMetadata {
//                 title: "not in history".into(),
//                 ratingKey: "09823".into(),
//                 guid: "com.plexapp.agents.imdb://tt423090134?lang=en".into(),
//             },
//             plex::PlexMetadata {
//                 title: "no guid".into(),
//                 ratingKey: "2342145".into(),
//                 guid: "local://3149".into(),
//             }, ]
//     }
//
//     #[test]
//     fn test_missing_records() {
//         let mut manager = media_manager();
//         let pmds = plex_metadata();
//         manager.set_missing_records(&pmds);
//         assert!(matches!(manager.history.records[0].status, State::Downloading));
//         assert!(matches!(manager.history.records[1].status, State::Missing));
//     }
//
//     #[test]
//     fn test_pmds_to_upsert() {
//         assert!(false);
//
//         // let manager = media_manager();
//         // let pmds = plex_metadata();
//         //let upserts = manager.get_new_pmds(&pmds);
//         //assert_eq!(upserts.len(), 2);
//     }
//
//     #[test]
//     fn test_missing_guid() {
//         let pmds = plex_metadata();
//         assert_eq!(pmds[2].imdb_guid(), "");
//     }
//
//     #[test]
//     fn test_clean() {
//         assert!(false);
//
//         // let mut manager = media_manager();
//         // let pmds = plex_metadata();
//         // manager.get_new_pmds(&pmds)
//         //     .iter()
//         //     .copied()
//         //     .map(MediaManager::pmd_to_record)
//         //     .for_each(|record| manager.upsert_record(record));
//         //
//         // manager.clean_library(false);
//         // let expected_records = [
//         //     Record { imdb_id: "tt0021749".into(), title: "in pmds".into(), status: State::Cleaned },
//         //     Record { imdb_id: "tt0597723552".into(), title: "not in pmds and no magnet".into(), status: State::Cleaned },
//         //     Record { imdb_id: "tt423090134".into(), title: "not in history".into(), status: State::Missing },
//         // ];
//         // for (actual, expected) in expected_records.iter().zip(&manager.history.records) {
//         //     assert_eq!(actual.imdb_id, expected.imdb_id);
//         //     assert_eq!(actual.title, expected.title);
//         //     if actual.imdb_id == "tt423090134" {
//         //         assert!(matches!(actual.status, State::Missing));
//         //     } else {
//         //         assert!(matches!(actual.status, State::Cleaned));
//         //     }
//         // }
//     }
// }