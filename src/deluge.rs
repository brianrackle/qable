fn get_cookie(path: &str, password: &str) -> Option<String> {
    ureq::post(path)
        .set("content-type", "application/json")
        .send_json(serde_json::json!({
                "method":"auth.login",
                "params":[password],
                "id":42}))
        .header("set-cookie").map(|x| { x.to_owned() })
}

pub fn add_torrent(path: &str, password: &str, magnet: &str, download_location: &str, move_completed_path: &str) {
    if let Some(cookie) = get_cookie(path, password) {
        ureq::post(path)
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
                                        "download_location": download_location,
                                        "move_completed_path": move_completed_path,
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
}
