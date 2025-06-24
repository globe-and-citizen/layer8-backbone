mod handler;
mod proxy;

use crate::handler::ForwardHandler;
use env_logger::{Env, Target};
use futures::FutureExt;
use log::info;
use pingora::prelude::*;
use pingora_router::handler::APIHandler;
use pingora_router::router::Router;
use proxy::ForwardProxy;
use std::sync::Arc;

fn main() {
    // Load environment variables from .env file
    dotenv::dotenv().ok();

    // let log_file = fs::File::create("log.txt").expect("Failed to create log file");
    // let config = ConfigBuilder::new().set_time_to_local(true).build();
    // WriteLogger::init(LevelFilter::Debug, config, log_file).expect("Failed to initialize logger");
    env_logger::Builder::from_env(Env::default().write_style_or("RUST_LOG_STYLE", "always"))
        .format_file(true)
        .format_line_number(true)
        .target(Target::Stdout)
        .init();

    info!("Starting server...");

    let mut server = Server::new(Some(Opt {
        conf: std::env::var("SERVER_CONF").ok(),
        ..Default::default()
    }))
    .unwrap();
    server.bootstrap();

    let fp_handler = Arc::new(ForwardHandler {});
    let mut router: Router<Arc<ForwardHandler>> = Router::new(fp_handler.clone());

    let handle_init_tunnel: APIHandler<Arc<ForwardHandler>> =
        Box::new(|h, ctx| async move { h.handle_init_encrypted_tunnel(ctx).await }.boxed());

    let handle_proxy: APIHandler<Arc<ForwardHandler>> =
        Box::new(|h, ctx| async move { h.handle_proxy(ctx).await }.boxed());

    let handle_healthcheck: APIHandler<Arc<ForwardHandler>> =
        Box::new(|h, ctx| async move { h.handle_healthcheck(ctx).await }.boxed());

    router.post(
        "/init-tunnel?backend_url={}".to_string(),
        Box::new([handle_init_tunnel]),
    );
    router.post("/proxy".to_string(), Box::new([handle_proxy]));
    router.get(
        "/healthcheck?error={}".to_string(),
        Box::new([handle_healthcheck]),
    );

    let mut proxy = http_proxy_service(&server.configuration, ForwardProxy::new(router));
    proxy.add_tcp("localhost:6191");
    server.add_service(proxy);
    server.run_forever();
}
