use crate::config::ProxyConfig;
use crate::handler::common::consts::LogTypes;
use async_trait::async_trait;
use bytes::Bytes;
use pingora::http::{ResponseHeader, StatusCode};
use pingora::prelude::{HttpPeer, ProxyHttp};
use pingora::proxy::Session;
use pingora_router::ctx::{Layer8Context, Layer8ContextTrait};
use pingora_router::router::Router;
use tracing::{debug, info};

/// Reverse proxy server for routing and processing HTTP requests.
///
/// # Type Parameters
/// * `T` — data type for the router
///
/// # Fields
/// * `config` — proxy server configuration
/// * `router` — router for request handling
///
/// # Examples
/// ```ignore
/// let config = ProxyConfig::default();
/// let router = Router::new();
/// let proxy = ReverseProxy::new(config, router);
/// ```
pub struct ReverseProxy<T> {
    config: ProxyConfig,
    router: Router<T>,
}

impl<T> ReverseProxy<T> {
    pub fn new(config: ProxyConfig, router: Router<T>) -> Self {
        ReverseProxy { config, router }
    }

    /// Sets response headers for the proxy session.
    ///
    /// This method constructs and writes response headers to the client session, including:
    /// - CORS headers based on configuration and request origin
    /// - Common headers (Content-Type, Access-Control headers)
    /// - Custom headers from the context response
    ///
    /// # Arguments
    /// * `session` — the client session to write headers to
    /// * `ctx` — the Layer8 context containing request/response data
    /// * `response_status` — the HTTP status code for the response
    ///
    /// # Returns
    /// * `Ok(())` on successful header write
    /// * `Err(pingora::Error)` if header building or writing fails
    async fn set_headers(
        &self,
        session: &mut Session,
        ctx: &mut Layer8Context,
        response_status: StatusCode,
    ) -> pingora::Result<()> {
        let mut header = ResponseHeader::build(response_status, None)?;
        ctx.response.status = response_status; // store status in context for logging

        let response_header = ctx.get_response_header().clone();
        for (key, val) in response_header.iter() {
            header
                .insert_header(key.clone(), val.clone())
                .unwrap_or_default();
        }

        // Common headers
        header
            .insert_header("Content-Type", "application/json")
            .unwrap_or_default();
        header
            .insert_header("Access-Control-Allow-Methods", "*")
            .unwrap_or_default();
        header
            .insert_header("Access-Control-Max-Age", "86400")
            .unwrap_or_default();

        header
            .insert_header(
                "Access-Control-Allow-Credentials",
                self.config.cors_allow_credentials.to_string(),
            )
            .unwrap_or_default();

        if let Some(origin) = ctx.request.header.get("origin")
            && self
                .config
                .cors_allow_origins
                .iter()
                .any(|allowed| allowed == origin)
        {
            header
                .insert_header("Access-Control-Allow-Origin", origin.to_string())
                .unwrap_or_default();
        }

        if let Some(req_headers) = ctx.request.header.get("Access-Control-Request-Headers") {
            header
                .insert_header("Access-Control-Allow-Headers", req_headers)
                .unwrap_or_default();
        }

        let correlation_id = ctx.get_correlation_id();
        debug!(
            %correlation_id,
            log_type=LogTypes::HANDLE_BACKEND_RESPONSE,
            "Response Headers: {:?}",
            header.headers
        );
        session.write_response_header_ref(&header, false).await
    }
}

#[async_trait]
impl<T: Sync> ProxyHttp for ReverseProxy<T> {
    type CTX = Layer8Context;

    fn new_ctx(&self) -> Self::CTX {
        Layer8Context::default()
    }

    /// This method is required by the `ProxyHttp` trait but is not invoked in the current
    /// implementation, as all request handling is completed in `request_filter` before
    /// upstream communication occurs.
    async fn upstream_peer(
        &self,
        _session: &mut Session,
        _ctx: &mut Self::CTX,
    ) -> pingora::Result<Box<HttpPeer>> {
        let peer: Box<HttpPeer> = Box::new(HttpPeer::new("", false, "".to_string()));
        Ok(peer)
    }

    /// Handle incoming HTTP requests by routing and generating responses.
    ///
    /// This method processes incoming HTTP requests by:
    /// 1. Updating the context with session data and reading the complete request body
    /// 2. Routing the request to the appropriate handler via the router
    /// 3. Handling 404 responses by writing a default NOT_FOUND header
    /// 4. Processing the handler response (status, body, cookies)
    /// 5. Setting response headers including CORS headers
    /// 6. Writing the response body back to the client
    ///
    /// # Arguments
    /// * `session` — the client session containing request/response data
    /// * `ctx` — the Layer8 context for storing request/response state
    ///
    /// # Returns
    /// * `Ok(true)` on successful request processing (request fully handled)
    /// * `Err(pingora::Error)` if any step fails (context update, routing, header/body write)
    async fn request_filter(
        &self,
        session: &mut Session,
        ctx: &mut Self::CTX,
    ) -> pingora::Result<bool>
    where
        Self::CTX: Send + Sync,
    {
        // create Context
        ctx.update(session).await?;
        ctx.read_request_body(session).await?;

        let handler_response = self.router.call_handler(ctx).await;
        if handler_response.status == StatusCode::NOT_FOUND && handler_response.body.is_none() {
            let header = ResponseHeader::build(StatusCode::NOT_FOUND, None)?;
            session.write_response_header_ref(&header, false).await?;
            session.set_keepalive(None);
            return Ok(true);
        }

        let mut response_bytes = vec![];
        if let Some(body_bytes) = handler_response.body {
            ctx.insert_response_header("Content-length", &body_bytes.len().to_string());
            response_bytes = body_bytes;
        };

        // set cookies to response
        if let Some(cookies) = handler_response.cookies {
            ctx.insert_response_header("Set-Cookie", &cookies);
        }
        self.set_headers(session, ctx, handler_response.status)
            .await?;
        ctx.set_response_body(response_bytes.clone()); // store response body in context for logging

        // Write the response body to the session after setting headers
        session
            .write_response_body(Some(Bytes::from(response_bytes)), true)
            .await?;

        Ok(true)
    }

    /// Log details about the completed request/response transaction.
    ///
    /// This method captures comprehensive information about the HTTP request lifecycle including:
    /// - Correlation ID for request tracing
    /// - Request summary (method, path, protocol)
    /// - Response status code (from context or session if error occurred)
    /// - Request headers (origin, referer, user-agent)
    /// - Response body size
    /// - Request latency in milliseconds
    /// - Any errors that occurred during processing
    ///
    /// # Arguments
    /// * `session` — the client session containing request/response data
    /// * `e` — optional error that occurred during request processing
    /// * `ctx` — the Layer8 context with request/response state and metrics
    ///
    /// # Note
    /// This method is called after request processing completes, regardless of success or failure.
    async fn logging(
        &self,
        session: &mut Session,
        e: Option<&pingora::Error>,
        ctx: &mut Self::CTX,
    ) {
        let mut status = ctx.response.status.as_u16();
        if let Some(_err) = e {
            status = session.response_written().unwrap().status.as_u16();
        }
        let correlation_id = ctx.get_correlation_id();

        info!(
            %correlation_id,
            log_type=LogTypes::ACCESS_LOG,
            status=status,
            request_summary = session.request_summary(),
            origin = ctx.request.header.get("origin"),
            referer = ctx.request.header.get("referer"),
            latency_micros=ctx.get_latency().as_micros() as i64,
            response_body_size=ctx.get_response_body().len(),
            user_agent=ctx.request.header.get("User-Agent"),
            error=?e,
        );
    }
}
