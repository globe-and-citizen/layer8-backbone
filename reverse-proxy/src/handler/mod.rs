use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard};
use log::{debug};
use ntor::common::{InitSessionMessage, NTorParty};
use ntor::server::NTorServer;
use pingora::http::StatusCode;
use pingora_router::ctx::{Layer8Context, Layer8ContextTrait};
use pingora_router::handler::{APIHandlerResponse, RequestBodyTrait, ResponseBodyTrait};
use crate::handler::common::consts::HeaderKeys::{RpHeaderRequestKey, RpHeaderResponseKey};
use init_tunnel::handler::InitTunnelHandler;
use proxy::handler::ProxyHandler;
use init_tunnel::{InitEncryptedTunnelResponse};
use proxy::{ProxyRequestToBackend};
use crate::config::{HandlerConfig};
use crate::handler::common::handler::CommonHandler;
use crate::handler::common::utils::{new_uuid, string_to_array32};

mod common;
mod init_tunnel;
mod proxy;

thread_local! {
    static TEMPORARY_MEMORY: Mutex<HashMap<String, Vec<u8>>> = Mutex::new(HashMap::new());
}

pub struct ReverseHandler {
    config: HandlerConfig,
    ntor_static_secret: [u8; 32],
}

impl ReverseHandler {
    pub fn new(config: HandlerConfig) -> Self {
        let ntor_secret = string_to_array32(config.ntor_static_secret.clone()).unwrap();

        ReverseHandler {
            config,
            ntor_static_secret: ntor_secret,
        }
    }

    pub async fn handle_init_tunnel(&self, ctx: &mut Layer8Context) -> APIHandlerResponse {
        // validate request body
        let request_body = match InitTunnelHandler::validate_request_body(ctx).await {
            Ok(res) => res,
            Err(res) => return res
        };
        debug!("[REQUEST /init-tunnel] Parsed body: {:?}", request_body);

        // todo I think there are prettier ways to use nTor since we are free to modify the nTor crate, but I'm lazy
        let mut ntor_server = NTorServer::new_with_secret(
            self.config.ntor_server_id.clone(),
            self.ntor_static_secret,
        );

        if request_body.public_key.len() != 32 {
            return APIHandlerResponse {
                status: StatusCode::BAD_REQUEST,
                body: Some("Invalid public key length".as_bytes().to_vec()),
            };
        }

        // Client initializes session with the server
        let init_session_msg = InitSessionMessage::from(request_body.public_key);

        let init_session_response = ntor_server.accept_init_session_request(&init_session_msg);

        let ntor_session_id = new_uuid();

        let response = InitEncryptedTunnelResponse {
            public_key: init_session_response.public_key(),
            t_b_hash: init_session_response.t_b_hash(),
            session_id: ntor_session_id.clone(),
        };

        // set ReverseProxy's response header
        ctx.insert_response_header(
            RpHeaderResponseKey.as_str(),
            RpHeaderResponseKey.placeholder_value()
        );

        InitTunnelHandler::send_result_to_be(true).await;

        TEMPORARY_MEMORY.with(|memory| {
            let mut guard: MutexGuard<HashMap<String, Vec<u8>>> = memory.lock().unwrap();
            guard.insert(ntor_session_id, ntor_server.get_shared_secret().unwrap());
        });

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