
use async_trait::async_trait;
use bytes::Bytes;
use chrono::Utc;
use jsonwebtoken::{EncodingKey, Header, encode};
use log::info;
use pingora_core::prelude::*;
use pingora_error::{ErrorType, Result};
use pingora_proxy::{ProxyHttp, Session};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use simplelog::{ConfigBuilder, LevelFilter, WriteLogger};
use std::collections::HashMap;
use std::fs;
use tokio::time::Duration;

const LAYER8_URL: &str = "http://127.0.0.1:5001";
const RP_URL: &str = "http://127.0.0.1:6193";
const RP_PORT: u16 = 6193;


struct ForwardProxy;

fn get_secret_key() -> String {
    std::env::var("JWT_SECRET_KEY").expect("JWT_SECRET_KEY must be set")
}

#[derive(Serialize, Deserialize, Debug)]
struct ResponseBody {
    data: String,
}

#[derive(Serialize)]
struct Claims {
    exp: usize,
}

#[derive(Serialize, Deserialize, Debug)]
struct FpRequestBodyInit {
    fp_request_body_init: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct FpResponseBodyInit {
    fp_response_body_init: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct FpRequestBodyProxied {
    fp_request_body_proxied: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct FpResponseBodyProxied {
    fp_response_body_proxied: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct FpHealthcheckSuccess {
    fp_healthcheck_success: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct FpHealthcheckError {
    fp_healthcheck_error: String,
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

    async fn request_filter(&self, session: &mut Session, _ctx: &mut Self::CTX) -> Result<bool> {
        let path = session.req_header().uri.path();

        // Handle /init-tunnel endpoint
        if path == "/init-tunnel" {
            let query_params = session.req_header().uri.query();
            let params: Vec<&str> = query_params.unwrap().split("=").collect();
            let backend_url = params.get(1).unwrap();
            let secret_key = get_secret_key();
            let token = generate_standard_token(&secret_key).unwrap();
            let client = Client::builder().timeout(Duration::from_secs(2)).build().unwrap();

            let res = client
                .get(format!(
                    "{}{}?backend_url={}",
                    LAYER8_URL, "/sp-pub-key", backend_url
                ))
                .header("Authorization", format!("Bearer {}", token))
                .send()
                .await
                .unwrap();

            if res.status().as_u16() != 200 {
                return Err(pingora_core::Error::explain(
                    ErrorType::ConnectProxyFailure,
                    format!(
                        "Failed to get public key from layer8, status code: {}",
                        res.status().as_u16()
                    ),
                ));
            }

            // Copy the body
            let mut old_request_body = Vec::new();
            while let Some(chunk) = session.read_request_body().await? {
                old_request_body.extend_from_slice(&chunk);
            }
            let body_str = String::from_utf8_lossy(&old_request_body).to_string();

            // Build new request
            let new_body = FpRequestBodyInit {
                fp_request_body_init: body_str,
            };

            let client = Client::new();
            let res = client
                .post(format!("{}/init-tunnel", RP_URL))
                .header(
                    "x-fp-request-header-init",
                    "request-header-forward-proxy-init",
                )
                .json(&new_body)
                .send()
                .await
                .unwrap();

            if !res.status().is_success() {
                return Err(pingora_core::Error::explain(
                    ErrorType::ConnectProxyFailure,
                    format!("RP returned error status: {}", res.status().as_u16()),
                ));
            }

            // Get response body from RP
            let response_body = res.text().await.unwrap();

            // Modify response to client
            let mut response = pingora_http::ResponseHeader::build(200, None)?;
            response.insert_header(
                "x-fp-response-header-init",
                "response-header-forward-proxy-init",
            )?;
            session
                .write_response_header(Box::new(response), true)
                .await?;

            let response_body_json = FpResponseBodyInit {
                fp_response_body_init: response_body,
            };
            let response_body_json_str = serde_json::to_string(&response_body_json).unwrap();
            let response_body_json_bytes =
                Bytes::copy_from_slice(response_body_json_str.as_bytes());
            session
                .write_response_body(Some(response_body_json_bytes), true)
                .await?;
            session.finish_body().await?;

            return Ok(true);
           
        }

        // Handle /proxy endpoint
        if path == "/proxy" {
            // Copy the body
            let old_request_body = session.read_request_body().await?.unwrap();
            let body_str = String::from_utf8_lossy(&old_request_body).to_string();

            // Build new request
            let new_body = FpRequestBodyProxied {
                fp_request_body_proxied: body_str,
            };

            let client = Client::new();
            let res = client
                .post(format!("{}/proxy", RP_URL))
                .header(
                    "x-fp-request-header-proxied",
                    "request-header-forward-proxied",
                )
                .json(&new_body)
                .send()
                .await
                .unwrap();

            if !res.status().is_success() {
                return Err(pingora_core::Error::explain(
                    ErrorType::ConnectProxyFailure,
                    format!("RP returned error status: {}", res.status().as_u16()),
                ));
            }

            // Get response headers and body from RP
            let response_headers = res.headers().clone();
            let response_body = res.text().await.unwrap();

            // Modify response to client
            let mut response = pingora_http::ResponseHeader::build(200, None)?;
            response.insert_header(
                "x-fp-response-header-proxied",
                "response-header-forward-proxied",
            )?;

            if let Some(rp_header) = response_headers.get("x-rp-response-header-proxied") {
                response
                    .insert_header("x-rp-response-header-proxied", rp_header.to_str().unwrap())?;
            }

            session
                .write_response_header(Box::new(response), false)
                .await?;

            let response_body_json = FpResponseBodyProxied {
                fp_response_body_proxied: response_body,
            };
            let response_body_json_str = serde_json::to_string(&response_body_json).unwrap();
            let response_body_json_bytes =
                Bytes::copy_from_slice(response_body_json_str.as_bytes());
            session
                .write_response_body(Some(response_body_json_bytes), true)
                .await?;

            return Ok(true);
        }

        // Handle /healthcheck endpoint

        if path == "/healthcheck" {
            let query_params: HashMap<_, _> = session
                .req_header()
                .uri
                .query()
                .map(|q| {
                    q.split('&')
                        .filter_map(|p| {
                            let mut kv = p.splitn(2, '=');
                            match (kv.next(), kv.next()) {
                                (Some(k), Some(v)) => Some((k.to_string(), v.to_string())),
                                _ => None,
                            }
                        })
                        .collect()
                })
                .unwrap_or_default();

            let error_param = query_params
                .get("error")
                .map(|s| s.as_str())
                .unwrap_or("false");

            if error_param == "true" {
                let mut response = pingora_http::ResponseHeader::build(418, None)?;
                response.insert_header("x-fp-healthcheck-error", "response-header-error")?;
                session
                    .write_response_header(Box::new(response), false)
                    .await?;

                let response_body = FpHealthcheckError {
                    fp_healthcheck_error: "this is placeholder for a custom error".to_string(),
                };
                let response_body_json_str = serde_json::to_string(&response_body).unwrap();
                let response_body_json_bytes =
                    Bytes::copy_from_slice(response_body_json_str.as_bytes());
                session
                    .write_response_body(Some(response_body_json_bytes), true)
                    .await?;
            } else {
                let mut response = pingora_http::ResponseHeader::build(200, None)?;
                response.insert_header("x-fp-healthcheck-success", "response-header-success")?;
                session
                    .write_response_header(Box::new(response), false)
                    .await?;

                let response_body = FpHealthcheckSuccess {
                    fp_healthcheck_success: "this is placeholder for a custom body".to_string(),
                };
                let response_body_json_str = serde_json::to_string(&response_body).unwrap();
                let response_body_json_bytes =
                    Bytes::copy_from_slice(response_body_json_str.as_bytes());
                session
                    .write_response_body(Some(response_body_json_bytes), true)
                    .await?;
            }

            return Ok(true);
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
    dotenv::dotenv().ok();
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
