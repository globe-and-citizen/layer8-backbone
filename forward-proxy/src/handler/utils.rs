use std::collections::HashMap;
use chrono::Utc;
use jsonwebtoken::{encode, EncodingKey, Header};
use reqwest::header::HeaderMap;
use serde::{Deserialize, Serialize};

// Get SECRET_KEY from environment variable
pub fn get_secret_key() -> String {
    std::env::var("JWT_SECRET_KEY").expect("JWT_SECRET_KEY must be set")
}

#[derive(Serialize)]
struct Claims {
    exp: usize,
}

pub fn generate_standard_token(secret_key: &str) -> pingora::Result<String, Box<dyn std::error::Error>> {
    let now = Utc::now();
    let claims = Claims {
        exp: (now + chrono::Duration::days(1)).timestamp() as usize,
    };

    let token = encode(
        &Header::new(jsonwebtoken::Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(secret_key.as_bytes()),
    )?;

    Ok(token)
}

pub fn bytes_to_json<T: Serialize + for<'de> Deserialize<'de>>(bytes: Vec<u8>) -> Result<T, serde_json::Error> {
    serde_json::from_slice::<T>(&bytes)
}

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