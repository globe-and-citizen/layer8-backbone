mod config;
mod handler;
mod proxy;
mod statistics;

use crate::config::FPConfig;
use crate::handler::ForwardHandler;
use crate::statistics::Statistics;
use pingora::prelude::*;
use proxy::ForwardProxy;
use tokio::runtime::Runtime;
use tracing::{debug, info};

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
    // let influxdb_client = InfluxDBClient::new(&config.influxdb_config);

    // Initialize the async runtime
    let rt = Runtime::new().unwrap();
    rt.block_on(Statistics::init_influxdb_client(&config.influxdb_config));

    let _logger_guard = utils::log::init_logger(
        config.log_config.log_level.clone(),
        config.log_config.log_format.clone(),
        config.log_config.log_path.clone(),
        config.log_config.log_filename.clone(),
    );

    let mut server = Server::new(Some(Opt {
        conf: std::env::var("SERVER_CONF").ok(),
        ..Default::default()
    }))
    .expect("Failed to create server");
    server.bootstrap();

    let fp_handler = ForwardHandler::new(config.handler_config);

    let mut proxy = http_proxy_service(
        &server.configuration,
        ForwardProxy::new(config.tls_config, fp_handler),
    );

    proxy.add_tcp(&format!("{}:{}", config.listen_address, config.listen_port));

    server.add_service(proxy);

    info!(
        "Starting server at {}:{}",
        config.listen_address, config.listen_port
    );

    server.run_forever();
}
