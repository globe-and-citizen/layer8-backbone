mod utils;
mod proxy;
mod router;
mod handler;

use pingora::prelude::*;
use simplelog::{ConfigBuilder, LevelFilter, WriteLogger};
use std::fs;
use log::info;
use proxy::ForwardProxy;
use crate::handler::ForwardHandler;

fn main() {
    // Load environment variables from .env file
    dotenv::dotenv().ok();
    // Initialize logger
    let log_file = fs::File::create("log.txt").expect("Failed to create log file");
    let config = ConfigBuilder::new().set_time_to_local(true).build();
    WriteLogger::init(LevelFilter::Debug, config, log_file).expect("Failed to initialize logger");

    info!("Starting server...");

    let mut server = Server::new(None).unwrap();
    server.bootstrap();

    let fp_handler = ForwardHandler {};
    let mut router = router::Router::new(fp_handler);
    let mut proxy = http_proxy_service(&server.configuration, ForwardProxy::new(router));

    proxy.add_tcp("0.0.0.0:6191");

    server.add_service(proxy);

    server.run_forever();
}
