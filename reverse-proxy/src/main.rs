mod handler;
mod proxy;
mod tls_conf;

use std::fs::OpenOptions;
use crate::handler::ReverseHandler;
use env_logger::{self, Env, Target};
use futures::FutureExt;
use pingora::server::Server;
use pingora::server::configuration::Opt;
use pingora::{listeners::tls::TlsSettings, prelude::http_proxy_service};
use pingora_router::handler::APIHandler;
use pingora_router::router::Router;
use std::sync::Arc;
use log::{debug, error};
use crate::config::RPConfig;
use crate::proxy::ReverseProxy;

mod config;

fn load_config() -> RPConfig {
    // Load environment variables from .env file
    dotenv::dotenv().ok();

    // Deserialize from env vars
    let config: RPConfig = envy::from_env().map_err(|e| {
        error!("Failed to load configuration: {}", e);
    }).unwrap();

    let target = match config.log.log_path.as_str() {
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
        .filter(None, config.log.to_level_filter())
        .target(target)
        .init();

    debug!("Loaded ReverseProxyConfig: {:?}", config);
    config
}

fn main() {
    // Load environment variables from .env file
    let rp_config = load_config();

    let mut my_server = Server::new(Some(Opt {
        conf: std::env::var("SERVER_CONF").ok(),
        ..Default::default()
    })).unwrap();
    my_server.bootstrap();

    let handle_init_tunnel: APIHandler<Arc<ReverseHandler>> =
        Box::new(|h, ctx| async move { h.handle_init_tunnel(ctx).await }.boxed());

    let handle_proxy: APIHandler<Arc<ReverseHandler>> =
        Box::new(|h, ctx| async move { h.handle_proxy_request(ctx).await }.boxed());

    let handle_healthcheck: APIHandler<Arc<ReverseHandler>> =
        Box::new(|h, ctx| async move { h.handle_healthcheck(ctx).await }.boxed());

    let rp_handler = Arc::new(ReverseHandler::new(rp_config.clone()));
    let mut router: Router<Arc<ReverseHandler>> = Router::new(rp_handler.clone());
    router.post("/init-tunnel".to_string(), Box::new([handle_init_tunnel]));
    router.post("/proxy".to_string(), Box::new([handle_proxy]));
    router.get("/healthcheck".to_string(), Box::new([handle_healthcheck]));

    let mut my_proxy = http_proxy_service(
        &my_server.configuration,
        ReverseProxy::new(router),
    );

    if rp_config.tls.enable_tls {
        my_proxy.add_tls_with_settings(
            &format!(
                "{}:{}",
                rp_config.server.listen_address,
                rp_config.server.listen_port
            ),
            None,
            TlsSettings::with_callbacks(Box::new(rp_config.tls)).expect("Cannot set TlsSettings callbacks")
        );
    } else {
        my_proxy.add_tcp(&format!(
            "{}:{}",
            rp_config.server.listen_address,
            rp_config.server.listen_port
        ));
    }

    // Listen on both endpoints
    // my_proxy.add_tcp("0.0.0.0:6193"); // Publicly accessible
    // my_proxy.add_tcp("127.0.0.1:6194"); // Localhost only

    my_server.add_service(my_proxy);
    my_server.run_forever();
}
