use std::borrow::{Borrow, BorrowMut};
use std::thread::sleep;
use std::time;

use ureq::Response;

pub fn put_response(url: &str,
                    headers: &[(&str, &str)],
                    queries: &[(&str, &str)]) -> Response {
    let mut put = ureq::put(url);
    for header in headers {
        put.set(header.0, header.1);
    }
    for query in queries {
        put.query(query.0, query.1);
    }
    put.call()
}

fn get_response(url: &str,
                headers: &[(&str, &str)],
                queries: &[(&str, &str)]) -> Response {
    let mut get = ureq::get(url);
    for header in headers {
        get.set(header.0, header.1);
    }
    for query in queries {
        get.query(query.0, query.1);
    }
    get.call()
}

pub fn get_response_data<T>(url: &str,
                               headers: &[(&str, &str)],
                               query: &[(&str, &str)],
                               api_backoff_millis: u64,
                               retries: i8, ok_handler: impl Fn(Box<Response>) -> (bool, Option<T>))
    -> Option<T> {
    let mut data: Option<T> = None;
    let mut complete = false;
    let mut backoff = api_backoff_millis;
    let mut attempts = 0;

    while !complete {
        attempts += 1;
        let response = get_response(url, headers, query);
        if response.ok() {
            //TODO: destructure response when it's supported
            let r = ok_handler(Box::new(response));
            complete = r.0;
            data = r.1;
        }
        if !complete {
            if attempts >= retries {
                complete = true;
            }
            backoff += api_backoff_millis;
            sleep(time::Duration::from_millis(backoff));
        }
    }
    data
}