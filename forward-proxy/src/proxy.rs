use crate::config::TlsConfig;
use crate::handler::consts::{HeaderKeys, LogTypes, CtxKeys};
use crate::handler::types::response::ErrorResponse;
use crate::handler::ForwardHandler;
use crate::statistics::Statistics;
use async_trait::async_trait;
use boring::x509::X509;
use bytes::Bytes;
use pingora::Error;
use pingora::OrErr;
use pingora::http::{RequestHeader, ResponseHeader, StatusCode};
use pingora::listeners::tls::TLS_CONF_ERR;
use pingora::prelude::{HttpPeer, ProxyHttp, Session};
use pingora::upstreams::peer::PeerOptions;
use pingora::utils::tls::CertKey;
use pingora_error::ErrorType;
use pingora_router::ctx::{Layer8Context, Layer8ContextTrait};
use pingora_router::handler::ResponseBodyTrait;
use reqwest::header::TRANSFER_ENCODING;
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, error, info};

pub struct ForwardProxy {
    tls_config: TlsConfig,
    handler: ForwardHandler,
}

impl ForwardProxy {
    pub fn new(tls_config: TlsConfig, handler: ForwardHandler) -> Self {
        ForwardProxy {
            tls_config,
            handler,
        }
    }
}

/// To see the order of execution and how the request is processed, refer to the documentation
/// see https://github.com/cloudflare/pingora/blob/main/docs/user_guide/phase.md
#[async_trait]
impl ProxyHttp for ForwardProxy {
    type CTX = Layer8Context;

    fn new_ctx(&self) -> Self::CTX {
        Layer8Context::default()
    }

