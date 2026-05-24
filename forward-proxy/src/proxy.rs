use crate::config::ProxyConfig;
use crate::handler::ForwardHandler;
use crate::handler::consts::{CtxKeys, HeaderKeys, LogTypes, RequestPaths};
use crate::handler::types::response::ErrorResponse;
use crate::statistics::Statistics;
use async_trait::async_trait;
use bytes::Bytes;
use pingora::OrErr;
use pingora::http::{RequestHeader, ResponseHeader, StatusCode};
use pingora::prelude::{HttpPeer, ProxyHttp, Session};
use pingora::upstreams::peer::PeerOptions;
use pingora::{Error, ErrorType};
use pingora_router::ctx::{Layer8Context, Layer8ContextTrait};
use pingora_router::handler::ResponseBodyTrait;
use reqwest::header::TRANSFER_ENCODING;
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, error, info};
use utils::cert::TLSCredentials;

pub struct ForwardProxy {
    config: ProxyConfig,
    tls_credentials: Arc<TLSCredentials>,
    handler: ForwardHandler,
}

impl ForwardProxy {
    pub fn new(
        config: ProxyConfig,
        tls_credentials: Arc<TLSCredentials>,
        handler: ForwardHandler,
    ) -> Self {
        ForwardProxy {
            config,
            tls_credentials,
            handler,
        }
    }

    /// Sets CORS (Cross-Origin Resource Sharing) response headers.
    /// This function is responsible for:
    /// - Adding Access-Control-Allow-Credentials header based on config
    /// - Setting Access-Control-Allow-Methods to allow common HTTP methods
    /// - Setting Access-Control-Max-Age to 24 hours (86400 seconds)
    /// - Setting Access-Control-Allow-Origin header if the origin is in the allowlist
    ///
    /// # Arguments
    /// * `ctx` - The request context for retrieving the origin from the request headers
    /// * `response` - The response header to add CORS headers to
    ///
    /// # Returns
    /// * `Ok(())` - Successfully added all CORS headers
    /// * `Err(Error)` - If an error occurred while inserting headers
    pub fn set_response_header(
        &self,
        ctx: &Layer8Context,
        response: &mut ResponseHeader,
    ) -> pingora::Result<()> {
        response.insert_header(
            "Access-Control-Allow-Credentials",
            self.config.cors_allow_credentials.to_string(),
        )?;
        response.insert_header(
            "Access-Control-Allow-Methods",
            "GET, POST, PUT, DELETE, OPTIONS",
        )?;
        response.insert_header("Access-Control-Max-Age", "86400")?;

        if let Some(origin) = ctx.request.header.get("origin") {
            if self
                .config
                .cors_allow_origins
                .iter()
                .any(|allowed| allowed == origin)
            {
                response.insert_header("Access-Control-Allow-Origin", origin.to_string())?;
            }
        }

        Ok(())
    }

    /// Handles CORS preflight (OPTIONS) requests.
    /// This function is responsible for:
    /// - Setting the response status to 204 No Content
    /// - Extracting Access-Control-Request-Headers from the request if present
    /// - Setting CORS response headers (origin, credentials, methods, max age)
    /// - Writing the response headers and keeping the connection alive
    ///
    /// # Arguments
    /// * `ctx` - The request context for retrieving and setting response data
    /// * `session` - The HTTP session for writing the response
    ///
    /// # Returns
    /// * `Ok(true)` - Successfully handled the preflight request
    /// * `Err(Error)` - If an error occurred while processing or writing the response
    pub async fn handle_preflight_request(
        &self,
        ctx: &mut Layer8Context,
        session: &mut Session,
    ) -> pingora::Result<bool> {
        // Handle CORS preflight request
        ctx.response.status = StatusCode::NO_CONTENT;
        let mut header = ResponseHeader::build(StatusCode::NO_CONTENT, None)?;
        if let Some(req_headers) = session
            .req_header()
            .headers
            .get("Access-Control-Request-Headers")
        {
            header.insert_header("Access-Control-Allow-Headers", req_headers)?;
        }
        self.set_response_header(ctx, &mut header)?;

        session.write_response_header_ref(&header, false).await?;
        session.set_keepalive(None);
        Ok(true)
    }

