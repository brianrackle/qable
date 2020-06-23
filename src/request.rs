use std::borrow::{Borrow, BorrowMut};
use std::thread::sleep;
use std::time;
use ureq::Response;

pub fn get_response(url: &str, headers: &[(&str, &str)]) -> Response {
    let mut get = ureq::get(url);
    for header in headers {
        get.set(header.0, header.1);
    }
    get.call()
}

//performs a get request with the given url and headers
//executes passed in function for OK response handling
//ok_property: fn(r:&T) -> String
// pub fn get_response_body<'a, T: serde::Deserialize<'a>>(url: &str, headers: &[(&str, &str)],
//                                                         api_backoff_millis: u64, retries: i32) -> Option<String> {
//     let mut response_body: Option<String> = None;
//     let mut complete = false;
//     let mut backoff = api_backoff_millis;
//     let mut attempts = 0;
//
//     while !complete {
//         attempts += 1;
//         let mut get = ureq::get(url);
//         for header in headers {
//             get.set(header.0, header.1);
//         }
//         let response = get.call();
//         if response.ok() {
//             // let t = response.into_string();
//             if let Ok(_) = serde_json::from_str::<T>(&response.into_string().unwrap()) {
//                 complete = true;
//                 //response_body = Some(ok_property(&deserialized_response));
//             }
//         }
//         if !complete {
//             if attempts >= retries {
//                 complete = true;
//             }
//             backoff += api_backoff_millis;
//             sleep(time::Duration::from_millis(backoff));
//         }
//     }
//     response_body
// }