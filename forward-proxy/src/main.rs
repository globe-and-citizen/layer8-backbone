use async_trait::async_trait;
use log::info;
use pingora_core::prelude::*;
use pingora_error::{ErrorType, Result};
use pingora_proxy::{ProxyHttp, Session};
use simplelog::{ConfigBuilder, LevelFilter, WriteLogger};
use std::fs;
use serde::{Deserialize, Serialize};
use jsonwebtoken::{encode, Header, EncodingKey};
use chrono::Utc;
use reqwest::Client;

const LAYER8_URL: &str = "http://127.0.0.1:5001";
struct ForwardProxy;

// Get SECRET_KEY from environment variable
fn get_secret_key() -> String {
    let secret_key = std::env::var("JWT_SECRET_KEY").expect("JWT_SECRET_KEY must be set");
    secret_key
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ResponseBody {
    data: String,
}

#[derive(Serialize)]
struct Claims {
    exp: usize,
}

#[async_trait]
impl ProxyHttp for ForwardProxy {
    type CTX = ();

    fn new_ctx(&self) -> Self::CTX {}

    async fn upstream_peer(
        &self,
        _session: &mut Session,
        _ctx: &mut Self::CTX,
    ) -> Result<Box<pingora_core::upstreams::peer::HttpPeer>> {
        Ok(Box::from(HttpPeer::new(
            String::from("127.0.0.1:6193"),
            false,
            String::from(""),
        )))
    }

    async fn request_filter(&self, session: &mut Session, _ctx: &mut Self::CTX) -> Result<bool>
    where
        Self::CTX: Send + Sync,
    {
        let request_url = session.req_header().uri.to_string();
        if request_url.contains("init-tunnel") {
            let query_params = session.req_header().uri.query();
            let params: Vec<&str> = query_params.unwrap().split("=").collect();
            let backend_url = params.get(1).unwrap();
            let secret_key = get_secret_key();
            let token = generate_standard_token(&secret_key).unwrap();
            let client = Client::new();
            println!("token: {}", token);
            let res = client
                .get(format!("{}{}?backend_url={}", LAYER8_URL, "/sp-pub-key", backend_url))
                .header("Authorization", format!("Bearer {}", token))
                .send()
                .await
                .unwrap();
            println!("res status: {}", res.status().as_u16());
            // res status will either be: 500 or 401 or 200
            if res.status().as_u16() != 200 {
                // NOTE: error message not printing
                return Err(pingora_core::Error::explain(
                    ErrorType::ConnectProxyFailure,
                    format!(
                        "Failed to get public key from layer8, status code: {}",
                        res.status().as_u16()
                    ),
                ));
            } else {
                // FOR LATER: Return public key here
                // FOR NOW: return here a specific message
                println!("Backend is registered");
            }
        }
        Ok(false)
    }

    async fn response_filter(
        &self,
        _session: &mut Session,
        upstream_response: &mut pingora_http::ResponseHeader,
        _ctx: &mut Self::CTX,
    ) -> Result<()> {
        upstream_response.insert_header("Access-Control-Allow-Origin", "*")?;
        upstream_response.insert_header("Access-Control-Allow-Methods", "GET, POST")?;
        upstream_response.insert_header("Access-Control-Allow-Headers", "Content-Type")?;
        Ok(())
    }
}

fn generate_standard_token(secret_key: &str) -> Result<String, Box<dyn std::error::Error>> {
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

fn main() {
    // Load environment variables from .env file
    dotenv::dotenv().ok();
    // Initialize logger
    let log_file = fs::File::create("log.txt").expect("Failed to create log file");
    let config = ConfigBuilder::new().set_time_to_local(true).build();
    WriteLogger::init(LevelFilter::Debug, config, log_file).expect("Failed to initialize logger");

    info!("Starting server...");

    let mut server = Server::new(None).unwrap();
    server.bootstrap();

    let mut proxy = pingora_proxy::http_proxy_service(&server.configuration, ForwardProxy);

    proxy.add_tcp("0.0.0.0:6191");

    server.add_service(proxy);

    server.run_forever();
}