    /// Handles healthcheck requests.
    /// This function is responsible for:
    /// - Calling the handler to process the healthcheck request
    /// - Building a response header with the appropriate status code
    /// - Adding response headers from the handler response
    /// - Setting the Content-Length header if a response body is present
    /// - Writing the response header and body to the session
    ///
    /// # Arguments
    /// * `ctx` - The request context for retrieving and setting response data
    /// * `session` - The HTTP session for writing the response
    ///
    /// # Returns
    /// * `Ok(true)` - Successfully handled the healthcheck request
    /// * `Err(Error)` - If an error occurred while processing or writing the response
    pub async fn handle_healthcheck(
        &self,
        ctx: &mut Layer8Context,
        session: &mut Session,
    ) -> pingora::Result<bool> {
        let correlation_id = ctx.get_correlation_id();
        let handler_response = self.handler.handle_healthcheck(ctx);
        let mut header = ResponseHeader::build(handler_response.status, None)?;
        let response_headers = header.headers.clone();
        for (key, val) in response_headers.iter() {
            header
                .insert_header(key.clone(), val.clone())
                .map_err(|e| {
                    error!(
                        %correlation_id,
                        log_type = LogTypes::HEALTHCHECK,
                        "Cannot add request header {}:{:?}, err: {:?}",
                        key.clone(), val.clone(), e
                    )
                })
                .unwrap_or_default();
        }

        let mut response_bytes = vec![];
        if let Some(body_bytes) = handler_response.body {
            header
                .insert_header("Content-length", &body_bytes.len().to_string())
                .unwrap_or_default();
            response_bytes = body_bytes;
        };

        session.write_response_header_ref(&header, false).await?;
        // Write the response body to the session after setting headers
        session
            .write_response_body(Some(Bytes::from(response_bytes)), true)
            .await?;

        Ok(true)
    }

    /// Validates and processes init-tunnel requests.
    /// This function is responsible for:
    /// - Extracting and validating the `backend_url` query parameter
    /// - Resolving the backend URL to socket addresses (may resolve to multiple IPs)
    /// - Setting upstream connection details (address and SNI) in the context for next phase (upstream_peer)
    /// - Returning an empty response on success or a serialized error response on failure
    ///
    /// # Arguments
    /// * `ctx` - The request context for storing upstream address and SNI information
    ///
    /// # Returns
    /// * `Vec<u8>` - Empty vector on successful validation, or serialized error response if validation fails
    ///
    /// # Error Cases
    /// * `"backend_url is a required param"` - If the `backend_url` query parameter is missing
    /// * `"Invalid backend_url"` - If the URL is invalid or cannot be resolved to socket addresses
    ///
    /// # Flow
    /// 1. Extract `backend_url` query parameter
    /// 2. Validate URL format and resolve to socket addresses
    /// 3. Store resolved addresses and SNI in context for upstream_peer phase
    /// 4. Return empty Vec on success, or error response bytes on failure
    fn request_filter_init_tunnel(&self, ctx: &mut Layer8Context) -> Vec<u8> {
        if let Some(url) = ctx.param("backend_url") {
            if let Some(url) = utils::validate_url(url) {
                let socket_addr = utils::get_socket_addrs(&url);
                ctx.set(CtxKeys::UPSTREAM_ADDRESS.to_string(), socket_addr);
                ctx.set(
                    CtxKeys::UPSTREAM_SNI.to_string(),
                    url.domain().unwrap_or_default().to_string(),
                );
                vec![]
            } else {
                ErrorResponse {
                    error: "Invalid backend_url".to_string(),
                }
                .to_bytes()
            }
        } else {
            ErrorResponse {
                error: "backend_url is a required param".to_string(),
            }
            .to_bytes()
        }
    }

