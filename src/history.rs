use core::time;
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{BufReader, BufWriter};
use std::path::Path;
use std::thread::sleep;

use serde::{Deserialize, Serialize};

use crate::{deluge, plex, rarbg, tmdb};
use crate::config::Config;

pub struct MediaManager {
    history: History,
    movies: plex::Movies,
    rarbg_token: String,
    pub config: Config,
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
    //Downloading torrent
    Downloading,
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

impl History {
    pub fn is_record_in_state(&self, imdb_id: &str, status: State) -> bool {
        match self.records.get(imdb_id) {
            Some(record) => std::mem::discriminant(&record.status) == std::mem::discriminant(&status),
            None => false,
        }
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
        let rarbg_token = rarbg::get_rarbg_token(&config).unwrap();
        let mut manager = MediaManager {
            config,
            rarbg_token,
            history: History { records: HashMap::new() },
            movies: pmds,
        };
        //init is overwriting existing history with plex
        manager.init_history();
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
                       imdb_id: &str) -> bool {
        let mut success = false;
        if let Some(title) = tmdb::get_movie_title(&self.config, imdb_id) {
            if self.history.records.get(&imdb_id.to_lowercase()).is_none() {
                if let Some(magnet) = rarbg::get_rarbg_magnet(&self.config, &self.rarbg_token, &imdb_id) {
                    deluge::add_torrent(&self.config, &magnet);
                    self.history.records.insert(imdb_id.into(),
                                                Record {
                                                    imdb_id: imdb_id.to_string(),
                                                    title: title.clone(),
                                                    status: State::Downloading,
                                                });
                    println!("Downloading {}: \"{}\"", &imdb_id, &title);
                    success = true;
                    sleep(time::Duration::from_millis(self.config.list_frequency_millis));
                }
            }
        }
        if !success {
            println!("Failed to download {}", imdb_id);
        }
        success
    }

    fn init_history(&mut self) {
        //(imdb_id, Some(State), Some(title)
        let mut changes: HashMap<String, (State, String)> = Default::default();

        match File::open(&self.config.history_file) {
            Ok(file) => {
                let reader = BufReader::new(file);
                self.history = serde_json::from_reader(reader)
                    .expect(&format!("Unable to deserialize history {}", &self.config.history_file));

                //if torrent can be downloaded, lookup the correct title and update it in history as downloading
                //if torrent cant be downloaded, change it to missing
                let in_history_only =
                    self.history.records.iter().filter(|(key, _record)| {
                        !self.movies.metadata.contains_key(*key)
                    });
                for (imdb_id, _record) in in_history_only {
                    if let Some(title) = tmdb::get_movie_title(&self.config, imdb_id) {
                        if let Some(magnet) = rarbg::get_rarbg_magnet(&self.config, &self.rarbg_token, &imdb_id) {
                            deluge::add_torrent(&self.config, &magnet);
                            changes.insert(imdb_id.clone(),
                                           (
                                               State::Downloading,
                                               title.clone(),
                                           ));
                        }
                    }
                }

                //insert into history, lookup and set the correct title and add it to history as cleaned
                let in_movies_only =
                    self.movies.metadata.iter().filter(|(key, metadata)| {
                        !self.history.records.contains_key(*key)
                    });
                for (imdb_id, metadata) in in_movies_only {
                    if let Some(title) = tmdb::get_movie_title(&self.config, imdb_id) {
                        plex::put_plex_movie_metadata(&self.config,
                                                      &metadata.plex_key,
                                                      &title);
                        changes.insert(imdb_id.clone(),
                                       (
                                           State::Cleaned,
                                           title.clone(),
                                       ));
                    }
                }

                //set the correct title update it in history as cleaned
                let in_history_and_movies =
                    self.history.records.iter().filter(|(key, record)| {
                        !matches!(record.status, State::Cleaned) && self.movies.metadata.contains_key(*key)
                    });
                for (imdb_id, record) in in_history_and_movies {
                    plex::put_plex_movie_metadata(&self.config,
                                                  &self.movies.metadata[imdb_id].plex_key,
                                                  &record.title);
                    changes.insert(imdb_id.clone(),
                                   (
                                       State::Cleaned,
                                       record.title.clone(),
                                   ));
                }
            }
            Err(..) => {
                //create new history from scratch,
                //get correct titles, add them to plex, and create clean records
                for imdb_id in self.movies.metadata.keys() {
                    if let Some(title) = tmdb::get_movie_title(&self.config, &imdb_id) {
                        plex::put_plex_movie_metadata(&self.config, &imdb_id, &title);
                        changes.insert(imdb_id.clone(),
                                       (
                                           State::Cleaned,
                                           title.clone(),
                                       ));
                    }
                }
            }
        }
        for (imdb_id, (status, title)) in changes {
            self.history.records.insert(imdb_id.clone(),
                                        Record {
                                            imdb_id,
                                            title,
                                            status,
                                        });
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