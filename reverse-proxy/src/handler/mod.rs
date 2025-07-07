use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard};
use log::debug;
use ntor::common::{InitSessionMessage, NTorParty};
use ntor::server::NTorServer;
use pingora::http::StatusCode;
use pingora_router::ctx::{Layer8Context};
use pingora_router::handler::{APIHandlerResponse, ResponseBodyTrait};
use init_tunnel::handler::InitTunnelHandler;
use proxy::handler::ProxyHandler;
use init_tunnel::InitEncryptedTunnelResponse;
use utils::{new_uuid, string_to_array32};
use utils::jwt::JWTClaims;
use crate::config::{HandlerConfig, RPConfig};

mod common;
mod init_tunnel;
mod proxy;

thread_local! {
    // <session_id, shared_secret>
    static NTOR_SHARED_SECRETS: Mutex<HashMap<String, Vec<u8>>> = Mutex::new(HashMap::new());
}

pub struct ReverseHandler {
    config: HandlerConfig,
    host: String,
    jwt_secret: Vec<u8>,
    ntor_static_secret: [u8; 32],
}

impl ReverseHandler {
    pub fn new(config: RPConfig) -> Self {
        let ntor_secret = string_to_array32(config.handler.ntor_static_secret.clone()).unwrap();
        let jwt_secret = config.handler.jwt_virtual_connection_secret.as_bytes().to_vec();

        ReverseHandler {
            config: config.handler,
            host: config.server.host,
            jwt_secret,
            ntor_static_secret: ntor_secret,
        }
    }

    fn get_ntor_shared_secret(&self, session_id: String) -> Result<Vec<u8>, APIHandlerResponse> {
        let shared_secret = NTOR_SHARED_SECRETS.with(|memory| {
            let guard = memory.lock().unwrap();
            guard.get(&session_id).cloned()
        });

        return match shared_secret {
            Some(secret) => Ok(secret.clone()),
            None => {
                Err(APIHandlerResponse {
                    status: StatusCode::UNAUTHORIZED,
                    body: Some("Invalid or expired nTor session ID".as_bytes().to_vec()),
                })
            }
        };
    }

    pub async fn handle_init_tunnel(&self, ctx: &mut Layer8Context) -> APIHandlerResponse {
        // validate request body
        let request_body = match InitTunnelHandler::validate_request_body(
            ctx,
            self.config.backend_url.clone(),
        ).await {
            Ok(res) => res,
            Err(res) => return res
        };
        debug!("[REQUEST /init-tunnel] Parsed body: {:?}", request_body);

        // todo I think there are prettier ways to use nTor since we are free to modify the nTor crate, but I'm lazy
        let mut ntor_server = NTorServer::new_with_secret(
            self.config.ntor_server_id.clone(),
            self.ntor_static_secret,
        );

        let init_session_response = {
            if request_body.public_key.len() != 32 {
                return APIHandlerResponse {
                    status: StatusCode::BAD_REQUEST,
                    body: Some("Invalid public key length".as_bytes().to_vec()),
                };
            }

            // Client initializes session with the server
            let init_session_msg = InitSessionMessage::from(request_body.public_key);
            ntor_server.accept_init_session_request(&init_session_msg)
        };

        let ntor_session_id = new_uuid();

        let int_rp_jwt = {
            let mut claims = JWTClaims {
                iss: Some(self.host.clone()),
                sub: Some("ntor-session-id".to_string()), // todo, this is about to change
                aud: Some(ctx.request.get_client_base_url()),
                exp: 0,
                nbf: 0,
                iat: 0,
                jti: None,
                rp_host: None,
                ntor_session_id: Some(ntor_session_id.clone()),
            };
            claims.set_current_iat();
            claims.set_exp(self.config.jwt_exp);
            utils::jwt::create_jwt_token(claims, &self.jwt_secret)
        };

        let fp_rp_jwt = {
            let mut claims = JWTClaims {
                iss: Some(self.host.clone()),
                sub: None,
                aud: Some(self.config.forward_proxy_url.clone().unwrap_or("".to_string())),
                exp: 0,
                nbf: 0,
                iat: 0,
                jti: None,
                rp_host: None,
                ntor_session_id: None,
            };
            claims.set_current_iat();
            claims.set_exp(self.config.jwt_exp);
            utils::jwt::create_jwt_token(claims, &self.jwt_secret)
        };

        let response = InitEncryptedTunnelResponse {
            public_key: init_session_response.public_key(),
            t_b_hash: init_session_response.t_b_hash(),
            int_rp_jwt,
            fp_rp_jwt,
        };

        InitTunnelHandler::send_result_to_be(self.config.backend_url.clone(), true).await;

        NTOR_SHARED_SECRETS.with(|memory| {
            let mut guard: MutexGuard<HashMap<String, Vec<u8>>> = memory.lock().unwrap();
            guard.insert(ntor_session_id, ntor_server.get_shared_secret().unwrap());
        });

        APIHandlerResponse {
            status: StatusCode::OK,
            body: Some(response.to_bytes()),
        }
    }

    pub async fn handle_proxy_request(&self, ctx: &mut Layer8Context) -> APIHandlerResponse {

        // validate request headers (nTor session ID)
        let session_id = match ProxyHandler::validate_request_headers(ctx, &self.jwt_secret) {
            Ok(session_id) => session_id,
            Err(res) => return res,
        };

        let shared_secret = match self.get_ntor_shared_secret(session_id) {
            Ok(secret) => secret,
            Err(res) => return res,
        };

        // validate request body
        let request_body = match ProxyHandler::validate_request_body(ctx) {
            Ok(res) => res,
            Err(res) => return res,
        };

        let wrapped_request = match ProxyHandler::decrypt_request_body(
            request_body,
            self.config.ntor_server_id.clone(),
            shared_secret.clone(),
        ) {
            Ok(req) => req,
            Err(res) => return res,
        };
        debug!("[REQUEST /proxy] Decrypted request: {:?}", wrapped_request);

        // reconstruct user request
        let wrapped_response = match ProxyHandler::rebuild_user_request(
            self.config.backend_url.clone(),
            wrapped_request,
        ).await {
            Ok(res) => res,
            Err(res) => return res,
        };

        debug!("[RESPONSE /proxy] Wrapped Backend response: {:?}", wrapped_response);

        return match ProxyHandler::encrypt_response_body(
            wrapped_response,
            self.config.ntor_server_id.clone(),
            shared_secret,
        ) {
            Ok(encrypted_message) => {
                APIHandlerResponse {
                    status: StatusCode::OK,
                    body: Some(encrypted_message.to_bytes()),
                }
            }
            Err(res) => res
        };
    }
}