    /// Validates and processes proxy requests.
    /// This function is responsible for:
    /// - Extracting and validating the `int_fp_jwt` header from the request
    /// - Verifying the JWT token to obtain session information
    /// - Extracting the backend URL (rp_base_url) from the verified session
    /// - Resolving the backend URL to socket addresses
    /// - Setting upstream connection details (address and SNI) in the context for next phase (upstream_peer)
    /// - Storing client authentication information for later logging and tracking
    /// - Returning an empty response on success or a serialized error response on failure
    ///
    /// # Arguments
    /// * `ctx` - The request context for storing upstream address, SNI, and client authentication information
    ///
    /// # Returns
    /// * `Vec<u8>` - Empty vector on successful validation, or serialized error response if validation fails
    ///
    /// # Error Cases
    /// * `"Missing int_fp_jwt header"` - If the `int_fp_jwt` header is missing from the request
    /// * `"Invalid backend_url"` - If the URL from the session is invalid or cannot be resolved to socket addresses
    /// * Custom JWT verification errors - If the JWT token is invalid, expired, or verification fails
    ///
    /// # Flow
    /// 1. Extract int_fp_jwt header from request
    /// 2. Verify JWT token and extract session data (includes rp_base_url and client_id)
    /// 3. Store client_id in context for statistics tracking
    /// 4. Validate and resolve rp_base_url to socket addresses
    /// 5. Store addresses and SNI in context for upstream_peer phase
    /// 6. Return empty Vec on success, or error response bytes on failure
    fn request_filter_proxy(&self, ctx: &mut Layer8Context) -> Vec<u8> {
        let correlation_id = ctx.get_correlation_id();
        // For proxy request, we expect the int-fp-jwt token in the header, and we will use it to
        // get the upstream address for the next phase (upstream_peer)
        match ctx.get_request_header().get(HeaderKeys::INT_FP_JWT) {
            None => ErrorResponse {
                error: "Missing int_fp_jwt header".to_string(),
            }
            .to_bytes(),
            Some(int_fp_jwt) => match self.handler.verify_int_fp_jwt(int_fp_jwt.as_str()) {
                Ok(session) => {
                    debug!(%correlation_id, "IntFPSession: {:?}", session);
                    ctx.set(
                        CtxKeys::BACKEND_AUTH_CLIENT_ID.to_string(),
                        session.client_id,
                    );

                    // `rp_base_url` should be validated before being saved in the session,
                    // but we call it again to get the parsed URL object for extracting socket
                    // addresses and SNI because the function is relatively simple and lightweight.
                    // If the URL is invalid, we handle error anyway.
                    if let Some(url) = utils::validate_url(&session.rp_base_url) {
                        let socket_addr = utils::get_socket_addrs(&url);
                        ctx.set(CtxKeys::UPSTREAM_ADDRESS.to_string(), socket_addr);
                        ctx.set(
                            CtxKeys::UPSTREAM_SNI.to_string(),
                            url.domain().unwrap_or_default().to_string(),
                        );
                        vec![]
                    } else {
                        ErrorResponse {
                            error: "Invalid backend_url".to_string(),
                        }
                        .to_bytes()
                    }
                }
                Err(err) => {
                    error!(
                        %correlation_id,
                        log_type = LogTypes::HANDLE_CLIENT_REQUEST,
                        "Error verifying int_fp_jwt: {}", err
                    );
                    ErrorResponse { error: err }.to_bytes()
                }
            },
        }
    }
}

/// Implementation of the Pingora ProxyHttp trait for the ForwardProxy.
///
/// This trait implementation defines the complete HTTP request/response processing pipeline,
/// including request filtering, upstream connection management, header modifications, response
/// filtering, logging, and error handling.
///
/// # Request Processing Pipeline
/// The following methods are executed in order during request processing:
/// 1. `request_filter()` - Initial request validation and routing
/// 2. `request_body_filter()` - Request body processing (chunked)
/// 3. `upstream_peer()` - Upstream connection establishment with mTLS
/// 4. `upstream_request_filter()` - Upstream request header modification
/// 5. `response_filter()` - Upstream response header modification
/// 6. `response_body_filter()` - Response body processing (chunked)
/// 7. `logging()` - Request/response logging and statistics
///
/// # Error Handling
/// - `fail_to_connect()` - Handles upstream connection failures with retry logic
///
/// # Security Features
/// - mTLS (mutual TLS) authentication with upstream servers
/// - JWT token verification for proxy requests
/// - CORS header validation and enforcement
/// - Certificate pinning and hostname verification
///
/// # See Also
/// - [Pingora Phases Documentation](https://github.com/cloudflare/pingora/blob/main/docs/user_guide/phase.md)
#[async_trait]
impl ProxyHttp for ForwardProxy {
    type CTX = Layer8Context;

    fn new_ctx(&self) -> Self::CTX {
        Layer8Context::default()
    }

