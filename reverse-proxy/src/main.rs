use async_trait::async_trait;
use bytes::Bytes;
use clap::Parser;
use serde::{Deserialize, Serialize};
use std::net::ToSocketAddrs;

use pingora::Result;
use pingora::http::{Method, ResponseHeader, StatusCode};
use pingora::proxy::{ProxyHttp, Session};
use pingora::server::Server;
use pingora::server::configuration::Opt;
use pingora::upstreams::peer::HttpPeer;

use chrono::Local;
use env_logger;
use log::*;
use reqwest::Client;
use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::Write;

const UPSTREAM_HOST: &str = "localhost";
const UPSTREAM_IP: &str = "0.0.0.0";
const BACKEND_PORT: u16 = 3000;

#[derive(Serialize, Deserialize, Debug)]
pub struct RequestBody {
    data: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ResponseBody {
    rp_response_body_init_proxied: Option<String>,
    rp_request_body_proxied: Option<String>,
    fp_request_body_proxied: Option<String>,
    error: Option<String>,
}

pub struct ReverseProxy {
    addr: std::net::SocketAddr,
}

impl ReverseProxy {
    fn get_method(session: &Session) -> String {
        let request_summary = session.request_summary();
        let tmp: Vec<&str> = request_summary.split(" ").collect();
        let method: &str = tmp.get(0).unwrap();
        method.to_string()
    }

    async fn handle_init_tunnel(session: &mut Session) -> Result<Option<ResponseBody>> {
        let response = ResponseBody {
            rp_response_body_init_proxied: Some("response body init, reverse proxy".to_string()),
            rp_request_body_proxied: None,
            fp_request_body_proxied: None,
            error: None,
        };
        Ok(Some(response))
    }

    async fn handle_proxy_request(session: &mut Session) -> Result<Option<ResponseBody>> {
        // read request body
        let mut body = Vec::new();
        loop {
            match session.read_request_body().await? {
                Some(chunk) => body.extend_from_slice(&chunk),
                None => break,
            }
        }

        // convert to json
        match serde_json::de::from_slice::<RequestBody>(&body) {
            Ok(request_body) => {
                debug!("Request body: {:?}", request_body.data);
                let request_url = session.req_header().uri.path().to_string();
                debug!(
                    "Creating a new request to http://localhost:{}{}",
                    BACKEND_PORT, request_url
                );
                
                let client = Client::new();
                let mut map = HashMap::new();
                map.insert("fp_request_body_proxied", request_body.data);
                
                let res = client
                    .post(format!("http://localhost:{}{}", BACKEND_PORT, request_url))
                    .header("x-fp-request-header-proxied", "request-header-forward-proxied")
                    .header("x-rp-request-header-proxied", "request-header-reverse-proxy")
                    .json(&map)
                    .send()
                    .await;
                
                match res {
                    Ok(response) => {
                        debug!(
                            "POST {}, Host: localhost:{}, response code: {}",
                            request_url,
                            BACKEND_PORT,
                            response.status()
                        );

                        let mut response_body: ResponseBody = response.json().await.unwrap();
                        response_body.rp_request_body_proxied = Some("data copied by reverse proxy".to_string());
                        Ok(Some(response_body))
                    }
                    Err(err) => {
                        error!("Error forwarding request to backend: {}", err);
                        let status = err.status().unwrap_or(reqwest::StatusCode::INTERNAL_SERVER_ERROR);
                        Ok(Some(ResponseBody {
                            rp_response_body_init_proxied: None,
                            rp_request_body_proxied: None,
                            fp_request_body_proxied: None,
                            error: Some(format!("Backend error: {}", status)),
                        }))
                    }
                }
            }
            Err(err) => {
                error!("Error parsing request body: {}", err);
                Ok(Some(ResponseBody {
                    rp_response_body_init_proxied: None,
                    rp_request_body_proxied: None,
                    fp_request_body_proxied: None,
                    error: Some("Invalid request body".to_string()),
                }))
            }
        }
    }

    async fn set_headers(
        response_status: StatusCode,
        body_bytes: &Vec<u8>,
        session: &mut Session,
        is_init_tunnel: bool,
    ) -> Result<()> {
        let mut header = ResponseHeader::build(response_status, None)?;
        header
            .append_header("Content-Length", body_bytes.len().to_string())
            .unwrap();
        
        // Common headers
        header
            .append_header("Access-Control-Allow-Origin", "*")
            .unwrap();
        header
            .append_header("Access-Control-Allow-Methods", "POST, OPTIONS")
            .unwrap();
        header
            .append_header("Access-Control-Allow-Headers", "Content-Type")
            .unwrap();
        header
            .append_header("Access-Control-Max-Age", "86400")
            .unwrap();
        
        // Endpoint-specific headers
        if is_init_tunnel {
            header
                .append_header("x-rp-response-header-init", "response-header-init-reverse-proxy")
                .unwrap();
        } else {
            header
                .append_header("x-rp-response-header-added", "response-header-forward-proxied")
                .unwrap();
        }
        
        session.write_response_header_ref(&header).await
    }
}

#[async_trait]
impl ProxyHttp for ReverseProxy {
    type CTX = ();

