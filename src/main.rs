use std::env;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use clap::{App, Arg};
use serde::Deserialize;


#[derive(Deserialize)]
struct Config {
    path: String,
    password: String,
    move_completed_path: String,
    download_location: String,
}

//take screen shot of movie poster, lookup movie name, find torrent and download
fn get_cookie(config: &Config) -> Option<String> {
    let resp = ureq::post(config.path.as_str())
        .set("content-type", "application/json")
        .send_json(serde_json::json!({
                "method":"auth.login",
                "params":[config.password],
                "id":42}));

    resp.header("set-cookie").map(|x| { x.to_owned() })
}

fn add_torrent(config: &Config, cookie: String, magnet: String) {
    ureq::post(config.path.as_str())
        .set("content-type", "application/json")
        .set("Cookie", cookie.as_str())
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
        .get_matches();

    let env = env::var("QABLE").unwrap();
    let config_path = Path::new(env.as_str());
    let config = match File::open(&config_path) {
        Err(why) => panic!("couldn't open config: {}", why),
        Ok(file) => serde_json::from_reader(BufReader::new(file)).unwrap(),
    };

    if let Some(magnet) = matches.value_of("magnet") {
        if let Some(cookie) = get_cookie(&config) {
            add_torrent(&config, cookie, magnet.into());
        }
    }
}