    /// Manages upstream connection with mTLS.
    ///
    /// This performs step 4 of mTLS handshake (client certificate presentation).
    /// The peer verification and certificate validation are handled by TLSCredentials.
    ///
    /// # Security & TLS Handshake
    /// The mTLS process is a 7-step mutual authentication mechanism:
    /// 1. Client connects to server
    /// 2. Server presents its TLS certificate
    /// 3. Client verifies the server's certificate
    /// 4. Client presents its TLS certificate (handled in this method)
    /// 5. Server verifies the client's certificate
    /// 6. Server grants access
    /// 7. Client and server exchange information over encrypted TLS connection
    ///
    /// # Failover & Retry Logic
    /// When a DNS name resolves to multiple socket addresses:
    /// - Attempts to create HttpPeer for each address in sequence
    /// - On panic during peer creation, removes that address and tries the next one
    /// - Returns ConnectError if all addresses fail
    /// - Failed addresses are logged and removed from the address list
    ///
    /// # See Also
    /// - [Pingora Phases](https://github.com/cloudflare/pingora/blob/main/docs/user_guide/phase.md)
    ///
    /// # Errors
    /// Returns `ConnectError` if no valid peer could be created for any address
    async fn upstream_peer(
        &self,
        _session: &mut Session,
        ctx: &mut Self::CTX,
    ) -> pingora::Result<Box<HttpPeer>> {
        let correlation_id = ctx.get_correlation_id();

        let addrs = ctx
            .get(CtxKeys::UPSTREAM_ADDRESS)
            .unwrap_or(&"".to_string())
            .clone();
        let sni = ctx
            .get(CtxKeys::UPSTREAM_SNI)
            .unwrap_or(&"".to_string())
            .clone();
        info!(
            %correlation_id,
            log_type = LogTypes::UPSTREAM_CONNECT,
            addresses = addrs,
            sni = sni
        );

        // HttpPeer cannot connect to upstream without a valid socket(IP:PORT) address.
        // A dns name can resolve to multiple socket addresses.
        // We will try to connect to each address until one succeeds.
        let mut address_list: Vec<&str> = addrs.split(',').collect();
        let upstream_sni = sni.to_string(); // clone for move into closure
        let mut opt_peer = None;
        for addr in address_list.clone() {
            match std::panic::catch_unwind(|| {
                HttpPeer::new(addr, self.config.tls.enable_tls, upstream_sni.clone())
            }) {
                Ok(p) => {
                    info!(
                        %correlation_id,
                        log_type = LogTypes::UPSTREAM_CONNECT,
                        "Created HttpPeer for addr: {}", addr
                    );
                    opt_peer = Some(p);
                    break;
                }
                Err(err) => {
                    error!(
                        %correlation_id,
                        log_type = LogTypes::UPSTREAM_CONNECT,
                        "Panic occurred while creating HttpPeer for addr: {}, error: {:?}",
                        addr,
                        err
                    );
                    address_list.retain(|&x| x != addr);
                    ctx.set(
                        CtxKeys::UPSTREAM_ADDRESS.to_string(),
                        address_list.join(","),
                    );
                }
            }
        }

        let mut peer = match opt_peer {
            Some(p) => p,
            None => {
                error!(
                    %correlation_id,
                    log_type = LogTypes::UPSTREAM_CONNECT,
                    "Failed to create HttpPeer for any socket address"
                );
                return Err(Error::new(ErrorType::ConnectError));
            }
        };

        if self.config.tls.enable_tls {
            // Configure mTLS peer options
            let mut peer_options = PeerOptions::new();
            {
                // Step 3 of mTLS: Verify the server's certificate against CA
                peer_options.verify_cert = true;
                peer_options.ca = Some(Arc::new(Box::new([self.tls_credentials.ca_cert.clone()])));
                // Verify that upstream server's certificate hostname matches the SNI
                peer_options.verify_hostname = true;
            }

            // Step 4 of mTLS: Present client certificate and key to upstream server
            peer.client_cert_key = Some(self.tls_credentials.cert_key.load_full());
            peer.options = peer_options;
        }

        Ok(Box::new(peer))
    }

