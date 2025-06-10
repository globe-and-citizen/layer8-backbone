use async_trait::async_trait;
use pingora::prelude::{HttpPeer, ProxyHttp, Session};
use reqwest::Client;
use pingora::http;
use log::{error, info};
use crate::utils;
use crate::message::types::{FpResponseBodyInit, FpResponseBodyProxied};

const LAYER8_URL: &str = "http://127.0.0.1:5001";
const RP_URL: &str = "http://127.0.0.1:6193";

async fn handle_init_tunnel(session: &mut Session, body_string: String) -> pingora::Result<bool> {
    let query_params = session.req_header().uri.query();
    let params: Vec<&str> = query_params.unwrap().split("=").collect();
    let backend_url = params.get(1).unwrap();
    let secret_key = utils::get_secret_key();
    let token = utils::generate_standard_token(&secret_key).unwrap();
    let client = Client::new();

    let res = match client
        .get(format!("{}{}?backend_url={}", LAYER8_URL, "/sp-pub-key", backend_url))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
    {
        Ok(res) => res,
        Err(e) => {
            let response_body = serde_json::json!({
                        "error": format!("Failed to connect to layer8: {}", e)
                    });

            let mut header = http::ResponseHeader::build(500, None)?;
            header.insert_header("Content-Type", "application/json")?;

            // Single response with headers and body
            session
                .write_response_header(Box::new(header), false)
                .await?;
            session
                .write_response_body(
                    Some(bytes::Bytes::from(response_body.to_string())),
                    true,
                )
                .await?;
            return Ok(true);
        }
    };

    if !res.status().is_success() {
        let response_body = serde_json::json!({
                    "error": format!("Failed to get public key from layer8, status code: {}", res.status().as_u16())
                });
        info!("Sending error response: {}", response_body);

        let mut header = http::ResponseHeader::build(
            res.status().as_u16().try_into().unwrap_or(400),
            None,
        )?;
        header.insert_header("Content-Type", "application/json")?;
        header.insert_header("Connection", "close")?; // Ensure connection closes
        header.insert_header("Content-Length", response_body.to_string().len())?;
        // Single response with headers and body
        session
            .write_response_header(Box::new(header), false)
            .await?;
        session
            .write_response_body(Some(bytes::Bytes::from(response_body.to_string())), true)
            .await?;
        return Ok(true);
    }

    let client = Client::new();
    let response = client
        .post(format!("{}/init-tunnel", RP_URL))
        .header(
            "x-fp-request-header-init",
            "request-header-forward-proxy-init",
        )
        .header("Content-Type", "application/json")
        .body(body_string.clone())
        .send()
        .await;

    match response {
        Ok(res) if res.status().is_success() => {
            let response_body = res.text().await.unwrap_or_default();
            let mut header = http::ResponseHeader::build(200, None)?;
            header.insert_header(
                "x-fp-response-header-init",
                "response-header-forward-proxy-init",
            )?;
            header.insert_header("Content-Type", "application/json")?;
            // header.insert_header("Content-Length", response_body.to_string().len())?;
            header.insert_header("Content-Length", 2048)?;

            let response_json = FpResponseBodyInit {
                fp_response_body_init: response_body.clone(),
            };

            session
                .write_response_header(Box::new(header), false)
                .await?;
            session
                .write_response_body(
                    Some(bytes::Bytes::from(
                        serde_json::to_string(&response_json).unwrap(),
                    )),
                    true,
                )
                .await?;
            return Ok(true);
        }
        Ok(res) => {
            // Handle 4xx/5xx errors
            let status = res.status();
            let error_body = res.text().await.unwrap_or_default();

            let mut header = http::ResponseHeader::build(status.as_u16(), None)?;
            header.insert_header("Content-Type", "application/json")?;
            header.insert_header("Content-Length", error_body.to_string().len())?;

            session
                .write_response_header(Box::new(header), false)
                .await?;
            session
                .write_response_body(Some(bytes::Bytes::from(error_body)), true)
                .await?;
            return Ok(true);
        }
        Err(e) => {
            error!("Failed to forward request to RP: {}", e);
            let mut header = http::ResponseHeader::build(500, None)?;
            header.insert_header("Content-Type", "application/json")?;
            header.insert_header("Content-Length", e.to_string().len())?;

            session
                .write_response_header(Box::new(header), false)
                .await?;
            session
                .write_response_body(
                    Some(bytes::Bytes::from(
                        serde_json::json!({
                                    "error": format!("Failed to forward request: {}", e)
                                })
                            .to_string(),
                    )),
                    true,
                )
                .await?;
            return Ok(true);
        }
    }
}

