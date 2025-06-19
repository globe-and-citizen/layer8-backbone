use log::{debug};
use pingora::http::StatusCode;
use pingora_router::ctx::{Layer8Context, Layer8ContextTrait};
use pingora_router::handler::{APIHandlerResponse, RequestBodyTrait, ResponseBodyTrait};
use crate::handler::common::consts::HeaderKeys::{RpHeaderRequestKey, RpHeaderResponseKey};
use init_tunnel::handler::InitTunnelHandler;
use proxy::handler::ProxyHandler;
use init_tunnel::{InitEncryptedTunnelResponse};
use proxy::{ProxyRequestToBackend};
use crate::handler::common::handler::CommonHandler;

mod common;
mod init_tunnel;
mod proxy;

pub struct ReverseHandler {}

impl ReverseHandler {
    pub async fn handle_init_tunnel(&self, ctx: &mut Layer8Context) -> APIHandlerResponse {
        // validate request body
        let request_body = match InitTunnelHandler::validate_request_body(ctx).await {
            Ok(res) => res,
            Err(res) => return res
        };
        debug!("[REQUEST /init-tunnel] Parsed body: {:?}", request_body);

        // todo validate request headers

        // set ReverseProxy's response header
        ctx.insert_response_header(
            RpHeaderResponseKey.as_str(),
            RpHeaderResponseKey.placeholder_value()
        );

        // ReverseProxy's response body
        let response = InitEncryptedTunnelResponse {
            rp_response_body: "body added in ReverseProxy".to_string(),
        };

        InitTunnelHandler::send_result_to_be(true).await;

        APIHandlerResponse {
            status: StatusCode::OK,
            body: Some(response.to_bytes()),
        }
    }

    pub async fn handle_proxy_request(&self, ctx: &mut Layer8Context) -> APIHandlerResponse {
        // validate request body
        let request_body = match ProxyHandler::validate_request_body(ctx).await {
            Ok(res) => res,
            Err(res) => return res,
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