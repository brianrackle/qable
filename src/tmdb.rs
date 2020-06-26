use serde::Deserialize;

use crate::config::Config;
use crate::request::get_response_body;

#[derive(Deserialize)]
struct FindResponse {
    movie_results: Vec<FindMovieResults>
}

#[derive(Deserialize)]
struct FindMovieResults {
    id: i32,
    title: String,
}

pub fn get_movie_title(config: &Config, imdb_id: &str) -> Option<String> {
    get_response_body(&format!("https://api.themoviedb.org/3/find/{}", imdb_id),
                      &[
                          ("Authorization", &format!("Bearer {}",config.tmdb_v4_api_key)),
                          ("Content-Type", "application/json;charset=utf-8"),
                          ("Accept", "application/json")
                      ],
                      &[
                          ("language","en-US"),
                          ("external_source","imdb_id")
                      ],
                      config.api_backoff_millis,
                      config.retries,
                      |response| -> (bool, Option<String>) {
                          match serde_json::from_str::<FindResponse>(&response.into_string().unwrap()) {
                              Err(_) => (false, None),
                              Ok(find_results) => {
                                  if find_results.movie_results.len() == 1 {
                                      (true, find_results.movie_results.first().map(|x| x.title.clone()))
                                  } else {
                                      (false, None)
                                  }
                              }
                          }
                      })
}