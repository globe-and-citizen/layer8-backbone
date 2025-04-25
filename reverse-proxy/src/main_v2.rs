use async_trait::async_trait;
use pingora_core::prelude::{HttpPeer, Server};
use pingora_proxy::ProxyHttp;

/// Configuration for the reverse proxy server
#[derive(Debug)]
struct ProxyConfig {
    upstream_host: String,
    listen_addr: String,
    listen_port: u16,
    use_tls: bool,
}

impl Default for ProxyConfig {
    fn default() -> Self {
        Self {
            upstream_host: String::from("127.0.0.1:3000"),
            listen_addr: String::from("0.0.0.0"),
            listen_port: 6193,
            use_tls: false,
        }
    }
}

/// ReverseProxy handles HTTP traffic forwarding to upstream servers
struct ReverseProxy {
    config: ProxyConfig,
}

impl ReverseProxy {
    fn new(config: ProxyConfig) -> Self {
        Self { config }
    }

    fn setup_server() -> Server {
        let mut server = Server::new(None).expect("Failed to create server");
        server.bootstrap();
        server
    }
}

#[async_trait]
impl ProxyHttp for ReverseProxy {
    type CTX = ();

    fn new_ctx(&self) -> Self::CTX {}

    async fn upstream_peer(
        &self,
        _session: &mut pingora_core::http::Session,
        _ctx: &mut Self::CTX,
    ) -> Result<Box<pingora_core::upstreams::peer::HttpPeer>> {
        Ok(Box::from(HttpPeer::new(
            self.config.upstream_host.clone(),
            self.config.use_tls,
            String::from(""),
        )))
    }
}

fn main() {
    env_logger::init();

    let config = ProxyConfig::default();
    let reverse_proxy = ReverseProxy::new(config);

    let mut server = ReverseProxy::setup_server();
    let mut proxy = pingora_proxy::http_proxy_service(&server.configuration, reverse_proxy);

    let listen_address = format!("{}:{}", config.listen_addr, config.listen_port);
    proxy.add_tcp(&listen_address);

    server.add_service(proxy);
    server.run_forever();
}