async fn handle_proxy(session: &mut Session, body_string: String) -> pingora::Result<bool> {
    // Handle proxy endpoint
    let client = Client::new();
    // let request_body = FpRequestBodyProxied {
    //     fp_request_body_proxied: body_string.clone(),
    // };

    let response = client
        .post(format!("{}/proxy", RP_URL))
        .header(
            "x-fp-request-header-proxied",
            "request-header-forward-proxied",
        )
        // .json(&request_body)
        .body(body_string.clone())
        .send()
        .await;

    match response {
        Ok(res) if res.status().is_success() => {
            let headers = res.headers().clone();
            let response_body = res.text().await.unwrap_or_default();

            let mut header = http::ResponseHeader::build(200, None)?;
            header.insert_header(
                "x-fp-response-header-proxied",
                "response-header-forward-proxied",
            )?;

            if let Some(rp_header) = headers.get("x-rp-response-header-proxied") {
                header.insert_header(
                    "x-rp-response-header-proxied",
                    rp_header.to_str().unwrap_or(""),
                )?;
            }

            header.insert_header("Content-Type", "application/json")?;
            // header.insert_header("Content-Length", response_body.to_string().len())?;
            header.insert_header("Content-Length", 2048)?;

            let response_json = FpResponseBodyProxied {
                fp_response_body_proxied: response_body.clone(),
            };

            session
                .write_response_header(Box::new(header), false)
                .await?;
            session
                .write_response_body(
                    Some(bytes::Bytes::from(
                        serde_json::to_string(&response_json).unwrap(),
                    )),
                    true,
                )
                .await?;
            return Ok(true);
        }
        Ok(res) => {
            // Handle 4xx/5xx errors
            let status = res.status();
            let error_body = res.text().await.unwrap_or_default();

            let mut header = http::ResponseHeader::build(status.as_u16(), None)?;
            header.insert_header("Content-Type", "application/json")?;
            header.insert_header("Content-Length", error_body.to_string().len())?;

            session
                .write_response_header(Box::new(header), false)
                .await?;
            session
                .write_response_body(Some(bytes::Bytes::from(error_body)), true)
                .await?;
            return Ok(true);
        }
        Err(e) => {
            error!("Failed to proxy request: {}", e);
            let mut header = http::ResponseHeader::build(500, None)?;
            header.insert_header("Content-Type", "application/json")?;
            header.insert_header("Content-Length", e.to_string().len())?;

            session
                .write_response_header(Box::new(header), false)
                .await?;
            session
                .write_response_body(
                    Some(bytes::Bytes::from(
                        serde_json::json!({
                                    "error": format!("Failed to proxy request: {}", e)
                                })
                            .to_string(),
                    )),
                    true,
                )
                .await?;
            return Ok(true);
        }
    }
}

async fn handle_healthcheck(session: &mut Session) -> pingora::Result<bool> {
    let query_params = session.req_header().uri.query();
    let params: Vec<&str> = query_params.unwrap().split("=").collect();
    let error = params.get(1).unwrap();
    if *error == "true" {
        let mut header = http::ResponseHeader::build(418, None)?;
        header.insert_header("Content-Type", "application/json")?;
        header.insert_header("x-fp-healthcheck-error", "response-header-error")?;
        let response_body = serde_json::json!({
                    "fp_healthcheck_error": "this is placeholder for a custom error"
                });
        header.insert_header("Content-Length", response_body.to_string().len())?;
        session
            .write_response_header(Box::new(header), false)
            .await?;
        session
            .write_response_body(Some(bytes::Bytes::from(response_body.to_string())), true)
            .await?;
        return Ok(true);
    }
    let mut header = http::ResponseHeader::build(200, None)?;
    header.insert_header("Content-Type", "application/json")?;
    header.insert_header("x-fp-healthcheck-success", "response-header-success")?;
    let response_body = serde_json::json!({
                "fp_healthcheck_success": "this is placeholder for a custom body"
            });
    header.insert_header("Content-Length", response_body.to_string().len())?;
    session
        .write_response_header(Box::new(header), false)
        .await?;
    session
        .write_response_body(Some(bytes::Bytes::from(response_body.to_string())), true)
        .await?;
    return Ok(true);
}

pub struct ForwardProxy {}

#[async_trait]
impl ProxyHttp for ForwardProxy {
    type CTX = ();

    fn new_ctx(&self) -> Self::CTX {()}

    async fn upstream_peer(
        &self,
        _session: &mut Session,
        _ctx: &mut Self::CTX,
    ) -> pingora::Result<Box<HttpPeer>> {
        Ok(Box::from(HttpPeer::new(
            String::from("127.0.0.1:6193"),
            false,
            String::from(""),
        )))
    }

    async fn request_filter(&self, session: &mut Session, ctx: &mut Self::CTX) -> pingora::Result<bool>
    where
        Self::CTX: Send + Sync,
    {
        // create Context
        let request_url = session.req_header().uri.to_string();
        let mut body = Vec::new();
        loop {
            match session.read_request_body().await? {
                Some(chunk) => body.extend_from_slice(&chunk),
                None => break,
            }
        }

        let body_string = String::from_utf8_lossy(&body).to_string();

        if request_url.contains("init-tunnel") {
            return handle_init_tunnel(session, body_string).await
        } else if request_url.contains("proxy") {
            return handle_proxy(session, body_string).await
        } else if request_url.contains("healthcheck") {
            return handle_healthcheck(session).await
        }

        // set headers
        let mut header = ResponseHeader::build(handler_response.status, None).unwrap();
        for (key, val) in ctx.get_response_header().iter() {
            header.append_header(key.into(), val).unwrap()?
        };

        let mut response_bytes = vec![];
        if let Some(body_bytes) = handler_response.body {
            header.append_header("Content-length", &body_bytes.len().to_string()).unwrap()?;
            response_bytes = body_bytes.clone()
        };

        session.write_response_header_ref(&header).await?;
        session.write_response_body(Some(Bytes::from(response_bytes)), true).await?;

        Ok(true)
    }

    async fn response_filter(
        &self,
        _session: &mut Session,
        upstream_response: &mut http::ResponseHeader,
        _ctx: &mut Self::CTX,
    ) -> pingora::Result<()> {
        upstream_response.insert_header("Access-Control-Allow-Origin", "*")?;
        upstream_response.insert_header("Access-Control-Allow-Methods", "GET, POST")?;
        upstream_response.insert_header("Access-Control-Allow-Headers", "Content-Type")?;
        Ok(())
    }
}
