use std::collections::HashMap;
use reqwest::header::HeaderMap;

pub fn to_reqwest_header(map: HashMap<String, String>) -> HeaderMap {
    let mut header_map = HeaderMap::new();
    for (k, v) in map {
        if let Ok(header_name) = reqwest::header::HeaderName::try_from(k.as_str()) {
            if let Ok(header_value) = reqwest::header::HeaderValue::from_str(&v) {
                header_map.insert(header_name, header_value);
            }
        }
    }
    header_map
}