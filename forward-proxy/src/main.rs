mod config;
mod handler;
mod proxy;
mod statistics;
use crate::config::FPConfig;
use crate::handler::ForwardHandler;
use crate::statistics::Statistics;
use crate::statistics::influxdb_client::InfluxDBClient;
use pingora::prelude::*;
use proxy::ForwardProxy;
use std::sync::Arc;
use tokio::runtime::Runtime;
use tracing::{debug, info};
use utils::cert::{TLSCredentials, watch_tls};

fn load_config() -> FPConfig {
    // Load environment variables from .env file
    dotenv::dotenv().ok();

    // Deserialize from env vars
    let config: FPConfig = envy::from_env().expect("Failed to load config");

    debug!(name: "FPConfig", value = ?config);
    config
}

fn main() {
    let config = load_config();
    let tls_cred = match TLSCredentials::load(&config.proxy.tls) {
        Ok(conf) => Arc::new(conf),
        Err(err) => {
            panic!("Failed to load TLS config {}", err)
        }
    };
    watch_tls(tls_cred.clone(), config.proxy.tls.clone());
    // let influxdb_client = InfluxDBClient::new(&config.influxdb_config);

    // Initialize the async runtime
    let rt = Runtime::new().unwrap();
    let influxdb_client = InfluxDBClient::new(&config.influxdb);
    rt.block_on(Statistics::init_statistics_writer(Box::new(
        influxdb_client,
    )));

    let _logger_guard = utils::log::init_logger(
        config.log.log_level.clone(),
        config.log.log_format.clone(),
        config.log.log_path.clone(),
        config.log.log_filename.clone(),
    );

    let mut server = Server::new(Some(Opt {
        conf: std::env::var("SERVER_CONF").ok(),
        ..Default::default()
    }))
    .expect("Failed to create server");
    server.bootstrap();

    let fp_handler = ForwardHandler::new(config.handler);

    let mut proxy = http_proxy_service(
        &server.configuration,
        ForwardProxy::new(config.proxy, tls_cred, fp_handler),
    );

    proxy.add_tcp(&format!("{}:{}", config.listen_address, config.listen_port));

    server.add_service(proxy);

    info!(
        "Starting server at {}:{}",
        config.listen_address, config.listen_port
    );

    server.run_forever();
}
