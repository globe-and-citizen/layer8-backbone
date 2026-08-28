mod config;
mod handler;
mod proxy;
mod statistics;

use crate::config::FPConfig;
use crate::handler::ForwardHandler;
use crate::statistics::influxdb_client::InfluxDBClient;
use crate::statistics::Statistics;
use pingora::prelude::*;
use proxy::ForwardProxy;
use std::sync::Arc;
use tokio::runtime::Runtime;
use tracing::{debug, info};
use utils::cert::{watch_tls, TLSCredentials};

fn load_config() -> FPConfig {
    // Load environment variables from .env file
    dotenv::dotenv().ok();

    // Deserialize from env vars
    let config: FPConfig = envy::from_env().expect("Failed to load config");

    debug!(name: "FPConfig", value = ?config);
    config
}

fn main() {
    // Load environment variables and deserialize configuration.
    let config = load_config();

    // Load TLS key/certificate pair and keep it hot-reloaded if changed on disk.
    let tls_cred = match TLSCredentials::load(&config.proxy.tls) {
        Ok(conf) => Arc::new(conf),
        Err(err) => {
            panic!("Failed to load TLS config {}", err)
        }
    };
    watch_tls(tls_cred.clone(), config.proxy.tls.clone());

    // Initialize the async runtime and the influxdb statistics writer.
    // This is required before the proxy can emit metrics.
    let rt = Runtime::new().unwrap();
    let influxdb_client = InfluxDBClient::new(&config.influxdb);
    let _logger_guard = rt.block_on(async {
        Statistics::init_statistics_writer(Box::new(influxdb_client)).await;

        // Initialize logging as early as possible so startup and runtime errors
        // are visible in the configured output.
        utils::log::init_logger(
            config.log,
            config.telemetry
        )
    });

    // Build the Pingora server and bootstrap the internal configuration.
    let mut server = Server::new(Some(Opt {
        conf: std::env::var("SERVER_CONF").ok(),
        ..Default::default()
    }))
    .expect("Failed to create server");
    server.bootstrap();

    // Create the HTTP forward proxy handler with the loaded configuration.
    let fp_handler = ForwardHandler::new(config.handler);

    // Create the proxy service bound to the configured address and port.
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
