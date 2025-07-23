mod proxy;
mod handler;

use crate::handler::{ForwardConfig, ForwardHandler};
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

    let config = {
        let jwt_secret = std::env::var("JWT_VIRTUAL_CONNECTION_KEY")
            .expect("JWT_VIRTUAL_CONNECTION_KEY must be set");

        let jwt_exp = std::env::var("JWT_EXP_IN_HOUR")
            .expect("JWT_EXP_IN_HOUR must be set")
            .parse::<i64>()
            .expect("JWT_EXP_IN_HOUR must be a valid integer");

        let auth_server_token = std::env::var("AUTH_SERVER_TOKEN")
            .expect("AUTH_SERVER_TOKEN must be set");

        ForwardConfig {
            jwt_secret: jwt_secret.as_bytes().to_vec(),
            jwt_exp_in_hours: jwt_exp,
            auth_access_token: auth_server_token,
        }
    };

    let fp_handler = ForwardHandler::new(config);

    let mut proxy = http_proxy_service(
        &server.configuration,
        ForwardProxy::new(fp_handler)
    );

    proxy.add_tcp("localhost:6191");

    server.add_service(proxy);

    server.run_forever();
}
