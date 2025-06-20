use std::collections::HashMap;
use reqwest::header::HeaderMap;
use uuid::Uuid;

use serde::{Deserialize, Serialize};

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

pub fn new_uuid() -> String {
    Uuid::new_v4().to_string()
}

pub fn vec_to_json(vec: Vec<u8>) -> String {
    serde_json::to_string(&vec).unwrap()
}

pub fn json_to_vec(json: &str) -> Vec<u8> {
    serde_json::from_str(json).unwrap()
}

pub fn string_to_array32(s: String) -> Option<[u8; 32]> {
    let bytes = s.into_bytes();
    if bytes.len() == 32 {
        Some(bytes.try_into().unwrap())
    } else {
        None
    }
}

pub fn bytes_to_json<T: Serialize + for<'de> Deserialize<'de>>(bytes: Vec<u8>) -> Result<T, serde_json::Error> {
    serde_json::from_slice::<T>(&bytes)
}

pub fn bytes_to_string(bytes: &Vec<u8>) -> String {
    String::from_utf8_lossy(bytes).to_string()
}
