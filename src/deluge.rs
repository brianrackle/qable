use serde::Deserialize;

use crate::config::Config;
use crate::request;

use ureq::json;

//TODO: request should include a random id
#[derive(Deserialize)]
pub struct Response<T> {
    id: String,
    result: Option<T>,
    error: Option<Error>,
}

#[derive(Deserialize)]
pub struct Error {
    message: String,
    code: i32,
}

type RemoveTorrentResult = bool;

#[derive(Deserialize)]
pub struct MagnetInfoResult {
    files_tree: String,
    name: String,
    info_hash: String,
}

pub fn get_cookie(config: &Config) -> Option<String> {
    ureq::post(&config.deluge_url)
        .set("content-type", "application/json")
        .send_json(serde_json::json!({
                "method":"auth.login",
                "params":[&config.password],
                "id":42}))
        .header("set-cookie").map(|x| { x.to_owned() })
}

pub fn add_torrent(config: &Config, cookie: &String, magnet: &str) {
    ureq::post(&config.deluge_url)
        .set("content-type", "application/json")
        .set("Cookie", &cookie)
        .send_json(serde_json::json!({
                    "method":"web.add_torrents",
                    "params":[
                        [
                            {
                                "path": magnet,
                                "options":
                                    {
                                        "download_location": &config.download_location,
                                        "move_completed_path": &config.move_completed_path,
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
                    "id":42}));
}

pub fn remove_torrent(config: &Config, cookie: &String, magnet: &str) -> bool {
    let info_response = request::post_response(&config.deluge_url,
                           &[("content-type", "application/json"), ("Cookie", &cookie)],
                           &[],
                           json!({
                                "method":"web.get_magnet_info",
                                "params": [magnet],
                                "id":42
                            }));

    let info_result = serde_json::from_str::<Response<MagnetInfoResult>>(&info_response.into_string().unwrap())
        .expect("Error unpacking magnet response")
        .result
        .expect("Error unpacking magnet info");
    let info_hash = info_result.info_hash;

    let delete_response = request::post_response(&config.deluge_url,
                                          &[("content-type", "application/json"), ("Cookie", &cookie)],
                                          &[],
                                          json!({
                                "method":"core.remove_torrent",
                                "params": [info_hash],
                                "id":42
                            }));

    //return false if removal is unsuccessful
    //poll deluge torrents every 60 seconds
    //poll plex every 60 seconds
    //if completed torrent is found in plex
    //then remove completed torrent from deluge
    //and clean plex entry
    let delete_result = serde_json::from_str::<Response<RemoveTorrentResult>>(&delete_response.into_string().unwrap())
        .expect("Error unpacking magnet response")
        .result
        .expect("Error unpacking magnet info");

    let info_hash = delete_result;
    true
    //use response to get torrent id and remove the torrent
}
