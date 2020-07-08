use serde::Deserialize;

#[derive(Deserialize)]
struct ImdbRow {
    Position: String,
    Const: String,
}

pub fn get_imdb_list(list: &str) -> Vec<String> {
    let path = format!("https://www.imdb.com/list/{}/export?ref_=ttls_otexp", list);
    let resp = ureq::get(path.as_str()).call();
    let csv = resp.into_string().unwrap_or_else(|_| String::new());
    let mut result: Vec<String> = Vec::new();
    let mut rdr = csv::Reader::from_reader(csv.as_bytes());
    for line in rdr.deserialize::<ImdbRow>() {
        if let Ok(imdb_row) = line {
            result.push(imdb_row.Const);
        }
    }
    result
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn deserialize_imdb_list() {
        let list = get_imdb_list("ls057163861");
        assert_eq!(list[0], String::from("tt0137523"));
    }
}