    /// Filters and processes incoming client requests.
    /// This function is responsible for:
    /// - Updating the context with session information
    /// - Handling CORS preflight requests (OPTIONS method)
    /// - Routing requests to appropriate handlers based on path and method
    /// - Processing healthcheck requests (GET /healthcheck)
    /// - Processing init-tunnel requests (POST /init-tunnel) with backend URL validation
    /// - Processing proxy requests (POST /proxy) with JWT token verification
    /// - Returning error responses for invalid requests
    /// - Returning 404 for unmatched routes
    ///
    /// # Request Routing
    /// * `OPTIONS /any-path` → CORS preflight handling
    /// * `GET /healthcheck` → Health check response
    /// * `POST /init-tunnel` → Backend URL validation
    /// * `POST /proxy` → JWT verification
    /// * Anything else → 404 Not Found
    ///
    /// # Arguments
    /// * `session` - The current HTTP session
    /// * `ctx` - The request context for storing and retrieving request data
    ///
    /// # Returns
    /// * `Ok(true)` - If the request was fully handled (response already sent)
    /// * `Ok(false)` - If the request should continue to upstream processing
    /// * `Err(Error)` - If an error occurred during processing
    ///
    /// # Error Handling
    /// If request validation fails, returns HTTP 400 with error message JSON
    async fn request_filter(
        &self,
        session: &mut Session,
        ctx: &mut Self::CTX,
    ) -> pingora::Result<bool>
    where
        Self::CTX: Send + Sync,
    {
        // initialize context with request information for later use in the processing pipeline
        ctx.update(session).await?;

        if session.req_header().method == pingora::http::Method::OPTIONS {
            return self.handle_preflight_request(ctx, session).await;
        }

        let error_response_bytes = match (
            session.req_header().uri.path(),
            session.req_header().method.as_str(),
        ) {
            (RequestPaths::HEALTHCHECK, "GET") => {
                return self.handle_healthcheck(ctx, session).await;
            }
            (RequestPaths::INIT_TUNNEL, "POST") => self.request_filter_init_tunnel(ctx),
            (RequestPaths::PROXY, "POST") => self.request_filter_proxy(ctx),
            _ => {
                ctx.response.status = StatusCode::NOT_FOUND;
                let header = ResponseHeader::build(StatusCode::NOT_FOUND, None)?;
                session.write_response_header_ref(&header, false).await?;
                session.set_keepalive(None);
                return Ok(true);
            }
        };

        if error_response_bytes.len() > 0 {
            ctx.response.status = StatusCode::BAD_REQUEST;
            ctx.set_response_body(error_response_bytes.clone());
            let header = ResponseHeader::build(StatusCode::BAD_REQUEST, None)?;
            session.write_response_header_ref(&header, false).await?;
            session
                .write_response_body(Some(Bytes::from(error_response_bytes)), true)
                .await?;
            session.set_keepalive(None);
            return Ok(true);
        }

        Ok(false)
    }

    /// Processes the request body in chunks and handles init-tunnel request processing.
    /// This function is responsible for:
    /// - Accumulating the request body in chunks until end_of_stream is received
    /// - For `init-tunnel` requests: delegating to the handler for processing and transformation
    /// - For other requests: passing through the request body unchanged
    /// - Clearing individual chunks after storing them to free memory
    ///
    /// # Arguments
    /// * `session` - The current HTTP session
    /// * `body` - The current chunk of the request body (mutable)
    /// * `end_of_stream` - Boolean indicating if this is the final chunk
    /// * `ctx` - The request context for storing and retrieving body data
    ///
    /// # Returns
    /// * `Ok(())` - Successfully processed the request body chunk
    /// * `Err(Error)` - If an error occurred during init-tunnel handling
    async fn request_body_filter(
        &self,
        session: &mut Session,
        body: &mut Option<Bytes>,
        end_of_stream: bool,
        ctx: &mut Self::CTX,
    ) -> pingora::Result<()>
    where
        Self::CTX: Send + Sync,
    {
        // Read the request body in chunks and store it in the context until we receive the end_of_stream signal.
        // Once we receive end_of_stream, we know we have the full request body and can process it accordingly.
        if let Some(b) = body {
            ctx.extend_request_body(b.to_vec());
            // drop the body
            b.clear();
        }

        if end_of_stream {
            let correlation_id = ctx.get_correlation_id();

            if session.req_header().uri.path() != RequestPaths::INIT_TUNNEL {
                info!(
                    %correlation_id,
                    log_type = LogTypes::HANDLE_CLIENT_REQUEST,
                    request_summary = session.request_summary(),
                    "Forward proxy passing through request body unchanged."
                );
                *body = Some(Bytes::copy_from_slice(ctx.get_request_body().as_slice()));
                return Ok(());
            }

            let handler_response = self.handler.handle_init_tunnel_request(ctx).await;
            if handler_response.status != StatusCode::OK {
                error!(
                    %correlation_id,
                    log_type = LogTypes::HANDLE_CLIENT_REQUEST,
                    request_summary = session.request_summary(),
                    "Failed to handle init-tunnel request with status: {}, error: {}",
                    handler_response.status,
                    utils::bytes_to_string(&handler_response.body.unwrap_or_default())
                );
                return Err(pingora::Error::new(pingora::ErrorType::HTTPStatus(
                    u16::from(handler_response.status),
                )));
            }

            info!(
                %correlation_id,
                log_type = LogTypes::HANDLE_CLIENT_REQUEST,
                request_summary = session.request_summary(),
                "Handle init-tunnel Request response with status: {}",
                handler_response.status,
            );

            let fp_req_body = handler_response.body.as_ref().unwrap_or(&vec![]).clone();

            *body = Some(Bytes::copy_from_slice(fp_req_body.as_slice()));
        }

        Ok(())
    }

