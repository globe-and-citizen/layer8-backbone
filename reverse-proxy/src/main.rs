use std::{collections::HashMap, fs::OpenOptions, io::Write, sync::Arc};

use async_trait::async_trait;
use boring::x509::X509;
use bytes::Bytes;
use chrono::Local;
use env_logger;
use log::*;
use pingora::Result;
use pingora::server::configuration::Opt;
use pingora::upstreams::peer::HttpPeer;
use pingora::{OrErr, server::Server};
use pingora::{
    http::{Method, ResponseHeader, StatusCode},
    listeners::tls::TLS_CONF_ERR,
};
use pingora::{
    proxy::{ProxyHttp, Session},
    utils::tls::CertKey,
};
use reqwest::Client;
use serde::{Deserialize, Serialize};

const UPSTREAM_HOST: &str = "localhost";
const UPSTREAM_IP: &str = "0.0.0.0"; //"125.235.4.59"
const BACKEND_PORT: u16 = 3000;

#[derive(Serialize, Deserialize, Debug)]
pub struct RequestBody {
    data: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ResponseBody {
    data: String,
}

pub struct ReverseProxy {
    // addr: std::net::SocketAddr,
}

impl ReverseProxy {
    fn get_method(session: &Session) -> String {
        let request_summary = session.request_summary();
        let tmp: Vec<&str> = request_summary.split(" ").collect();
        let method: &str = tmp.get(0).unwrap();
        method.to_string()
    }

    async fn handle_request(session: &mut Session) -> Result<Option<ResponseBody>> {
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
                // println!("Request body data: {}", request_body.data); // For debugging
                let request_url = session.req_header().uri.path().to_string();
                debug!(
                    "Creating a new request to http://localhost:{}{}",
                    BACKEND_PORT, request_url
                );
                let client = Client::new();
                let mut map = HashMap::new();
                map.insert("data", request_body.data);
                let res = client
                    .post(format!("http://localhost:{}{}", BACKEND_PORT, request_url))
                    .json(&map)
                    .send()
                    .await
                    .unwrap();
                debug!(
                    "POST {}, Host: localhost:{}, response code: {}",
                    BACKEND_PORT,
                    res.status(),
                    request_url
                );

                Ok(Some(res.json().await.unwrap()))
            }
            Err(err) => {
                error!("ERROR: {err}");
                Ok(None)
            }
        }
    }

    async fn set_headers(
        response_status: StatusCode,
        body_bytes: &Vec<u8>,
        session: &mut Session,
    ) -> Result<()> {
        let mut header = ResponseHeader::build(response_status, None)?;
        header
            .append_header("Content-Length", body_bytes.len().to_string())
            .unwrap();
        // access headers below are needed to pass browser's policy
        header
            .append_header("Access-Control-Allow-Origin", "*".to_string())
            .unwrap();
        header
            .append_header("Access-Control-Allow-Methods", "POST".to_string())
            .unwrap();
        header
            .append_header("Access-Control-Allow-Headers", "Content-Type".to_string())
            .unwrap();
        header
            .append_header("Access-Control-Max-Age", "86400".to_string())
            .unwrap();
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
        // testing certs data; fixme to be dynamic

        // mTLS Steps:
        // 1. Client connects to server
        // 2. Server presents its TLS certificate
        // 3. Client verifies the server's certificate
        // 4. Client presents its TLS certificate
        // 5. Server verifies the client's certificate
        // 6. Server grants access
        // 7. Client and server exchange information over encrypted TLS connection
        //
        // Code below is for step 2(this a a server to FP), presenting the servers's TLS certificate.
        let mut peer = HttpPeer::new(String::from("localhost:6191"), true, String::from(""));
        {
            let cert = X509::from_pem(include_bytes!("../../certs/generated/reverse_proxy.pem"))
                .or_err(TLS_CONF_ERR, "Failed to load CA certificate")?;

            let ca_cert = X509::from_pem(include_bytes!("../../certs/generated/ca.pem"))
                .or_err(TLS_CONF_ERR, "Failed to load CA certificate")?;

            let key = boring::pkey::PKey::private_key_from_pem(include_bytes!(
                "../../certs/generated/reverse_proxy-key.pem"
            ))
            .or_err(TLS_CONF_ERR, "Failed to load private key")?;

            // The certificate to present in mTLS connections to upstream
            // The organization implementing mTLS acts as its own certificate authority.
            let cert_key = CertKey::new(vec![cert, ca_cert], key);
            peer.client_cert_key = Some(Arc::new(cert_key));
        }

        Ok(Box::new(peer))
    }

    async fn request_filter(&self, session: &mut Session, _ctx: &mut Self::CTX) -> Result<bool>
    where
        Self::CTX: Send + Sync,
    {
        let mut response_body = ResponseBody {
            data: String::from(""),
        };
        let mut response_status = StatusCode::OK;

        // get request method
        let method = ReverseProxy::get_method(session);

        // only POST method is allowed for now
        if method == Method::POST.to_string() {
            match ReverseProxy::handle_request(session).await? {
                Some(res) => {
                    response_body = res;
                }
                None => {
                    response_status = StatusCode::BAD_REQUEST;
                }
            }
            // browser always sends an OPTIONS request along with POST for 'application/json' content-type
        } else if method == Method::OPTIONS.to_string() {
            response_status = StatusCode::NO_CONTENT;
        } else {
            response_status = StatusCode::METHOD_NOT_ALLOWED;
        }

        // convert json response to vec
        let response_body_bytes = serde_json::ser::to_vec(&response_body).unwrap();
        ReverseProxy::set_headers(response_status, &response_body_bytes, session).await?;
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

    // env_logger::Builder::from_env(Env::default().write_style_or("RUST_LOG_STYLE", "always"))
    //     .format_file(true)
    //     .format_line_number(true)
    //     .target(Target::Stdout)
    //     .init();

    // let opt = Opt::parse();
    // let mut my_server = Server::new(Some(opt)).unwrap();
    let mut my_server = Server::new(Some(Opt {
        conf: Some(format!(
            "{}/../server_conf.yml",
            env!("CARGO_MANIFEST_DIR")
        )),
        ..Default::default()
    }))
    .unwrap();

    my_server.bootstrap();

    let mut my_proxy =
        pingora::proxy::http_proxy_service(&my_server.configuration, ReverseProxy {});

    // fixme; figure out how to dynamically get certs paths in a real network, maybe an endpoint behind an API key...
    {
        let server_pem = format!(
            "{}/../certs/generated/reverse_proxy.pem",
            env!("CARGO_MANIFEST_DIR")
        );
        let server_key = format!(
            "{}/../certs/generated/reverse_proxy-key.pem",
            env!("CARGO_MANIFEST_DIR")
        );

        my_proxy
            .add_tls("localhost:6193", &server_pem, &server_key)
            .unwrap();
    }

    my_server.add_service(my_proxy);
    my_server.run_forever();
}
