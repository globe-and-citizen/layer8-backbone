mod proxy;
mod handler;

use crate::handler::ForwardHandler;
use env_logger::{Env, Target};
use log::info;
use proxy::ForwardProxy;
use pingora::prelude::*;

fn main() {
    // Load environment variables from .env file
    dotenv::dotenv().ok();

    // let log_file = fs::File::create("log.txt").expect("Failed to create log file");
    // let config = ConfigBuilder::new().set_time_to_local(true).build();
    // WriteLogger::init(LevelFilter::Debug, config, log_file).expect("Failed to initialize logger");
    env_logger::Builder::from_env(Env::default()
        .write_style_or("RUST_LOG_STYLE", "always"))
        .format_file(true)
        .format_line_number(true)
        .target(Target::Stdout)
        .init();

    info!("Starting server...");

    let mut server = Server::new(Some(Opt {
        conf: std::env::var("SERVER_CONF").ok(),
        ..Default::default()
    })).unwrap();
    server.bootstrap();

    let fp_handler = ForwardHandler{};

    let mut proxy = http_proxy_service(
        &server.configuration,
        ForwardProxy::new(fp_handler)
    );

    proxy.add_tcp("localhost:6191");

    server.add_service(proxy);

    server.run_forever();
}
