use serde::Deserialize;

use crate::config::Config;
use crate::request;

#[derive(Deserialize)]
struct FindResponse {
    movie_results: Vec<FindMovieResults>
}

#[derive(Deserialize)]
struct FindMovieResults {
    id: i32,
    title: String,
}

#[derive(Deserialize)]
struct MovieDetails {
    adult: bool,
    budget: i32,
    // genres: [{
    //         "id": 878,
    //         "name": "Science Fiction"
    //         }
    //         ],
    id: i32,
    imdb_id: String,
    original_language: String,
    original_title: String,
    overview: String,
    popularity: f32,
    poster_path: String,
    // "production_companies": [
    // {
    // "id": 490,
    // "logo_path": null,
    // "name": "New Regency Productions",
    // "origin_country": "US"
    // }
    // ],
    // "production_countries": [
    // {
    // "iso_3166_1": "US",
    // "name": "United States of America"
    // }
    // ],
    release_date: String, //"2019-09-17",
    revenue: i32,
    runtime: i32,
    // "spoken_languages": [
    // {
    // "iso_639_1": "en",
    // "name": "English"
    // },
    // ],
    status: String, // "Released",
    tagline: String,
    title: String,
    vote_average: f32,
    vote_count: i32,
}

pub fn get_movie_title(config: &Config, imdb_id: &str) -> Option<String> {
    request::get_response_data(&format!("https://api.themoviedb.org/3/find/{}", imdb_id),
                      &[
                          ("Authorization", &format!("Bearer {}", config.tmdb_v4_api_key)),
                          ("Content-Type", "application/json;charset=utf-8"),
                          ("Accept", "application/json")
                      ],
                      &[
                          ("language", "en-US"),
                          ("external_source", "imdb_id")
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

//use to get extra filtering details see movie details struct
pub fn get_movie_details(config: &Config, tmdb_id: i32) {
    request::get_response_data(&format!("https://api.themoviedb.org/3/movie/{}", tmdb_id),
                      &[
                          ("Authorization", &format!("Bearer {}", config.tmdb_v4_api_key)),
                          ("Content-Type", "application/json;charset=utf-8"),
                          ("Accept", "application/json")
                      ],
                      &[
                          ("language", "en-US"),
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
                      });
}