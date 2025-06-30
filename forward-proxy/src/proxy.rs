use std::time::Duration;
use async_trait::async_trait;
use bytes::Bytes;
use log::{error, info};
use pingora::Error;
use pingora::prelude::{HttpPeer, ProxyHttp, Session};
use pingora::http::{RequestHeader, ResponseHeader, StatusCode};
use pingora_router::ctx::{Layer8Context, Layer8ContextTrait};
use reqwest::header::TRANSFER_ENCODING;
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
        Ok(Box::from(HttpPeer::new(
            String::from("127.0.0.1:6194"),
            false,
            String::from("localhost"),
        )))
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

        match session.req_header().uri.path() {
            "/healthcheck" => {
                let handler_response = self.handler.handle_healthcheck(ctx);
                let mut header = ResponseHeader::build(handler_response.status, None)?;
                let response_headers = header.headers.clone();
                for (key, val) in response_headers.iter() {
                    header.insert_header(key.clone(), val.clone()).unwrap();
                };

                let mut response_bytes = vec![];
                if let Some(body_bytes) = handler_response.body {
                    header.insert_header("Content-length", &body_bytes.len().to_string()).unwrap();
                    response_bytes = body_bytes;
                };

                session.write_response_header_ref(&header).await?;

                println!();
                info!("[RESPONSE {}] Header: {:?}", request_summary, header.headers);
                info!("[RESPONSE {}] Body: {}", request_summary, String::from_utf8_lossy(&*response_bytes));
                println!();

                // Write the response body to the session after setting headers
                session.write_response_body(Some(Bytes::from(response_bytes)), true).await?;
                return Ok(true);
            }
            "/init-tunnel" => {}
            "/proxy" => {}
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
        _session: &mut Session,
        upstream_request: &mut RequestHeader,
        _ctx: &mut Self::CTX,
    ) -> pingora::Result<()>
    where
        Self::CTX: Send + Sync,
    {
        upstream_request // is this still needed?
            .insert_header("fp_request_header", "fp_request_value")
            .unwrap();

        upstream_request
            .insert_header(TRANSFER_ENCODING.as_str(), "chunked")
            .unwrap();

        Ok(())
    }

    async fn response_filter(
        &self,
        _session: &mut Session,
        upstream_response: &mut ResponseHeader,
        _ctx: &mut Self::CTX,
    ) -> pingora::Result<()> {
        upstream_response.insert_header("Access-Control-Allow-Origin", "*")?;
        upstream_response.insert_header("Access-Control-Allow-Methods", "*")?;
        upstream_response.insert_header("Access-Control-Allow-Headers", "*")?;
        upstream_response.insert_header(TRANSFER_ENCODING.as_str(), "chunked")?;

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
