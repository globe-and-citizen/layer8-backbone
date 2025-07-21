mod proxy;
mod handler;
mod config;

use std::fs::OpenOptions;
use crate::handler::ForwardHandler;
use env_logger::{Env, Target};
use log::{debug, info, LevelFilter};
use proxy::ForwardProxy;
use pingora::prelude::*;
use crate::config::FPConfig;

fn load_config() -> FPConfig {
    // Load environment variables from .env file
    dotenv::dotenv().ok();

    // Deserialize from env vars
    let config: FPConfig = envy::from_env().unwrap();

    let target = match config.log_config.log_path.as_str() {
        "console" => Target::Stdout,
        path => {
            let file = OpenOptions::new()
                .append(true)
                .create(true)
                .open(path)
                .expect("Can't create log file!");

            Target::Pipe(Box::new(file))
        }
    };

    env_logger::Builder::from_env(Env::default()
        .write_style_or("RUST_LOG_STYLE", "always"))
        .format_file(true)
        .format_line_number(true)
        .filter(None, config.log_config.to_level_filter())
        .target(target)
        .init();

    debug!("Loaded ForwardProxyConfig: {:?}", config);
    config
}

fn main() {
    let config = load_config();

    info!("Starting server...");

    let mut server = Server::new(Some(Opt {
        conf: std::env::var("SERVER_CONF").ok(),
        ..Default::default()
    })).unwrap();
    server.bootstrap();

    let fp_handler = ForwardHandler::new(config.handler_config);

    let mut proxy = http_proxy_service(
        &server.configuration,
        ForwardProxy::new(fp_handler)
    );

    proxy.add_tcp(&format!("{}:{}", config.listen_address, config.listen_port));

    server.add_service(proxy);

    server.run_forever();
}
