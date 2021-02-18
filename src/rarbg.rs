use serde::Deserialize;

use crate::config::Config;
use crate::request::get_response_data;

#[derive(Deserialize)]
struct RToken {
    token: String,
}

#[derive(Deserialize)]
struct RResults {
    torrent_results: Vec<RMagnet>,
}

#[derive(Deserialize)]
struct RMagnet {
    title: String,
    category: String,
    download: String,
    seeders: i32,
    leechers: i32,
    size: i64,
}

pub fn get_rarbg_token(config: &Config) -> Option<String> {
    get_response_data("https://torrentapi.org/pubapi_v2.php?get_token=get_token&app_id=qable",
                      &[("Content-Type", "application/json"), ("Accept", "application/json")],
                      &[],
                      config.api_backoff_millis,
                      config.retries,
                      |response| -> (bool, Option<String>) {
                          match serde_json::from_str::<RToken>(&response.into_string().unwrap()) {
                              Err(_) => (false, None),
                              Ok(token_result) => {
                                  (true, Some(token_result.token))
                              }
                          }
                      })
}

fn filter_magnets<'a>(config: &Config, results: &'a RResults) -> Vec<&'a RMagnet> {
    let mut magnets: Vec<&RMagnet> = results.torrent_results
        .iter()
        .filter(|&magnet|
            config.target_categories.iter().any(|category| magnet.category.ends_with(category))
                && magnet.size > config.min_file_size).collect();
    // determine min seeders
    let mut max_index = 0usize;
    for i in 0..config.seeders.len() {
        let rule = &config.seeders[i];
        let magnet_count = magnets
            .iter()
            .filter(|magnet| magnet.seeders >= rule.min_seeders.into()).count();
        if magnet_count >= rule.available_magnets.into() {
            max_index = i;
        } else {
            break;
        }
    }
    magnets.retain(|magnet| magnet.seeders >= config.seeders[max_index].min_seeders.into());
    magnets
}

fn match_magnet<'a>(config: &Config, magnets: &'a [&RMagnet]) -> &'a RMagnet {
    magnets.iter().min_by(|left, right| (config.ideal_file_size - left.size).abs().cmp(&(config.ideal_file_size - right.size).abs())).unwrap()
}

//TODO: log to file the list/imdb id/magnet, and details about each step
pub fn get_rarbg_magnet(config: &Config, token: &str, imdb_guid: &str) -> Option<String> {
    get_response_data(&format!("https://torrentapi.org/pubapi_v2.php?mode=search&search_imdb={}&format=json_extended&token={}&app_id=qable", imdb_guid, token),
                      &[("Content-Type", "application/json"), ("Accept", "application/json")],
                      &[],
                      config.api_backoff_millis,
                      config.retries,
                      |response| -> (bool, Option<String>) {
                          match serde_json::from_str::<RResults>(&response.into_string().unwrap()) {
                              Err(_) => (false, None),
                              Ok(search_results) => {
                                  let filtered_magnets = filter_magnets(&config, &search_results);
                                  if !filtered_magnets.is_empty() {
                                      let magnet_match = match_magnet(&config, &filtered_magnets);
                                      (true, Some(magnet_match.download.clone()))
                                  } else {
                                      (false, None)
                                  }
                              }
                          }
                      })
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn deserialize_rarbgtoken_test() {
        let test_string = r#"{"token":"omsdfh2yc"}"#;
        let s: RToken = serde_json::from_str(test_string).unwrap();
        assert_eq!(s.token, "omsdfh2yc");
    }

    #[test]
    fn rarbg_magnet_test() {
        let test_string = r#"{"torrent_results":[{"title":"Back.to.the.Future.1985.iNTERNAL.1080p.BluRay.x264-iLLUSiON","category":"Movies/x264/1080","download":"magnet:?xt=urn:btih:6f53fd2643bc2bc77728fcdcd58af1cac83b382c&dn=Back.to.the.Future.1985.iNTERNAL.1080p.BluRay.x264-iLLUSiON&tr=http%3A%2F%2Ftracker.trackerfix.com%3A80%2Fannounce&tr=udp%3A%2F%2F9.rarbg.me%3A2710&tr=udp%3A%2F%2F9.rarbg.to%3A2710&tr=udp%3A%2F%2Fopen.demonii.com%3A1337%2Fannounce","seeders":13,"leechers":3,"size":9387148339,"pubdate":"2018-12-19 16:28:13 +0000","episode_info":{"imdb":"tt0088763","tvrage":null,"tvdb":null,"themoviedb":"105"},"ranked":1,"info_page":"https://torrentapi.org/redirect_to_info.php?token=om759bh2yc&p=1_7_1_8_2_2_1__6f53fd2643"}]}"#;
        let s: RResults = serde_json::from_str(test_string).unwrap();
        assert_eq!(s.torrent_results[0].download, "magnet:?xt=urn:btih:6f53fd2643bc2bc77728fcdcd58af1cac83b382c&dn=Back.to.the.Future.1985.iNTERNAL.1080p.BluRay.x264-iLLUSiON&tr=http%3A%2F%2Ftracker.trackerfix.com%3A80%2Fannounce&tr=udp%3A%2F%2F9.rarbg.me%3A2710&tr=udp%3A%2F%2F9.rarbg.to%3A2710&tr=udp%3A%2F%2Fopen.demonii.com%3A1337%2Fannounce");
    }
}