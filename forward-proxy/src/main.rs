use async_trait::async_trait;
use log::info;
use pingora_core::prelude::*;
use pingora_proxy::{ProxyHttp, Session};
use simplelog::{ConfigBuilder, LevelFilter, WriteLogger};
use std::fs;
use serde::{Deserialize, Serialize};


struct ReverseProxy;

#[derive(Serialize, Deserialize, Debug)]
pub struct ResponseBody {
    data: String,
}

#[async_trait]
impl ProxyHttp for ReverseProxy {
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

    async fn response_filter(
        &self,
        _session: &mut Session,
        upstream_response: &mut pingora_http::ResponseHeader,
        _ctx: &mut Self::CTX,
    ) -> Result<()> {
        // Fixed CORS issue
        upstream_response.insert_header("Access-Control-Allow-Origin", "*")?;
        upstream_response.insert_header("Vary", "Origin")?;
        Ok(())
    }
}

fn main() {
    // Initialize logger
    let log_file = fs::File::create("log.txt").expect("Failed to create log file");
    let config = ConfigBuilder::new().set_time_to_local(true).build();
    WriteLogger::init(LevelFilter::Debug, config, log_file).expect("Failed to initialize logger");

    info!("Starting server...");

    let mut server = Server::new(None).unwrap();
    server.bootstrap();

    let mut proxy = pingora_proxy::http_proxy_service(&server.configuration, ReverseProxy);

    proxy.add_tcp("0.0.0.0:6191");

    server.add_service(proxy);

    server.run_forever();
}