    /// Modifies upstream request headers based on the request path and token information.
    /// This function is responsible for:
    /// - For proxy requests: extracting the int_fp_jwt token, verifying it, and replacing it with fp_rp_jwt header
    /// - Removing the int_fp_jwt header from upstream requests (internal token)
    /// - Setting Transfer-Encoding to chunked for requests with a body (for streaming support)
    /// - Adding correlation ID header to all upstream requests (for request tracing)
    ///
    /// # JWT Token Flow
    /// The forward proxy uses two different JWT tokens:
    /// - `int_fp_jwt`: Internal token received from client (contains session and reverse proxy info)
    /// - `fp_rp_jwt`: Token sent to reverse proxy (derived from int_fp_jwt session)
    ///
    /// # Header Transformations
    /// 1. Extract `int_fp_jwt` from request headers (if proxy request)
    /// 2. Verify JWT and retrieve session containing `fp_rp_jwt`
    /// 3. Insert `fp_rp_jwt` into upstream request headers
    /// 4. Remove `int_fp_jwt` from upstream headers (internal only)
    /// 5. Set `Transfer-Encoding: chunked` if body is present
    /// 6. Add `x-correlation-id` for distributed tracing
    ///
    /// # Arguments
    /// * `session` - The current HTTP session
    /// * `upstream_request` - The upstream request header to be modified
    /// * `ctx` - The request context for retrieving client information and tokens
    ///
    /// # Returns
    /// * `Ok(())` - Successfully processed the upstream request headers
    /// * `Err(Error)` - If JWT token verification fails or header manipulation fails
    async fn upstream_request_filter(
        &self,
        session: &mut Session,
        upstream_request: &mut RequestHeader,
        ctx: &mut Self::CTX,
    ) -> pingora::Result<()>
    where
        Self::CTX: Send + Sync,
    {
        let correlation_id = ctx.get_correlation_id();

        if session.req_header().uri.path() == RequestPaths::PROXY {
            // get int_fp_jwt token for upstream request header manipulation,
            // cannot be done earlier in request_filter because it doesn't have access to upstream request header
            let mut pingora_err = None;
            if let Some(token) = upstream_request.headers.get(HeaderKeys::INT_FP_JWT) {
                let token_str = token.to_str().or_err(
                    pingora::ErrorType::InvalidHTTPHeader,
                    "This value should be checked in the previous step, it should never fail here",
                )?;

                match self.handler.get_session(token_str) {
                    Ok(session) => {
                        upstream_request
                            .insert_header(HeaderKeys::FP_RP_JWT, session.fp_rp_jwt)
                            .unwrap_or_default();
                        upstream_request.remove_header(HeaderKeys::INT_FP_JWT);
                    }
                    Err(err) => {
                        pingora_err = Some(pingora::Error::explain(
                            pingora::ErrorType::InvalidHTTPHeader,
                            err,
                        ));
                    }
                }
            } else {
                pingora_err = Some(pingora::Error::new(pingora::ErrorType::HTTPStatus(
                    u16::from(StatusCode::INTERNAL_SERVER_ERROR),
                )));
            }

            if let Some(err) = pingora_err {
                error!(
                    %correlation_id,
                    log_type = LogTypes::HANDLE_CLIENT_REQUEST,
                    request_summary = session.request_summary(),
                    "Failed to get session",
                );

                return Err(err);
            }
        }

        if upstream_request.headers.get("x-empty-body").is_none() {
            upstream_request.remove_header("content-length");
            upstream_request
                .insert_header(TRANSFER_ENCODING.as_str(), "chunked")
                .unwrap_or_default();
        }

        upstream_request
            .insert_header("x-correlation-id", correlation_id)
            .unwrap_or_default();

        Ok(())
    }

