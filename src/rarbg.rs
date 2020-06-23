use std::thread::sleep;
use std::time;

use serde::Deserialize;

use crate::config::Config;

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


fn get_rarbg_token() -> String {
    let resp = ureq::get("https://torrentapi.org/pubapi_v2.php?get_token=get_token&app_id=qable")
        .set("Content-Type", "application/json")
        .set("Accept", "application/json")
        .call();
    //TODO: add retry
    serde_json::from_str::<RToken>(&resp.into_string().unwrap()).unwrap().token
}

fn filter_magnets<'a>(config: &Config, results: &'a RResults) -> Vec<&'a RMagnet> {
    results.torrent_results.iter().filter(|&magnet|
        config.target_categories.iter().any(|category| magnet.category.ends_with(category))
            && magnet.seeders >= config.min_seeders && magnet.size > config.min_file_size).collect()
}

fn match_magnet<'a>(config: & Config, magnets: &'a[&RMagnet]) -> &'a RMagnet {
    magnets.iter().min_by(|left, right| (config.ideal_file_size - left.size).abs().cmp(&(config.ideal_file_size - right.size).abs())).unwrap()
}

//TODO: log to file the list/imdb id/magnet, and details about each step
pub fn get_rarbg_magnet(config: &Config, imdb_guid: &str) -> Option<String> {
    let token = get_rarbg_token();

    let mut result: Option<String> = None;
    let mut complete = false;
    let mut backoff = config.api_backoff_millis;
    let mut attempts = 0;
    while !complete {
        attempts += 1;
        let path = format!("https://torrentapi.org/pubapi_v2.php?mode=search&search_imdb={}&format=json_extended&token={}&app_id=qable", imdb_guid, token);
        let resp = ureq::get(path.as_str())
            .set("Content-Type", "application/json")
            .set("Accept", "application/json")
            .call();
        if resp.ok() {
            if let Ok(search_results) = serde_json::from_str::<RResults>(&resp.into_string().unwrap()) {
                complete = true;
                //TODO: if filtered magnets is 0 change filter params to get a match?
                let filtered_magnets = filter_magnets(&config, &search_results);
                if !filtered_magnets.is_empty() {
                    let magnet_match = match_magnet(&config, &filtered_magnets);
                    result = Some(magnet_match.download.clone());
                }
            }
        }
        if !complete {
            if attempts >= config.retries {
                complete = true;
            }
            backoff += config.api_backoff_millis;
            sleep(time::Duration::from_millis(backoff));
        }
    }
    result
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