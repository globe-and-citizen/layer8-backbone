mod proxy;
mod handler;
mod config;

use crate::handler::ForwardHandler;
use proxy::ForwardProxy;
use pingora::prelude::*;
use crate::config::FPConfig;
use tracing::{info, debug};

fn load_config() -> FPConfig {
    // Load environment variables from .env file
    dotenv::dotenv().ok();

    // Deserialize from env vars
    let config: FPConfig = envy::from_env().expect("Failed to load config");

    utils::log::init_logger(
        "ForwardProxy",
        config.log_config.log_level.clone(),
        config.log_config.log_path.clone(),
    );

    debug!(name: "FPConfig", value = ?config);
    config
}

fn main() {
    let config = load_config();

    let mut server = Server::new(Some(Opt {
        conf: std::env::var("SERVER_CONF").ok(),
        ..Default::default()
    })).expect("Failed to create server");
    server.bootstrap();

    let fp_handler = ForwardHandler::new(config.handler_config);

    let mut proxy = http_proxy_service(
        &server.configuration,
        ForwardProxy::new(config.tls_config, fp_handler),
    );

    proxy.add_tcp(&format!("{}:{}", config.listen_address, config.listen_port));

    server.add_service(proxy);

    info!("Starting server at {}:{}", config.listen_address, config.listen_port);

    server.run_forever();
}
