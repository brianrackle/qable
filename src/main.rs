use std::env;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use clap::{App, Arg};
use serde::Deserialize;
use serde_json::{Result, Value};

//o - get imdb list as csv (https://www.imdb.com/list/ls057589528 + /export?ref_=ttls_otexp)
//x - list all movies on plex
//o - look for imdb id
//o - if exists skip (could also do a name/display name check)
//o - if doesnt exist search rarbg.to using imbd id
//o - download torrent if > 3 seeders and closest to 8GB and equal to or more than 4GB
//1 req / 2 sec
//x - get token
//x - https://torrentapi.org/pubapi_v2.php?mode=search&search_imdb=tt0107207&format=json_extended&token=om759bh2yc&app_id=qable

#[derive(Deserialize)]
struct Config {
    path: String,
    password: String,
    move_completed_path: String,
    download_location: String,
    plex_server_library: String,
    plex_token: String,
}

#[derive(Deserialize)]
struct PlexResults {
    MediaContainer: PlexMediaContainer,
}

#[derive(Deserialize)]
struct PlexMediaContainer {
    Metadata: Vec<PlexMetadata>,
}

#[derive(Deserialize)]
struct PlexMetadata {
    guid: String,
}

#[derive(Deserialize)]
struct RarbgToken {
    token: String,
}

#[derive(Deserialize)]
struct RarbgResults {
    torrent_results: Vec<RarbgMagnet>,
}

#[derive(Deserialize)]
struct RarbgMagnet {
    title: String,
    category: String,
    download: String,
    seeders: i32,
    leechers: i32,
    size: i64,
}

fn get_imdb_list(url: String) {
    unimplemented!();
    // let resp = ureq::get(path.as_str())
    //     .set("Content-Type", "application/json")
    //     .set("Accept", "application/json")
    //     .call();
    // if resp.ok() {
    //     let s : Value = serde_json::from_str(&resp.into_string().unwrap()).unwrap();
    //     if let serde_json::Value::Array(t) = &s["MediaContainer"]["Metadata"] {
    //         result = t.iter().map(|x| String::from(&x["guid"].as_str().unwrap()[26..35]) ).collect();
    //     }
    // } else {
    //     result = Vec::new();
    // }
}

fn get_rarbg_token() -> String {
    let mut result: String = String::new();
    let resp = ureq::get("https://torrentapi.org/pubapi_v2.php?get_token=get_token&app_id=qable")
        .set("Content-Type", "application/json")
        .set("Accept", "application/json")
        .call();
    if resp.ok() {
        let s: RarbgToken = serde_json::from_str(&resp.into_string().unwrap()).unwrap();
        result = s.token;
    }
    result
}

//category ends with x264/1080 or x264/720 && seeders >= 3 with size closest to 8589934592 (8 gibibytes)
fn get_rarbg_magnet(imdb_guid: String, token: String) -> String {
    let mut result: String = String::new();
    let path = format!("https://torrentapi.org/pubapi_v2.php?mode=search&search_imdb={}&format=json_extended&token={}&app_id=qable", imdb_guid, token);
    let resp = ureq::get(path.as_str())
        .set("Content-Type", "application/json")
        .set("Accept", "application/json")
        .call();
    if resp.ok() {
        let t = &resp.into_string().unwrap();
        println!("{}", t);
        let results: RarbgResults = serde_json::from_str(t).unwrap();
        let filtered_results = results.torrent_results.iter().filter(|&x|
            (x.category.ends_with("x264/1080") || x.category.ends_with("x264/720")) && x.seeders > 2);
        result = filtered_results.min_by(|x, y| (8589934592 - x.size).cmp(&(8589934592 - y.size))).unwrap().download.clone();
    }

    result
}

fn get_plex_library_guids(url: &str, token: &str) -> Vec<String> {
    let mut result: Vec<String> = Vec::new();
    let path = format!("{}all?X-Plex-Token={}", url, token);
    let resp = ureq::get(path.as_str())
        .set("Content-Type", "application/json")
        .set("Accept", "application/json")
        .call();
    if resp.ok() {
        let s: PlexMediaContainer = serde_json::from_str(&resp.into_string().unwrap()).unwrap();
        result = s.Metadata.iter().map(|x| String::from(&x.guid[26..35])).collect();
    }
    result
}

fn get_cookie(config: &Config) -> Option<String> {
    ureq::post(config.path.as_str())
        .set("content-type", "application/json")
        .send_json(serde_json::json!({
                "method":"auth.login",
                "params":[config.password],
                "id":42}))
        .header("set-cookie").map(|x| { x.to_owned() })
}

fn add_torrent(config: &Config, cookie: &str, magnet: &str) {
    ureq::post(config.path.as_str())
        .set("content-type", "application/json")
        .set("Cookie", cookie)
        .send_json(serde_json::json!({
                    "method":"web.add_torrents",
                    "params":[
                        [
                            {
                                "path": magnet,
                                "options":
                                    {
                                        "download_location": config.download_location,
                                        "move_completed_path": config.move_completed_path,
                                        "file_priorities":[],
                                        "add_paused":false,
                                        "compact_allocation":false,
                                        "move_completed":false,
                                        "max_connections":50,
                                        "max_download_speed":-1,
                                        "max_upload_slots":-1,
                                        "max_upload_speed":-1,
                                        "prioritize_first_last_pieces":false
                                    }
                                }
                            ]
                        ],
                    "id":618}));
}

fn main() {
    let matches = App::new("qable")
        .version("0.1.0")
        .author("Brian Rackle <brian@rackle.me>")
        .about("Queues Torrents")
        .arg(Arg::with_name("magnet")
            .short('m')
            .long("magnet")
            .takes_value(true)
            .about("Magnet link"))
        .arg(Arg::with_name("imdb")
            .short('i')
            .long("imdb")
            .takes_value(true)
            .conflicts_with("magnet")
            .about("imdb guid"))
        .arg(Arg::with_name("unique")
            .short('u')
            .long("unique")
            .takes_value(false)
            .about("Download if item doesn't exist"))
        .get_matches();

    let env = match env::var("QABLE") {
        Err(_) => env::var("HOME").expect("$HOME not defined") + "/.qable/config.json",
        Ok(e) => e,
    };

    let config_path = Path::new(env.as_str());
    let config: Config = match File::open(&config_path) {
        Err(why) => panic!("couldn't open config {}: {}", env, why),
        Ok(file) => serde_json::from_reader(BufReader::new(file)).unwrap(),
    };

    let plex_guids = get_plex_library_guids(&config.plex_server_library, &config.plex_token);


    if let Some(magnet) = matches.value_of("magnet") {
        if let Some(cookie) = get_cookie(&config) {
            add_torrent(&config, cookie.as_str(), magnet);
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

}
