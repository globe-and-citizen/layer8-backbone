use crate::mock;
use futures::FutureExt;
use once_cell::sync::Lazy;
use pingora::listeners::tls::TlsSettings;
use pingora::prelude::{Opt, Server, http_proxy_service};
use pingora_router::handler::APIHandler;
use pingora_router::router::Router;
use reverse_proxy::handler::ReverseHandler;
use reverse_proxy::proxy::ReverseProxy;
use reverse_proxy::tls_conf::TLSServerConfig;
use std::sync::Arc;
use std::thread;
use utils::cert::TLSCredentials;

#[allow(dead_code)]
pub static TEST_REVERSE_PROXY: Lazy<TestServer> = Lazy::new(TestServer::start);

pub struct TestServer {
    #[allow(dead_code)]
    pub handle: thread::JoinHandle<()>,
}

impl TestServer {
    pub fn start() -> Self {
        let server_handle = thread::spawn(|| {
            start_reverse_proxy();
        });
        TestServer {
            handle: server_handle,
        }
    }
}

fn start_reverse_proxy() {
    let rp_config = reverse_proxy::config::RPConfig {
        log: reverse_proxy::config::LogConfig {
            log_level: "info".to_string(),
            log_format: "plain".to_string(),
            log_path: "console".to_string(),
            log_filename: "".to_string(),
        },
        server: reverse_proxy::config::ServerConfig {
            listen_address: "localhost".to_string(),
            listen_port: mock::data::REVERSE_PROXY_PORT,
        },
        proxy: reverse_proxy::config::ProxyConfig {
            tls: utils::cert::TLSConfig {
                enable_tls: false,
                ca_path: "./certs/root_ca.crt".to_string(),
                cert_path: "./certs/server.crt".to_string(),
                key_path: "./certs/server.key".to_string(),
            },
            cors_allow_credentials: false,
            cors_allow_origins: vec!["*".to_string()],
        },
        handler: reverse_proxy::config::HandlerConfig {
            ntor_server_id: mock::data::MOCK_BACKEND_URL.to_string(),
            ntor_static_secret: mock::data::MOCK_NTOR_SERVER_STATIC_PUBLIC_KEY,
            jwt_virtual_connection_secret: mock::data::VALID_JWT_SECRET.to_vec(),
            jwt_exp_in_hours: 1,
            backend_url: mock::data::MOCK_BACKEND_URL.to_string(),
        },
    };

    let tls_cred = match TLSCredentials::load(&rp_config.proxy.tls) {
        Ok(conf) => Arc::new(conf),
        Err(err) => {
            panic!("Failed to load TLS config {}", err)
        }
    };

    let _logger_guard = utils::log::init_logger(
        rp_config.log.log_level.clone(),
        rp_config.log.log_format.clone(),
        rp_config.log.log_path.clone(),
        rp_config.log.log_filename.clone(),
    );

    let mut server = Server::new(Some(Opt {
        conf: std::env::var("SERVER_CONF").ok(),
        ..Default::default()
    }))
    .expect("Failed to create server");
    server.bootstrap();

    let handle_init_tunnel: APIHandler<Arc<ReverseHandler>> =
        Box::new(|h, ctx| async move { h.handle_init_tunnel(ctx).await }.boxed());

    let handle_proxy: APIHandler<Arc<ReverseHandler>> =
        Box::new(|h, ctx| async move { h.handle_proxy_request(ctx).await }.boxed());

    let handle_healthcheck: APIHandler<Arc<ReverseHandler>> =
        Box::new(|h, ctx| async move { h.handle_healthcheck(ctx).await }.boxed());

    let rp_handler = Arc::new(ReverseHandler::new(rp_config.clone()));
    let mut router: Router<Arc<ReverseHandler>> = Router::new(rp_handler);
    router.post("/init-tunnel".to_string(), Box::new([handle_init_tunnel]));
    router.post("/proxy".to_string(), Box::new([handle_proxy]));
    router.get("/healthcheck".to_string(), Box::new([handle_healthcheck]));

    let mut my_proxy = http_proxy_service(
        &server.configuration,
        ReverseProxy::new(rp_config.proxy.clone(), router),
    );

    let tls_server_config = TLSServerConfig {
        host_name: "reverse-proxy".to_string(),
        tls_credentials: tls_cred,
    };

    if rp_config.proxy.tls.enable_tls {
        my_proxy.add_tls_with_settings(
            &format!(
                "{}:{}",
                rp_config.server.listen_address, rp_config.server.listen_port
            ),
            None,
            TlsSettings::with_callbacks(Box::new(tls_server_config))
                .expect("Cannot set TlsSettings callbacks"),
        );
    } else {
        my_proxy.add_tcp(&format!(
            "{}:{}",
            rp_config.server.listen_address, rp_config.server.listen_port
        ));
    }

    server.add_service(my_proxy);
    server.run_forever();
}
