use log::{debug, error};
use pingora::http::StatusCode;
use pingora_router::ctx::{Layer8Context, Layer8ContextTrait};
use pingora_router::handler::{APIHandlerResponse, DefaultHandlerTrait, RequestBodyTrait, ResponseBodyTrait};
use crate::handler::common::consts::HeaderKeys::{RpHeaderRequestKey, RpHeaderResponseKey};
use init_tunnel::handler::InitTunnelHandler;
use proxy::handler::ProxyHandler;
use common::types::ErrorResponse;
use init_tunnel::{InitEncryptedTunnelRequest, InitEncryptedTunnelResponse};
use proxy::{ProxyRequest, ProxyRequestToBackend};
use crate::handler::common::handler::CommonHandler;

mod common;
mod init_tunnel;
mod proxy;

pub struct ReverseHandler {}

impl DefaultHandlerTrait for ReverseHandler {}

impl ReverseHandler {
    pub async fn handle_init_tunnel(&self, ctx: &mut Layer8Context) -> APIHandlerResponse {
        // validate request body
        let request_body = match ReverseHandler::parse_request_body::
        <InitEncryptedTunnelRequest, ErrorResponse>(&ctx.get_request_body())
        {
            Ok(res) => res.to_bytes(),
            Err(err) => {
                let body = match err {
                    None => None,
                    Some(err_response) => Some(err_response.to_bytes())
                };

                InitTunnelHandler::init_tunnel_result_to_be(false).await;

                return APIHandlerResponse {
                    status: StatusCode::BAD_REQUEST,
                    body,
                };
            }
        };
        debug!("[REQUEST /init-tunnel] Parsed body: {:?}", request_body);

        // todo validate request headers

        // set ReverseProxy's response header
        ctx.insert_response_header(RpHeaderResponseKey.as_str(), RpHeaderResponseKey.placeholder_value());

        // ReverseProxy's response body
        let response = InitEncryptedTunnelResponse {
            rp_response_body: "body added in ReverseProxy".to_string(),
        };

        InitTunnelHandler::init_tunnel_result_to_be(true).await;

        APIHandlerResponse {
            status: StatusCode::OK,
            body: Some(response.to_bytes()),
        }
    }

    pub async fn handle_proxy_request(&self, ctx: &mut Layer8Context) -> APIHandlerResponse {
        // validate request body
        let body = ctx.get_request_body();
        let request_body = match ReverseHandler::parse_request_body::<ProxyRequest, ErrorResponse>(&body) {
            Ok(proxy_request) => proxy_request,
            Err(err) => {
                let err_body = match err {
                    None => None,
                    Some(err_response) => {
                        error!("Error parsing request body: {}", err_response.error);
                        Some(err_response.to_bytes())
                    }
                };

                return APIHandlerResponse {
                    status: StatusCode::BAD_REQUEST,
                    body: err_body,
                };
            }
        };

        let new_body = ProxyRequestToBackend {
            spa_request_body: request_body.spa_request_body,
        };

        // todo validate request headers
        let new_header = CommonHandler::create_forward_request_headers(
            ctx,
            RpHeaderRequestKey.placeholder_value(),
            new_body.to_bytes().len(),
        );

        ProxyHandler::proxy_request_to_backend(ctx, new_header, new_body).await
    }
}