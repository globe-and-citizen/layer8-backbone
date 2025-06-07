use async_trait::async_trait;
use bytes::Bytes;
use chrono::Utc;
use env_logger::{Env, Target};
use jsonwebtoken::{EncodingKey, Header, encode};
use log::{info, trace};
use pingora_core::prelude::*;
use pingora_error::{ErrorType, Result};
use pingora_http::{RequestHeader, ResponseHeader};
use pingora_proxy::{ProxyHttp, Session};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::env;

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
            String::from("localhost:6193"),
            true,
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
                .get(format!(
                    "{}{}?backend_url={}",
                    LAYER8_URL, "/sp-pub-key", backend_url
                ))
                .header("Authorization", format!("Bearer {}", token))
                .send()
                .await
                .unwrap();

            println!("=---------------------");
            println!("THIS IS OK");
            println!("=---------------------");

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
    // let log_file = fs::File::create("log.txt").expect("Failed to create log file");
    // let config = ConfigBuilder::new().set_time_to_local(true).build();
    // WriteLogger::init(LevelFilter::Debug, config, log_file).expect("Failed to initialize logger");
    env_logger::Builder::from_env(Env::default().write_style_or("RUST_LOG_STYLE", "always"))
        .format_file(true)
        .format_line_number(true)
        .target(Target::Stdout)
        .init();

    info!("Starting server...");
    let mut server = Server::new(Some(Opt {
        conf: Some(format!("{}/server_conf.yml", env!("CARGO_MANIFEST_DIR"))),
        ..Default::default()
    }))
    .unwrap();
    server.bootstrap();

    let mut proxy = pingora_proxy::http_proxy_service(&server.configuration, ForwardProxy);

    // testing certs data; fixme to be dynamic
    {
        let server_pem = format!(
            "{}/../certs/generated/forward_proxy.pem",
            env!("CARGO_MANIFEST_DIR")
        );
        let server_key = format!(
            "{}/../certs/generated/forward_proxy-key.pem",
            env!("CARGO_MANIFEST_DIR")
        );

        let mut tls_settings =
            pingora_core::listeners::tls::TlsSettings::intermediate(&server_pem, &server_key)
                .unwrap();
        tls_settings.enable_h2();
        proxy.add_tls_with_settings("localhost:6191", None, tls_settings);
        info!("Proxy service added with TLS endpoint on");
    }

    server.add_service(proxy);
    server.run_forever();
}
