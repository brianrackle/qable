use serde::Deserialize;

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

pub fn get_plex_library_guids(url: &str, token: &str) -> Vec<String> {
    let mut result: Vec<String> = Vec::new();
    let path = format!("{}all?X-Plex-Token={}", url, token);
    let resp = ureq::get(path.as_str())
        .set("Content-Type", "application/json")
        .set("Accept", "application/json")
        .call();
    if resp.ok() {
        let response = resp.into_string().unwrap();
        let s: PlexResults = serde_json::from_str(&response).unwrap();
        result = s.MediaContainer.Metadata.iter().map(|x| String::from(&x.guid[26..35]).to_lowercase()).collect();
    }
    result
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn deserialize_plexresults_test() {
        let s: PlexResults = serde_json::from_str(test_string).unwrap();
        assert_eq!(s.MediaContainer.Metadata[0].guid, "com.plexapp.agents.imdb://tt7541106?lang=en");
    }
}