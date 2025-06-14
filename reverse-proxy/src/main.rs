mod handler;
mod proxy;

use clap::Parser;
use std::net::ToSocketAddrs;

use pingora::prelude::{http_proxy_service};
use pingora::server::Server;
use pingora::server::configuration::Opt;

use chrono::Local;
use env_logger;
use log::*;
use std::io::Write;
use std::sync::Arc;
use pingora_router::handler::{APIHandler};
use pingora_router::router::Router;
use proxy::{BACKEND_PORT, ReverseProxy, UPSTREAM_IP};
use crate::handler::ReverseHandler;
use futures::FutureExt;

fn main() {
    // let file = OpenOptions::new()
    //     .append(true)
    //     .create(true)
    //     .open("log.txt")
    //     .expect("Can't create file!");
    //
    // let target = Box::new(file);

    let target = env_logger::Target::Stdout;
    env_logger::Builder::new()
        .target(target)
        .filter(None, LevelFilter::Debug)
        .format(|buf, record| {
            writeln!(
                buf,
                "[{} {} {}:{}] {}",
                Local::now().format("%Y-%m-%d %H:%M:%S%.3f"),
                record.level(),
                record.file().unwrap_or("unknown"),
                record.line().unwrap_or(0),
                record.args()
            )
        })
        .init();

    let opt = Opt::parse();
    let mut my_server = Server::new(Some(opt)).unwrap();
    my_server.bootstrap();

    let handle_init_tunnel: APIHandler<Arc<ReverseHandler>> = Box::new(|h, ctx| {
        async move { h.handle_init_tunnel(ctx).await }.boxed()
    });

    let handle_proxy: APIHandler<Arc<ReverseHandler>> = Box::new(|h, ctx| {
        async move { h.handle_proxy_request(ctx).await }.boxed()
    });

    let rp_handler = Arc::new(ReverseHandler{});
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

    // Listen on both endpoints
    my_proxy.add_tcp("0.0.0.0:6193");  // Publicly accessible
    my_proxy.add_tcp("127.0.0.1:6194"); // Localhost only

    my_server.add_service(my_proxy);
    my_server.run_forever();
}