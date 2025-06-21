use pingora::prelude::{HttpPeer, ProxyHttp};
use pingora::proxy::Session;
use pingora::http::{ResponseHeader, StatusCode};
use log::{info};
use async_trait::async_trait;
use bytes::Bytes;
use pingora_router::ctx::{Layer8Context, Layer8ContextTrait};
use pingora_router::router::Router;

const UPSTREAM_HOST: &str = "localhost";
pub const UPSTREAM_IP: &str = "0.0.0.0";
pub const BACKEND_PORT: u16 = 3000;

pub struct ReverseProxy<T> {
    addr: std::net::SocketAddr,
    router: Router<T>
}

impl<T> ReverseProxy<T> {

    pub fn new(addr: std::net::SocketAddr, router: Router<T>) -> Self {
        ReverseProxy {
            addr,
            router
        }
    }

    async fn set_headers(
        session: &mut Session,
        ctx: &mut Layer8Context,
        response_status: StatusCode
    ) -> pingora::Result<()> {
        let mut header = ResponseHeader::build(response_status, None)?;

        let response_header = ctx.get_response_header().clone();
        for (key, val) in response_header.iter() {
            header.insert_header(key.clone(), val.clone()).unwrap();
        };

        // Common headers
        header.insert_header("Content-Type", "application/json").unwrap();
        header
            .insert_header("Access-Control-Allow-Origin", "*")
            .unwrap();
        header
            .insert_header("Access-Control-Allow-Methods", "*")
            .unwrap();
        header
            .insert_header("Access-Control-Allow-Headers", "*")
            .unwrap();
        header
            .insert_header("Access-Control-Max-Age", "86400")
            .unwrap();

        println!();
        info!("[RESPONSE {} {}] Header: {:?}", session.req_header().method,
            session.req_header().uri.to_string(), header.headers);
        session.write_response_header_ref(&header).await
    }
}

#[async_trait]
impl<T: Sync> ProxyHttp for ReverseProxy<T> {
    type CTX = Layer8Context;

    fn new_ctx(&self) -> Self::CTX {
        Layer8Context::default()
    }

    async fn upstream_peer(
        &self,
        _session: &mut Session,
        _ctx: &mut Self::CTX,
    ) -> pingora::Result<Box<HttpPeer>> {
        let peer: Box<HttpPeer> =
            Box::new(HttpPeer::new(self.addr, false, UPSTREAM_HOST.to_owned()));
        Ok(peer)
    }

    async fn request_filter(&self, session: &mut Session, ctx: &mut Self::CTX) -> pingora::Result<bool>
    where
        Self::CTX: Send + Sync,
    {
        // create Context
        ctx.update(session).await?;
        let request_summary = format!("{} {}", session.req_header().method, session.req_header().uri.to_string());
        println!();
        info!("[REQUEST {}] {:?}", request_summary, ctx.request);
        info!("[REQUEST {}] Decoded body: {}", request_summary, String::from_utf8_lossy(&*ctx.get_request_body()));
        println!();

        let handler_response = self.router.call_handler(ctx).await;

        let mut response_bytes = vec![];
        if let Some(body_bytes) = handler_response.body {
            ctx.insert_response_header("Content-length", &body_bytes.len().to_string());
            response_bytes = body_bytes;
        };
        ReverseProxy::<T>::set_headers(session, ctx, handler_response.status).await?;

        info!("[RESPONSE {}] Body: {}", request_summary, String::from_utf8_lossy(&*response_bytes));
        println!();

        // Write the response body to the session after setting headers
        session.write_response_body(Some(Bytes::from(response_bytes)), true).await?;

        Ok(true)
    }

    async fn logging(
        &self,
        session: &mut Session,
        _e: Option<&pingora::Error>,
        ctx: &mut Self::CTX,
    ) {
        let response_code = session
            .response_written()
            .map_or(0, |resp| resp.status.as_u16());
        // access log
        info!(
            "{} response code: {response_code}",
            self.request_summary(session, ctx)
        );
    }
}
