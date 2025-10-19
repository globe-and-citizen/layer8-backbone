pub mod jwt;
pub mod cert;
pub mod deserializer;
pub mod log;

use url::Url;

use std::collections::HashMap;
use base64::Engine;
use base64::engine::general_purpose;
use uuid::Uuid;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};

use serde::{Deserialize, Serialize};
use tracing::error;

pub fn to_reqwest_header(map: HashMap<String, String>) -> HeaderMap {
    let mut header_map = HeaderMap::new();
    for (k, v) in map {
        if let Ok(header_name) = HeaderName::try_from(k.as_str()) {
            if let Ok(header_value) = HeaderValue::from_str(&v) {
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

pub fn bytes_to_json<T>(bytes: Vec<u8>) -> Result<T, serde_json::Error>
where
    T: Serialize + for<'de> Deserialize<'de>,
{
    serde_json::from_slice::<T>(&bytes)
}

pub fn bytes_to_string(bytes: &Vec<u8>) -> String {
    String::from_utf8_lossy(bytes).to_string()
}

// String to HeaderMap
pub fn string_to_headermap(s: &str) -> Result<HeaderMap, Box<dyn std::error::Error>> {
    let pairs: Vec<(String, String)> = serde_json::from_str(s)?;
    let mut headers = HeaderMap::new();
    for (k, v) in pairs {
        let name = HeaderName::from_bytes(k.as_bytes())?;
        let value = HeaderValue::from_str(&v)?;
        headers.insert(name, value);
    }
    Ok(headers)
}

// HeaderMap to String
pub fn headermap_to_string(headers: &HeaderMap) -> String {
    let pairs: Vec<(String, String)> = headers.iter()
        .map(
            |(k, v)| (
                k.to_string(),
                v.to_str().unwrap_or("").to_string()
            )
        )
        .collect();
    serde_json::to_string(&pairs).unwrap()
}

// http::header::value::HeaderValue to serde_json::Value
fn headervalue_to_json(val: &HeaderValue) -> serde_json::Value {
    match val.to_str() {
        Ok(s) => serde_json::Value::String(s.to_string()),
        Err(_) => serde_json::Value::String(general_purpose::STANDARD.encode(val.as_bytes())),
    }
}

// serde_json::Value to http::header::value::HeaderValue
fn json_to_headervalue(
    val: &serde_json::Value
) -> Result<HeaderValue, reqwest::header::InvalidHeaderValue>
{
    match val {
        serde_json::Value::String(s) => HeaderValue::from_str(s),
        _ => HeaderValue::from_str(&val.to_string()),
    }
}

pub fn hashmap_to_headermap(
    map: &HashMap<String, serde_json::Value>
) -> Result<HeaderMap, Box<dyn std::error::Error>>
{
    let mut headers = HeaderMap::new();
    for (k, v) in map {
        let name = HeaderName::from_bytes(k.as_bytes())?;
        let value = json_to_headervalue(v)
            .map_err(|e| {
                error!("Invalid header value for '{}': {}", k, e);
            })
            .unwrap_or_else(|_| HeaderValue::from_str("").unwrap());
        headers.insert(name, value);
    }
    Ok(headers)
}

pub fn headermap_to_hashmap(headers: &HeaderMap) -> HashMap<String, serde_json::Value> {
    let mut map = HashMap::new();
    for (k, v) in headers.iter() {
        let key = k.as_str().to_string();
        let value = headervalue_to_json(v);
        map.insert(key, value);
    }
    map
}

/// Validates the input string as a URL and returns the parsed `Url` if valid.
/// Returns `None` if the URL is invalid.
pub fn validate_url(url: &str) -> Option<Url> {
    Url::parse(url).ok()
}

pub fn get_socket_addrs(url: &Url) -> String {
    url.socket_addrs(|| None).unwrap_or_default()
        .iter()
        .map(|addr| addr.to_string())
        .collect::<Vec<String>>()
        .join(",")
}