    /// Modifies upstream response headers based on the request context.
    /// This function is responsible for:
    /// - Setting CORS response headers (origin, credentials, methods, max age)
    /// - Removing content-length header and setting Transfer-Encoding to chunked for non-empty responses
    ///
    /// # Arguments
    /// * `session` - The current HTTP session (unused in this implementation)
    /// * `upstream_response` - The upstream response header to be modified
    /// * `ctx` - The request context for retrieving CORS configuration
    ///
    /// # Returns
    /// * `Ok(())` - Successfully processed the upstream response headers
    /// * `Err(Error)` - If header manipulation fails
    async fn response_filter(
        &self,
        _session: &mut Session,
        upstream_response: &mut ResponseHeader,
        ctx: &mut Self::CTX,
    ) -> pingora::Result<()> {
        self.set_response_header(ctx, upstream_response)?;
        // if let Some(req_headers) = session
        //     .req_header()
        //     .headers
        //     .get("Access-Control-Request-Headers")
        // {
        //     upstream_response.insert_header("Access-Control-Allow-Headers", req_headers)?;
        // }

        if let Some(length) = upstream_response.headers.get("content-length") {
            if length != "0" {
                upstream_response.remove_header("content-length");
                upstream_response.insert_header(TRANSFER_ENCODING.as_str(), "chunked")?;
            }
        }

        Ok(())
    }

    /// Processes the upstream response body in chunks and handles init-tunnel response transformation.
    /// This function is responsible for:
    /// - Accumulating the response body in chunks until end_of_stream is received
    /// - For init-tunnel requests: delegating to the handler for processing and transformation
    /// - For other requests: passing through the response body unchanged
    /// - Clearing individual chunks after storing them to free memory
    ///
    /// # Arguments
    /// * `session` - The current HTTP session
    /// * `body` - The current chunk of the response body (mutable)
    /// * `end_of_stream` - Boolean indicating if this is the final chunk
    /// * `ctx` - The request context for storing and retrieving response body data
    ///
    /// # Returns
    /// * `Ok(None)` - Successfully processed the response body chunk
    /// * `Err(Error)` - If an error occurred during init-tunnel response handling
    fn response_body_filter(
        &self,
        session: &mut Session,
        body: &mut Option<Bytes>,
        end_of_stream: bool,
        ctx: &mut Self::CTX,
    ) -> pingora::Result<Option<Duration>>
    where
        Self::CTX: Send + Sync,
    {
        // Read the response body in chunks and store it in the context until we receive the end_of_stream signal.
        // Once we receive end_of_stream, we know we have the full response body and can process it accordingly.
        if let Some(b) = body {
            ctx.extend_response_body(b.to_vec());
            // drop the body
            b.clear();
        }

        if end_of_stream {
            let correlation_id = ctx.get_correlation_id();

            if session.req_header().uri.path() != RequestPaths::INIT_TUNNEL {
                info!(
                    %correlation_id,
                    log_type = LogTypes::HANDLE_UPSTREAM_RESPONSE,
                    request_summary = session.request_summary(),
                    "Forward proxy passing through response body unchanged."
                );
                *body = Some(Bytes::copy_from_slice(ctx.get_response_body().as_slice()));
                return Ok(None);
            }

            let handler_response = self.handler.handle_init_tunnel_response(ctx);
            if handler_response.status != StatusCode::OK {
                error!(
                    %correlation_id,
                    log_type = LogTypes::HANDLE_UPSTREAM_RESPONSE,
                    request_summary = session.request_summary(),
                    "Failed to handle init-tunnel Response response with status: {}, error: {}",
                    handler_response.status,
                    utils::bytes_to_string(&handler_response.body.unwrap_or_default())
                );

                ctx.response.status = StatusCode::INTERNAL_SERVER_ERROR;
                return Err(pingora::Error::new(pingora::ErrorType::HTTPStatus(
                    u16::from(StatusCode::INTERNAL_SERVER_ERROR),
                )));
            }

            info!(
                %correlation_id,
                log_type = LogTypes::HANDLE_UPSTREAM_RESPONSE,
                request_summary = session.request_summary(),
                "Handle init-tunnel Response response with status: {}",
                handler_response.status,
            );

            let fp_res_body = handler_response.body.as_ref().unwrap_or(&vec![]).clone();

            ctx.response.status = handler_response.status;
            ctx.set_response_body(fp_res_body.clone());
            *body = Some(Bytes::copy_from_slice(fp_res_body.as_slice()));
        }

        Ok(None)
    }

