use std::sync::Arc;
use std::thread;
use once_cell::sync::Lazy;
use pingora::prelude::{http_proxy_service, Opt, Server};
use forward_proxy::handler::{ForwardHandler, IntFPSession};
use forward_proxy::proxy::ForwardProxy;
use utils::cert::TLSCredentials;
use crate::mock;
use crate::mock::data::{AUTH_ACCESS_TOKEN, AUTH_NTOR_CERT_API_PATH, AUTH_SERVER_PORT, FORWARD_PROXY_PORT};

pub static TEST_FORWARD_PROXY: Lazy<TestServer> = Lazy::new(TestServer::start);

pub struct TestServer {
    pub handle: thread::JoinHandle<()>,
}

impl TestServer {
    pub fn start() -> Self {
        let server_handle = thread::spawn(|| {
            start_forward_proxy();
        });
        TestServer {
            handle: server_handle,
        }
    }
}

fn start_forward_proxy() {
    let fp_config = forward_proxy::config::FPConfig {
        proxy: forward_proxy::config::ProxyConfig {
            tls: utils::cert::TLSConfig {
                enable_tls: false,
                ca_path: "./certs/root_ca.crt".to_string(),
                cert_path: "./certs/client.crt".to_string(),
                key_path: "./certs/client.key".to_string(),
            },
            cors_allow_credentials: false,
            cors_allow_origins: vec!["*".to_string()],
        },
        handler: forward_proxy::config::HandlerConfig {
            jwt_virtual_connection_key: Vec::from(mock::data::MOCK_JWT_SECRET.to_string()),
            jwt_exp_in_hours: 24,
            auth_access_token: AUTH_ACCESS_TOKEN.to_string(),
            auth_get_certificate_url: format!(
                "http://127.0.0.1:{}{}?backend_url=",
                AUTH_SERVER_PORT,
                AUTH_NTOR_CERT_API_PATH
            ),
        },
        log: forward_proxy::config::LogConfig {
            log_level: "info".to_string(),
            log_format: "plain".to_string(),
            log_path: "console".to_string(),
            log_filename: "".to_string(),
        },
        influxdb: forward_proxy::config::InfluxDBConfig {
            influxdb_url: "".to_string(),
            influxdb_org: "".to_string(),
            influxdb_bucket: "".to_string(),
            influxdb_auth_token: "".to_string(),
        },
        listen_address: "localhost".to_string(),
        listen_port: FORWARD_PROXY_PORT,
    };

    let tls_cred = match TLSCredentials::load(&fp_config.proxy.tls) {
        Ok(conf) => Arc::new(conf),
        Err(err) => {
            panic!("Failed to load TLS config {}", err)
        }
    };

    let _logger_guard = utils::log::init_logger(
        fp_config.log.log_level.clone(),
        fp_config.log.log_format.clone(),
        fp_config.log.log_path.clone(),
        fp_config.log.log_filename.clone(),
    );

    let mut server = Server::new(Some(Opt {
        conf: std::env::var("SERVER_CONF").ok(),
        ..Default::default()
    })).expect("Failed to create server");
    server.bootstrap();

    let fp_handler = ForwardHandler::new(fp_config.handler);
    fp_handler.set_session(&mock::data::MOCK_INT_FP_JWT, IntFPSession {
        client_id: mock::data::MOCK_AUTH_CLIENT_ID.to_string(),
        rp_base_url: mock::data::BACKEND_URL.to_string(),
        fp_rp_jwt: mock::data::MOCK_FP_RP_JWT.to_string(),
    });

    let mut proxy = http_proxy_service(
        &server.configuration,
        ForwardProxy::new(fp_config.proxy, tls_cred, fp_handler),
    );

    proxy.add_tcp(&format!("{}:{}", fp_config.listen_address, fp_config.listen_port));

    server.add_service(proxy);

    println!("Starting forward proxy at {}:{}", fp_config.listen_address, fp_config.listen_port);

    server.run_forever();
}