    fn new_ctx(&self) -> Self::CTX {}

    async fn upstream_peer(
        &self,
        _session: &mut Session,
        _ctx: &mut Self::CTX,
    ) -> Result<Box<HttpPeer>> {
        let peer: Box<HttpPeer> =
            Box::new(HttpPeer::new(self.addr, false, UPSTREAM_HOST.to_owned()));
        Ok(peer)
    }

    async fn request_filter(&self, session: &mut Session, _ctx: &mut Self::CTX) -> Result<bool>
    where
        Self::CTX: Send + Sync,
    {
        let mut response_body = ResponseBody {
            rp_response_body_init_proxied: None,
            rp_request_body_proxied: None,
            fp_request_body_proxied: None,
            error: None,
        };
        let mut response_status = StatusCode::OK;
        let mut is_init_tunnel = false;

        // get request method
        let method = ReverseProxy::get_method(session);
        let path = session.req_header().uri.path();

        // Handle different endpoints
        if path == "/init-tunnel" {
            if method == Method::POST.to_string() {
                match ReverseProxy::handle_init_tunnel(session).await? {
                    Some(res) => {
                        response_body = res;
                        is_init_tunnel = true;
                    }
                    None => {
                        response_status = StatusCode::BAD_REQUEST;
                    }
                }
            } else if method == Method::OPTIONS.to_string() {
                response_status = StatusCode::NO_CONTENT;
            } else {
                response_status = StatusCode::METHOD_NOT_ALLOWED;
            }
        } else if path == "/proxy" {
            if method == Method::POST.to_string() {
                match ReverseProxy::handle_proxy_request(session).await? {
                    Some(res) => {
                        response_body = res;
                    }
                    None => {
                        response_status = StatusCode::BAD_REQUEST;
                    }
                }
            } else if method == Method::OPTIONS.to_string() {
                response_status = StatusCode::NO_CONTENT;
            } else {
                response_status = StatusCode::METHOD_NOT_ALLOWED;
            }
        } else {
            response_status = StatusCode::NOT_FOUND;
            response_body.error = Some("Endpoint not found".to_string());
        }

        // Handle error responses
        if response_body.error.is_some() {
            response_status = match response_body.error.as_ref().unwrap().contains("Backend error") {
                true => StatusCode::BAD_GATEWAY,
                false => StatusCode::BAD_REQUEST,
            };
        }

        // convert json response to vec
        let response_body_bytes = serde_json::ser::to_vec(&response_body).unwrap();
        ReverseProxy::set_headers(response_status, &response_body_bytes, session, is_init_tunnel).await?;
        session
            .write_response_body(Some(Bytes::from(response_body_bytes)), true)
            .await?;

        Ok(true)
    }

    async fn logging(
        &self,
        session: &mut Session,
        _e: Option<&pingora::Error>,
        ctx: &mut Self::CTX,
    ) {
        let response_code = session
            .response_written()
            .map_or(0, |resp| resp.status.as_u16());
        // access log
        info!(
            "{} response code: {response_code}",
            self.request_summary(session, ctx)
        );
    }
}

fn main() {
    let file = OpenOptions::new()
        .append(true)
        .create(true)
        .open("log.txt")
        .expect("Can't create file!");

    let target = Box::new(file);

    env_logger::Builder::new()
        .target(env_logger::Target::Pipe(target))
        .filter(None, LevelFilter::Debug)
        .format(|buf, record| {
            writeln!(
                buf,
                "[{} {} {}:{}] {}",
                Local::now().format("%Y-%m-%d %H:%M:%S%.3f"),
                record.level(),
                record.file().unwrap_or("unknown"),
                record.line().unwrap_or(0),
                record.args()
            )
        })
        .init();

    let opt = Opt::parse();
    let mut my_server = Server::new(Some(opt)).unwrap();
    my_server.bootstrap();

    let mut my_proxy = pingora::proxy::http_proxy_service(
        &my_server.configuration,
        ReverseProxy {
            addr: (UPSTREAM_IP.to_owned(), BACKEND_PORT)
                .to_socket_addrs()
                .unwrap()
                .next()
                .unwrap(),
        },
    );

    // Listen on both endpoints
    my_proxy.add_tcp("0.0.0.0:6193");  // Publicly accessible
    // my_proxy.add_tcp("127.0.0.1:6194"); // Localhost only

    my_server.add_service(my_proxy);
    my_server.run_forever();
}