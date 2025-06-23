use serde::Serialize;
use chrono::Utc;
use jsonwebtoken::{encode, EncodingKey, Header};

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