    async fn upstream_peer(
        &self,
        _session: &mut Session,
        ctx: &mut Self::CTX,
    ) -> pingora::Result<Box<HttpPeer>> {
        // testing certs data; fixme to be dynamic

        // mTLS Steps:
        // 1. Client connects to server
        // 2. Server presents its TLS certificate
        // 3. Client verifies the server's certificate
        // 4. Client presents its TLS certificate
        // 5. Server verifies the client's certificate
        // 6. Server grants access
        // 7. Client and server exchange information over encrypted TLS connection
        //
        // Code below is for step 4(this is a client to RP), presenting the client's TLS certificate.

        let addrs = ctx
            .get(&CtxKeys::UPSTREAM_ADDRESS.to_string())
            .unwrap_or(&"".to_string())
            .clone();
        let sni = ctx
            .get(&CtxKeys::UPSTREAM_SNI.to_string())
            .unwrap_or(&"".to_string())
            .clone();
        info!(
            log_type = LogTypes::UPSTREAM_CONNECT,
            addresses = addrs,
            sni = sni
        );

        // HttpPeer cannot connect to upstream without a valid socket(IP:PORT) address.
        // A dns name can resolve to multiple socket addresses.
        // We will try to connect to each address until one succeeds.
        let mut address_list: Vec<&str> = addrs.split(',').collect();
        let enable_tls = self.tls_config.enable_tls; // clone for move into closure
        let upstream_sni = sni.to_string(); // clone for move into closure
        let mut opt_peer = None;
        for addr in address_list.clone() {
            match std::panic::catch_unwind(|| HttpPeer::new(addr, enable_tls, upstream_sni.clone()))
            {
                Ok(p) => {
                    info!(
                        log_type = LogTypes::UPSTREAM_CONNECT,
                        "Created HttpPeer for addr: {}", addr
                    );
                    opt_peer = Some(p);
                    break;
                }
                Err(err) => {
                    error!(
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
                    log_type = LogTypes::UPSTREAM_CONNECT,
                    "Failed to create HttpPeer for any socket address"
                );
                return Err(Error::new(ErrorType::ConnectError));
            }
        };

        if self.tls_config.enable_tls {
            let cert = X509::from_pem(&self.tls_config.cert.clone().into_bytes())
                .or_err(TLS_CONF_ERR, "Failed to load FP's certificate")?;

            let ca_cert = X509::from_pem(&self.tls_config.ca_cert.clone().into_bytes())
                .or_err(TLS_CONF_ERR, "Failed to load CA certificate")?;

            let key =
                boring::pkey::PKey::private_key_from_pem(&self.tls_config.key.clone().into_bytes())
                    .or_err(TLS_CONF_ERR, "Failed to load private key")?;

            // The certificate to present in mTLS connections to upstream
            // The organization implementing mTLS acts as its own certificate authority.
            let cert_key = CertKey::new(vec![cert], key);

            // Providing Peer Options
            let mut peer_options = PeerOptions::new();
            {
                peer_options.verify_cert = true; // Verify the server's certificate
                peer_options.ca = Some(Arc::new(Box::new([ca_cert])));
                peer_options.verify_hostname = true; // Whether to check if upstream server cert's Host matches the SNI
            }

            peer.client_cert_key = Some(Arc::new(cert_key));
            peer.options = peer_options;
        }

        Ok(Box::new(peer))
    }

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
        let request_id = session
            .req_header()
            .headers
            .get("x-request-id")
            .map(|v| v.to_str().unwrap_or(""))
            .unwrap_or("");
        info!(
            log_type = LogTypes::ACCESS_LOG,
            client_ip = ctx.request.summary.host,
            request_summary = session.request_summary(),
            // status=resp.status,
            // duration_ms=duration_ms,
            // bytes_out=resp.body.len(),
            user_agent = ctx.request.header.get("User-Agent"),
            request_id = request_id,
        );

        match session.req_header().method {
            pingora::http::Method::OPTIONS => {
                // Handle CORS preflight request
                ctx.response.status = StatusCode::NO_CONTENT;
                let mut header = ResponseHeader::build(StatusCode::NO_CONTENT, None)?;
                header.insert_header("Access-Control-Allow-Origin", "*")?;
                header.insert_header("Access-Control-Allow-Methods", "POST")?;
                header.insert_header("Access-Control-Allow-Headers", "*")?;
                session.write_response_header_ref(&header).await?;
                session.set_keepalive(None);
                return Ok(true);
            }
            _ => {}
        }

        let mut error_response_bytes: Vec<u8> = vec![];
        match (
            session.req_header().uri.path(),
            session.req_header().method.as_str(),
        ) {
            ("/healthcheck", "GET") => {
                let handler_response = self.handler.handle_healthcheck(ctx);
                let mut header = ResponseHeader::build(handler_response.status, None)?;
                let response_headers = header.headers.clone();
                for (key, val) in response_headers.iter() {
                    header
                        .insert_header(key.clone(), val.clone())
                        .map_err(|e| {
                            error!(
                                log_type = LogTypes::HEALTHCHECK,
                                "Cannot add request header {}:{:?}, err: {:?}",
                                key.clone(),
                                val.clone(),
                                e
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

                session.write_response_header_ref(&header).await?;

                debug!(
                    log_type = LogTypes::HEALTHCHECK,
                    request_summary = session.request_summary(),
                    response_body = utils::bytes_to_string(&response_bytes)
                );

                // Write the response body to the session after setting headers
                session
                    .write_response_body(Some(Bytes::from(response_bytes)), true)
                    .await?;

                return Ok(true);
            }
            ("/init-tunnel", "POST") => {
                if let Some(url) = ctx.param("backend_url") {
                    if let Some(url) = utils::validate_url(url) {
                        let socket_addr = utils::get_socket_addrs(&url);
                        ctx.set(CtxKeys::UPSTREAM_ADDRESS.to_string(), socket_addr);
                        ctx.set(
                            CtxKeys::UPSTREAM_SNI.to_string(),
                            url.domain().unwrap_or_default().to_string(),
                        );
                    } else {
                        error_response_bytes = ErrorResponse {
                            error: "Invalid backend_url".to_string(),
                        }
                        .to_bytes();
                    }
                } else {
                    error_response_bytes = ErrorResponse {
                        error: "backend_url is a required param".to_string(),
                    }
                    .to_bytes();
                }
            }
            ("/proxy", "POST") => {
                error_response_bytes = match ctx
                    .get_request_header()
                    .get(HeaderKeys::INT_FP_JWT)
                {
                    None => ErrorResponse {
                        error: "Missing int_fp_jwt header".to_string(),
                    }
                    .to_bytes(),
                    Some(int_fp_jwt) => match self.handler.verify_int_fp_jwt(int_fp_jwt.as_str()) {
                        Ok(session) => {
                            debug!("IntFPSession: {:?}", session);
                            ctx.set(CtxKeys::FP_RP_JWT.to_string(), session.fp_rp_jwt);
                            ctx.set(
                                CtxKeys::BACKEND_AUTH_CLIENT_ID.to_string(),
                                session.client_id,
                            );

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
                                log_type = LogTypes::HANDLE_CLIENT_REQUEST,
                                "Error verifying int_fp_jwt: {}", err
                            );
                            ErrorResponse { error: err }.to_bytes()
                        }
                    },
                }
            }
            _ => {
                ctx.response.status = StatusCode::NOT_FOUND;
                let header = ResponseHeader::build(StatusCode::NOT_FOUND, None)?;
                session.write_response_header_ref(&header).await?;
                session.set_keepalive(None);
                return Ok(true);
            }
        }

        if error_response_bytes.len() > 0 {
            ctx.response.status = StatusCode::BAD_REQUEST;
            ctx.set_response_body(error_response_bytes.clone());
            let header = ResponseHeader::build(StatusCode::BAD_REQUEST, None)?;
            session.write_response_header_ref(&header).await?;
            session
                .write_response_body(Some(Bytes::from(error_response_bytes)), true)
                .await?;
            session.set_keepalive(None);
            return Ok(true);
        }

        Ok(false)
    }

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
        if let Some(b) = body {
            ctx.extend_request_body(b.to_vec());
            // drop the body
            b.clear();
        }

        if end_of_stream {
            debug!(
                request_summary = session.request_summary(),
                request_body = utils::bytes_to_string(&ctx.get_request_body()),
            );
            info!(
                log_type = LogTypes::HANDLE_CLIENT_REQUEST,
                request_summary = session.request_summary(),
                "Request Body Received: {} bytes.",
                &ctx.get_request_body().len()
            );

            // This is the last chunk, we can process the data now

            let handler_response = match session.req_header().uri.path() {
                "/init-tunnel" => self.handler.handle_init_tunnel_request(ctx).await,
                _ => {
                    info!(
                        log_type = LogTypes::HANDLE_CLIENT_REQUEST,
                        request_summary = session.request_summary(),
                        "Forward proxy passing through request body unchanged."
                    );
                    *body = Some(Bytes::copy_from_slice(ctx.get_request_body().as_slice()));
                    return Ok(());
                }
            };

            if handler_response.status != StatusCode::OK {
                error!(
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
                log_type = LogTypes::HANDLE_CLIENT_REQUEST,
                request_summary = session.request_summary(),
                "Handle init-tunnel Request response with status: {}",
                handler_response.status,
            );
            debug!(
                request_summary = session.request_summary(),
                "Handle init-tunnel response body: {}",
                utils::bytes_to_string(&handler_response.body.as_ref().unwrap_or(&vec![]))
            );
            let fp_req_body = handler_response.body.as_ref().unwrap_or(&vec![]).clone();

            *body = Some(Bytes::copy_from_slice(fp_req_body.as_slice()));
        }

        Ok(())
    }

    async fn upstream_request_filter(
        &self,
        session: &mut Session,
        upstream_request: &mut RequestHeader,
        _ctx: &mut Self::CTX,
    ) -> pingora::Result<()>
    where
        Self::CTX: Send + Sync,
    {
        match session.req_header().uri.path() {
            "/proxy" => {
                match upstream_request
                    .headers
                    .get(HeaderKeys::INT_FP_JWT)
                {
                    None => {
                        error!(
                            log_type = LogTypes::HANDLE_CLIENT_REQUEST,
                            request_summary = session.request_summary(),
                            "Missing required header: {}",
                            HeaderKeys::INT_FP_JWT
                        );

                        return Err(pingora::Error::new(pingora::ErrorType::HTTPStatus(
                            u16::from(StatusCode::BAD_REQUEST),
                        )));
                    }
                    Some(token) => {
                        let token_str = token.to_str().or_err(
                            pingora::ErrorType::InvalidHTTPHeader,
                            "Invalid header value for token",
                        )?;

                        if token_str.is_empty() {
                            error!(
                                log_type = LogTypes::HANDLE_CLIENT_REQUEST,
                                request_summary = session.request_summary(),
                                "{} token is empty",
                                HeaderKeys::INT_FP_JWT
                            );

                            return Err(pingora::Error::new(pingora::ErrorType::HTTPStatus(
                                u16::from(StatusCode::BAD_REQUEST),
                            )));
                        }

                        match self.handler.verify_int_fp_jwt(token_str) {
                            Ok(session) => {
                                upstream_request
                                    .insert_header(
                                        HeaderKeys::FP_RP_JWT,
                                        session.fp_rp_jwt,
                                    )
                                    .unwrap_or_default();
                                upstream_request.remove_header(HeaderKeys::INT_FP_JWT);
                            }
                            Err(err) => {
                                error!(
                                    log_type = LogTypes::HANDLE_CLIENT_REQUEST,
                                    request_summary = session.request_summary(),
                                    "Error verifying {} token: {}",
                                    HeaderKeys::INT_FP_JWT,
                                    err
                                );
                                return Err(pingora::Error::explain(
                                    pingora::ErrorType::InvalidHTTPHeader,
                                    err,
                                ));
                            }
                        }
                    }
                }
            }
            _ => {}
        }

        if upstream_request.headers.get("x-empty-body").is_none() {
            upstream_request.remove_header("content-length");
            upstream_request
                .insert_header(TRANSFER_ENCODING.as_str(), "chunked")
                .unwrap_or_default();
        }

        Ok(())
    }

    async fn response_filter(
        &self,
        _session: &mut Session,
        upstream_response: &mut ResponseHeader,
        _ctx: &mut Self::CTX,
    ) -> pingora::Result<()> {
        upstream_response.insert_header("Access-Control-Allow-Origin", "*")?;
        upstream_response.insert_header("Access-Control-Allow-Methods", "POST")?;
        upstream_response.insert_header("Access-Control-Allow-Headers", "*")?;

        if let Some(length) = upstream_response.headers.get("content-length") {
            if length != "0" {
                upstream_response.remove_header("content-length");
                upstream_response.insert_header(TRANSFER_ENCODING.as_str(), "chunked")?;
            }
        }

        Ok(())
    }

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
        if let Some(b) = body {
            ctx.extend_response_body(b.to_vec());
            // drop the body
            b.clear();
        }

        if end_of_stream {
            // This is the last chunk, we can process the data now
            debug!(
                log_type = LogTypes::HANDLE_UPSTREAM_RESPONSE,
                request_summary = session.request_summary(),
                body = utils::bytes_to_string(&ctx.get_response_body()),
            );
            info!(
                log_type = LogTypes::HANDLE_UPSTREAM_RESPONSE,
                request_summary = session.request_summary(),
                "Response Body Received: {} bytes.",
                &ctx.get_response_body().len(),
            );

            let handler_response = match session.req_header().uri.path() {
                "/init-tunnel" => self.handler.handle_init_tunnel_response(ctx),
                _ => {
                    info!(
                        log_type = LogTypes::HANDLE_UPSTREAM_RESPONSE,
                        request_summary = session.request_summary(),
                        "Forward proxy passing through response body unchanged."
                    );
                    *body = Some(Bytes::copy_from_slice(ctx.get_response_body().as_slice()));
                    return Ok(None);
                }
            };

            if handler_response.status != StatusCode::OK {
                error!(
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

    async fn logging(&self, session: &mut Session, e: Option<&Error>, ctx: &mut Self::CTX)
    where
        Self::CTX: Send + Sync,
    {
        let mut status = ctx.response.status.as_u16();
        if let Some(_err) = e {
            status = session.response_written().unwrap().status.as_u16();
        }
        info!(
            log_type=LogTypes::ACCESS_LOG_RESULT,
            client_ip=ctx.request.summary.host,
            request_summary=session.request_summary(),
            status=status,
            // duration_ms=session.duration_ms(),
            response_body_size=ctx.get_response_body().len(),
            user_agent=ctx.request.header.get("User-Agent"),
            request_id=session.req_header().headers.get("x-request-id").map(|v| v.to_str().unwrap_or("")).unwrap_or(""),
            error=?e,
        );

        let client_id = ctx
            .get(&CtxKeys::BACKEND_AUTH_CLIENT_ID.to_string())
            .unwrap_or(&"".to_string())
            .clone();
        let request_path = session.req_header().uri.path().to_string();
        let total_byte_transferred =
            (ctx.get_request_body().len() + ctx.get_response_body().len()) as i64;

        tokio::spawn(async move {
            Statistics::update(client_id, request_path, total_byte_transferred, status).await;
        });
    }

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
                .get(&CtxKeys::UPSTREAM_ADDRESS.to_string())
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
