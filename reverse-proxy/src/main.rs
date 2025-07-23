mod handler;
mod proxy;
mod tls_conf;

use std::env;
use std::net::ToSocketAddrs;

use crate::handler::ReverseHandler;
use crate::tls_conf::TlsConfig;
use env_logger::{self, Env, Target};
use futures::FutureExt;
use pingora::server::Server;
use pingora::server::configuration::Opt;
use pingora::{listeners::tls::TlsSettings, prelude::http_proxy_service};
use pingora_router::handler::APIHandler;
use pingora_router::router::Router;
use proxy::{BACKEND_PORT, ReverseProxy, UPSTREAM_IP};
use std::sync::Arc;
mod config;

fn main() {
    // let file = OpenOptions::new()
    //     .append(true)
    //     .create(true)
    //     .open("log.txt")
    //     .expect("Can't create file!");
    //
    // let target = Box::new(file);

    // let target = env_logger::Target::Stdout;
    // env_logger::Builder::new()
    //     .target(target)
    //     .filter(None, LevelFilter::Debug)
    //     .format(|buf, record| {
    //         writeln!(
    //             buf,
    //             "[{} {} {}:{}] {}",
    //             Local::now().format("%Y-%m-%d %H:%M:%S%.3f"),
    //             record.level(),
    //             record.file().unwrap_or("unknown"),
    //             record.line().unwrap_or(0),
    //             record.args()
    //         )
    //     })
    //     .init();

    // Load environment variables from .env file
    dotenv::dotenv().ok();

    env_logger::Builder::from_env(Env::default()
        .write_style_or("RUST_LOG_STYLE", "always"))
        .format_file(true)
        .format_line_number(true)
        .target(Target::Stdout)
        .init();

    let mut my_server = Server::new(Some(Opt {
        conf: std::env::var("SERVER_CONF").ok(),
        ..Default::default()
    })).unwrap();

    my_server.bootstrap();

    let handle_init_tunnel: APIHandler<Arc<ReverseHandler>> =
        Box::new(|h, ctx| async move { h.handle_init_tunnel(ctx).await }.boxed());

    let handle_proxy: APIHandler<Arc<ReverseHandler>> =
        Box::new(|h, ctx| async move { h.handle_proxy_request(ctx).await }.boxed());

    // todo: consider switching to .env
    let config_path = env::var("CONFIG_PATH").unwrap_or_else(|_| "config.toml".to_string());
    let backbone_config = config::RPConfig::from_file(&config_path);
    backbone_config.validate();
    println!("{:?}", backbone_config);

    let rp_handler = Arc::new(ReverseHandler::new(backbone_config));
    let mut router: Router<Arc<ReverseHandler>> = Router::new(rp_handler.clone());
    router.post("/init-tunnel".to_string(), Box::new([handle_init_tunnel]));
    router.post("/proxy".to_string(), Box::new([handle_proxy]));

    let upstream_addr = (UPSTREAM_IP.to_owned(), BACKEND_PORT)
        .to_socket_addrs()
        .unwrap()
        .next()
        .unwrap();

    let mut my_proxy = http_proxy_service(
        &my_server.configuration,
        ReverseProxy::new(upstream_addr, router),
    );

    my_proxy.add_tls_with_settings(
        "localhost:6193",
        None,
        TlsSettings::with_callbacks(Box::new(TlsConfig)).unwrap(),
    );

    // Listen on both endpoints
    // my_proxy.add_tcp("0.0.0.0:6193"); // Publicly accessible
    // my_proxy.add_tcp("127.0.0.1:6194"); // Localhost only

    my_server.add_service(my_proxy);
    my_server.run_forever();
}
