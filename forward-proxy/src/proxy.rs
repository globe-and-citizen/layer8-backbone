use async_trait::async_trait;
use bytes::Bytes;
use log::debug;
use pingora::prelude::{HttpPeer, ProxyHttp, Session};
use pingora::http::{ResponseHeader, StatusCode};
use pingora_router::ctx::{Layer8Context, Layer8ContextTrait};
use pingora_router::router::Router;

pub struct ForwardProxy<T> {
    router: Router<T>,
}

impl<T> ForwardProxy<T> {
    pub fn new(router: Router<T>) -> Self {
        ForwardProxy {
            router
        }
    }
}

#[async_trait]
impl<T: Sync> ProxyHttp for ForwardProxy<T> {
    type CTX = Layer8Context;

    fn new_ctx(&self) -> Self::CTX {
        Layer8Context::default()
    }

    async fn upstream_peer(
        &self,
        _session: &mut Session,
        _ctx: &mut Self::CTX,
    ) -> pingora::Result<Box<HttpPeer>> {
        Ok(Box::from(HttpPeer::new(
            String::from("127.0.0.1:6193"),
            false,
            String::from(""),
        )))
    }

    async fn request_filter(&self, session: &mut Session, ctx: &mut Self::CTX) -> pingora::Result<bool>
    where
        Self::CTX: Send + Sync,
    {
        // create Context
        ctx.update(session).await?;

        let handler_response = self.router.call_handler(ctx).await;
        if handler_response.status == StatusCode::NOT_FOUND && handler_response.body == None {
            return Ok(false)
        }
        debug!("handler_response: {:?}", handler_response);

        // set headers
        let mut header = ResponseHeader::build(handler_response.status, None)?;
        let response_header = ctx.get_response_header().clone();
        debug!("handler response_header: {:?}", response_header);
        for (key, val) in response_header.iter() {
            header.insert_header(key.clone(), val.clone()).unwrap();
        };

        ctx.insert_response_header("Content-Type", "application/json");
        let mut response_bytes = vec![];
        if let Some(body_bytes) = handler_response.body {
            header.insert_header("Content-length", &body_bytes.len().to_string()).unwrap();
            response_bytes = body_bytes;
            debug!("response data: {:?}", String::from_utf8_lossy(&*response_bytes));
        };

        session.write_response_header_ref(&header).await?;

        // Write the response body to the session after setting headers
        session.write_response_body(Some(Bytes::from(response_bytes)), true).await?;

        Ok(true)
    }

    async fn response_filter(
        &self,
        _session: &mut Session,
        upstream_response: &mut ResponseHeader,
        _ctx: &mut Self::CTX,
    ) -> pingora::Result<()> {
        upstream_response.insert_header("Access-Control-Allow-Origin", "*")?;
        upstream_response.insert_header("Access-Control-Allow-Methods", "GET, POST")?;
        upstream_response.insert_header("Access-Control-Allow-Headers", "Content-Type")?;
        Ok(())
    }
}
