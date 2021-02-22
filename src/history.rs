use crate::{plex, tmdb};
use crate::config::Config;
use std::io::stdin;

pub struct MediaManager {
    movies: plex::Movies,
    config: Config,
    test: bool,
    validate: bool,
}

impl MediaManager {
    pub fn new(config: Config, test :bool, validate :bool) -> MediaManager {
        let pmds = plex::get_plex_library_guids(&config).expect("Exiting (Plex GUIDs Not Found)");
        MediaManager {
            config,
            movies: pmds,
            test,
            validate
        }
    }

    fn is_dirty(plex_title :&str, tmdb_title : &str) -> bool {
        let accumlator = |acc, r :char| if r.is_alphanumeric() {acc} else {acc + 1};
        let plex_special_chars :i8 = plex_title.chars().into_iter().fold(0, accumlator);
        let tmdb_special_chars :i8 = tmdb_title.chars().into_iter().fold(0, accumlator);
        (plex_special_chars - tmdb_special_chars).abs() > 3
    }

    pub fn clean_history(&self) {
        for (imdb_id, plex_metadata) in &self.movies.metadata {
            if let Some(tmdb_title) = tmdb::get_movie_title(&self.config, &imdb_id) {
                if MediaManager::is_dirty(&plex_metadata.title, &tmdb_title) {
                    println!("Renaming {} into {}", plex_metadata.title, tmdb_title);
                    if self.validate {
                        let mut input_string = String::new();
                        stdin().read_line(&mut input_string)
                            .ok()
                            .expect("Failed to read line");
                    }
                    if !self.test {
                        plex::put_plex_movie_metadata(&self.config,
                                                      &plex_metadata.plex_key,
                                                      &tmdb_title)
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn is_dirty() {
        assert!(MediaManager::is_dirty("Eight.and.a.Half.1963.ITALIAN.1080p.BluRay.H264.AAC-VXT", "8½"))
    }
}