use pingora::prelude::*;
use proxy::ForwardProxy;
use tracing::{debug, info};

use crate::config::FPConfig;
use crate::handler::ForwardHandler;

mod config;
mod handler;
mod proxy;

fn load_config() -> FPConfig {
    // Load environment variables from .env file
    dotenv::dotenv().ok();

    // Deserialize from env vars
    let mut config: FPConfig = envy::from_env().expect("Failed to load config");

    config
        .tls_config
        .load()
        .expect("Failed to load TLS configuration");

    // let target = match config.log_config.log_path.as_str() {
    //     "console" => Target::Stdout,
    //     path => {
    //         let file = OpenOptions::new()
    //             .append(true)
    //             .create(true)
    //             .open(path)
    //             .expect("Can't create log file!");

    //         Target::Pipe(Box::new(file))
    //     }
    // };

    // env_logger::Builder::from_env(Env::default()
    //     .write_style_or("RUST_LOG_STYLE", "always"))
    //     .format_file(true)
    //     .format_line_number(true)
    //     .filter(None, config.log_config.to_level_filter())
    //     .target(target)
    //     .init();

    debug!("Loaded ForwardProxyConfig: {:?}", config);
    config
}

fn main() {
    #[cfg(feature = "hotpath")]
    let guard = hotpath::GuardBuilder::new("guard_timeout::main")
        .percentiles(&[50, 95, 99])
        .build();

    #[cfg(feature = "hotpath")]
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_secs(60));
        drop(guard);
        std::process::exit(0);
    });

    let config = load_config();

    info!("Starting server...");

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

    server.run_forever();
}
