use async_trait::async_trait;
use boring::x509::X509;
use bytes::Bytes;
use log::{error, info};
use pingora::Error;
use pingora::OrErr;
use pingora::http::{RequestHeader, ResponseHeader, StatusCode};
use pingora::listeners::tls::TLS_CONF_ERR;
use pingora::prelude::{HttpPeer, ProxyHttp, Session};
use pingora::upstreams::peer::PeerOptions;
use pingora::utils::tls::CertKey;
use pingora_router::ctx::{Layer8Context, Layer8ContextTrait};
use reqwest::header::TRANSFER_ENCODING;
use std::sync::Arc;
use std::time::Duration;
use crate::handler::consts::HeaderKeys;
use crate::handler::ForwardHandler;

pub struct ForwardProxy {
    handler: ForwardHandler,
}

impl ForwardProxy {
    pub fn new(handler: ForwardHandler) -> Self {
        ForwardProxy {
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
        _ctx: &mut Self::CTX,
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
        let mut peer = HttpPeer::new(
            String::from("localhost:6193"),
            true,
            "localhost".to_string(),
        );

        {
            let cert = X509::from_pem(&certs::cert())
                .or_err(TLS_CONF_ERR, "Failed to load FP's certificate")?;

            let ca_cert = X509::from_pem(&certs::ca_pem())
                .or_err(TLS_CONF_ERR, "Failed to load CA certificate")?;

            let key = boring::pkey::PKey::private_key_from_pem(&certs::key())
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
        let request_summary = session.request_summary();
        println!();
        info!("[REQUEST {}] {:?}", request_summary, ctx.request);
        println!();

        match session.req_header().method {
            pingora::http::Method::OPTIONS => {
                // Handle CORS preflight request
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

        match (
            session.req_header().uri.path(),
            session.req_header().method.as_str()
        ) {
            ("/healthcheck", "GET") => {
                let handler_response = self.handler.handle_healthcheck(ctx);
                let mut header = ResponseHeader::build(handler_response.status, None)?;
                let response_headers = header.headers.clone();
                for (key, val) in response_headers.iter() {
                    header.insert_header(key.clone(), val.clone()).unwrap();
                };

                let mut response_bytes = vec![];
                if let Some(body_bytes) = handler_response.body {
                    header
                        .insert_header("Content-length", &body_bytes.len().to_string())
                        .unwrap();
                    response_bytes = body_bytes;
                };

                session.write_response_header_ref(&header).await?;

                println!();
                info!("[RESPONSE {}] Header: {:?}", request_summary, header.headers);
                info!(
                    "[RESPONSE {}] Body: {}",
                    request_summary,
                    String::from_utf8_lossy(&*response_bytes)
                );
                println!();

                // Write the response body to the session after setting headers
                session
                    .write_response_body(Some(Bytes::from(response_bytes)), true)
                    .await?;

                return Ok(true);
            }
            ("/init-tunnel", "POST") => {}
            ("/proxy", "POST") => {}
            _ => {
                let header = ResponseHeader::build(StatusCode::NOT_FOUND, None)?;
                session.write_response_header_ref(&header).await?;
                session.set_keepalive(None);
                return Ok(true);
            }
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
            info!(
                "[REQUEST {}] Decoded body: {}",
                session.request_summary(),
                String::from_utf8_lossy(&*ctx.get_request_body()),
            );

            // This is the last chunk, we can process the data now

            let handler_response = match session.req_header().uri.path() {
                "/init-tunnel" => self.handler.handle_init_tunnel_request(ctx).await,
                _ => {
                    info!(
                        "[FORWARD {}] FP forward request body: {}",
                        session.request_summary(),
                        utils::bytes_to_string(&ctx.get_request_body())
                    );
                    *body = Some(Bytes::copy_from_slice(ctx.get_request_body().as_slice()));
                    return Ok(());
                }
            };

            if handler_response.status != StatusCode::OK {
                error!(
                    "[FORWARD {}] Error in request handler with status: {}, error: {}",
                    session.request_summary(),
                    handler_response.status,
                    utils::bytes_to_string(&handler_response.body.unwrap_or_default())
                );
                return Err(pingora::Error::new(
                    pingora::ErrorType::HTTPStatus(u16::from(handler_response.status)),
                ));
            }

            info!(
                "[FORWARD {}] Request handler response: status: {}, body: {}",
                session.request_summary(),
                handler_response.status,
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
                match upstream_request.headers.get(HeaderKeys::IntFpJwtKey.as_str()) {
                    None => {
                        error!(
                            "[REQUEST {}] Missing required header: {}",
                            session.request_summary(),
                            HeaderKeys::IntFpJwtKey.as_str()
                        );
                        return Err(pingora::Error::new(
                            pingora::ErrorType::HTTPStatus(u16::from(StatusCode::BAD_REQUEST)),
                        ));
                    }
                    Some(token) => {
                        let token_str = token.to_str().or_err(
                            pingora::ErrorType::InvalidHTTPHeader,
                            "Invalid header value for token",
                        )?;

                        if token_str.is_empty() {
                            error!(
                                "[REQUEST {}] Empty value for header: {}",
                                session.request_summary(),
                                HeaderKeys::IntFpJwtKey.as_str()
                            );
                            return Err(pingora::Error::new(
                                pingora::ErrorType::HTTPStatus(u16::from(StatusCode::BAD_REQUEST)),
                            ));
                        }

                        match self.handler.verify_token(token_str) {
                            Ok(fp_rp_jwt) => {
                                upstream_request
                                    .insert_header(HeaderKeys::FpRpJwtKey.as_str(), fp_rp_jwt)
                                    .unwrap();
                                upstream_request
                                    .remove_header(HeaderKeys::IntFpJwtKey.as_str())
                                    .unwrap();
                            }
                            Err(err) => {
                                error!(
                                    "[REQUEST {}] Error verify {} token: {}",
                                    session.request_summary(),
                                    HeaderKeys::IntFpJwtKey.as_str(),
                                    err
                                );
                                return Err(
                                    pingora::Error::explain(
                                        pingora::ErrorType::InvalidHTTPHeader,
                                        err
                                    )
                                )
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
                .unwrap();
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
            info!(
                "[FORWARD {}] RP Response decoded body: {}",
                session.request_summary(),
                String::from_utf8_lossy(&*ctx.get_response_body()),
            );

            let handler_response = match session.req_header().uri.path() {
                "/init-tunnel" => self.handler.handle_init_tunnel_response(ctx),
                _ => {
                    info!(
                        "[RESPONSE {}] FP forward response body: {}",
                        session.request_summary(),
                        utils::bytes_to_string(&ctx.get_response_body())
                    );
                    *body = Some(Bytes::copy_from_slice(ctx.get_response_body().as_slice()));
                    return Ok(None);
                }
            };

            if handler_response.status != StatusCode::OK {
                error!(
                    "[RESPONSE {}] Error in response handler with status: {}, error: {}",
                    session.request_summary(),
                    handler_response.status,
                    utils::bytes_to_string(&handler_response.body.unwrap_or_default())
                );
                return Err(pingora::Error::new(
                    pingora::ErrorType::HTTPStatus(
                        u16::from(StatusCode::INTERNAL_SERVER_ERROR)
                    ),
                ));
            }

            info!(
                "[RESPONSE {}] FP response with status: {}, body: {}",
                session.request_summary(),
                handler_response.status,
                utils::bytes_to_string(&handler_response.body.as_ref().unwrap_or(&vec![]))
            );
            let fp_res_body = handler_response.body.as_ref().unwrap_or(&vec![]).clone();

            *body = Some(Bytes::copy_from_slice(fp_res_body.as_slice()));
        }

        Ok(None)
    }

    fn fail_to_connect(
        &self,
        _session: &mut Session,
        _peer: &HttpPeer,
        _ctx: &mut Self::CTX,
        e: Box<Error>,
    ) -> Box<Error> {
        error!("Failed to connect to upstream: {}", e);
        e
    }
}

mod certs {
    use std::fs;

    pub fn ca_pem() -> Vec<u8> {
        fs::read(std::env::var("PATH_TO_CA_CERT").expect("PATH_TO_CA_PERM must be set"))
            .expect("Failed to read CA PEM file")
    }

    pub fn cert() -> Vec<u8> {
        fs::read(std::env::var("PATH_TO_CERT").expect("PATH_TO_CERT must be set"))
            .expect("Failed to read certificate file")
    }

    pub fn key() -> Vec<u8> {
        fs::read(std::env::var("PATH_TO_KEY").expect("PATH_TO_KEY must be set"))
            .expect("Failed to read key file")
    }
}