    /// Logs request and response information along with updating client usage statistics.
    /// This function is responsible for:
    /// - Extracting the response status code from the context or session
    /// - Recording client usage statistics for POST requests to proxy and init-tunnel endpoints
    /// - Spawning an async task to update statistics without blocking the response
    /// - Logging comprehensive access information including status, headers, latency, and errors
    ///
    /// # Arguments
    /// * `session` - The current HTTP session containing request and response information
    /// * `e` - Optional error that occurred during request processing
    /// * `ctx` - The request context for retrieving correlation ID, body sizes, and latency
    ///
    /// # Returns
    /// * No return value; logs are written asynchronously
    ///
    /// # Logged Information
    /// * Correlation ID - Unique request identifier
    /// * Status code - HTTP response status
    /// * Request summary - Method, path, and version
    /// * Request headers - Origin, referer, and user-agent
    /// * Performance metrics - Latency in milliseconds and response body size
    /// * Error details - Any errors that occurred during processing
    async fn logging(&self, session: &mut Session, e: Option<&Error>, ctx: &mut Self::CTX)
    where
        Self::CTX: Send + Sync,
    {
        let correlation_id = ctx.get_correlation_id();

        let mut status = ctx.response.status.as_u16();
        if let Some(_err) = e {
            status = session.response_written().unwrap().status.as_u16();
        }

        // Update client usage statistics
        if session.req_header().method.as_str() == "POST"
            && (session.req_header().uri.path() == RequestPaths::PROXY
                || session.req_header().uri.path() == RequestPaths::INIT_TUNNEL)
        {
            let client_id = ctx
                .get(CtxKeys::BACKEND_AUTH_CLIENT_ID)
                .unwrap_or(&"".to_string())
                .clone();
            let request_path = session.req_header().uri.path().to_string();
            let total_byte_transferred =
                (ctx.get_request_body().len() + ctx.get_response_body().len()) as i64;
            let correlation_id = correlation_id.clone();

            tokio::spawn(async move {
                Statistics::update(
                    client_id,
                    correlation_id,
                    request_path,
                    total_byte_transferred,
                    status,
                )
                .await;
            });
        }

        info!(
            %correlation_id,
            log_type=LogTypes::ACCESS_LOG,
            status=status,
            request_summary=session.request_summary(),
            origin = ctx.request.header.get("origin"),
            referer = ctx.request.header.get("referer"),
            user_agent = ctx.request.header.get("User-Agent"),
            latency_micro=ctx.get_latency().as_micros(),
            response_body_size=ctx.get_response_body().len(),
            error=?e,
        );
    }

    /// Logs connection failures to upstream servers and manages retry logic.
    /// This function is responsible for:
    /// - Detecting connection failures (timeout, refused, or generic connection errors)
    /// - Removing failed socket addresses from the upstream address list
    /// - Setting retry flag to attempt connection with the next available address
    /// - Logging detailed error information including the failed address and retry status
    ///
    /// # Arguments
    /// * `_session` - The current HTTP session (unused in this implementation)
    /// * `peer` - The HttpPeer that failed to connect
    /// * `ctx` - The request context for retrieving and updating upstream addresses
    /// * `e` - The error that occurred during the connection attempt
    ///
    /// # Returns
    /// * `Box<Error>` - The original error with retry flag set if another address is available
    ///
    /// # See Also:
    /// - [Pingora Failover Handling](https://github.com/cloudflare/pingora/blob/main/docs/user_guide/failover.md)
    ///
    /// # Retry Logic
    /// When a connection error occurs:
    /// 1. Checks if the error is a timeout, refused, or generic connection error
    /// 2. Extracts the next address from the comma-separated address list
    /// 3. Sets retry=true to trigger upstream_peer() to be called again with the next address
    /// 4. Logs the failure with peer address, error details, and retry status
    fn fail_to_connect(
        &self,
        _session: &mut Session,
        peer: &HttpPeer,
        ctx: &mut Self::CTX,
        mut e: Box<Error>,
    ) -> Box<Error> {
        let mut retry = false;
        if e.etype == ErrorType::ConnectTimedout
            || e.etype == ErrorType::ConnectError
            || e.etype == ErrorType::ConnectRefused
        {
            let mut addrs = ctx
                .get(CtxKeys::UPSTREAM_ADDRESS)
                .unwrap_or(&"".to_string())
                .clone();

            // remove failed socket address from the list
            let idx = addrs.find(",");
            if let Some(idx) = idx {
                // set retry=true to recall Self::upstream_peer to try next address
                retry = true;
                addrs = addrs[idx + 1..].to_string();

                ctx.set(CtxKeys::UPSTREAM_ADDRESS.to_string(), addrs);
            }

            error!(
                correlation_id = ctx.get_correlation_id(),
                log_type = LogTypes::UPSTREAM_CONNECT,
                "Failed to connect to upstream addr: {}, err: {}, retry: {}",
                peer._address.to_string(),
                e,
                retry
            );
        }
        e.set_retry(retry);
        e
    }
}
