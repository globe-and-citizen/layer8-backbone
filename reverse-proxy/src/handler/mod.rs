use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard};
use ntor::common::{InitSessionMessage, NTorParty};
use ntor::server::NTorServer;
use pingora::http::StatusCode;
use tracing::{debug, info, Level, span};
use pingora_router::ctx::{Layer8Context, Layer8ContextTrait};
use pingora_router::handler::{APIHandlerResponse, ResponseBodyTrait};
use init_tunnel::handler::InitTunnelHandler;
use proxy::handler::ProxyHandler;
use init_tunnel::InitEncryptedTunnelResponse;
use utils::{new_uuid};
use utils::jwt::JWTClaims;
use crate::config::{HandlerConfig, RPConfig};
use crate::handler::common::consts::LogTypes;
use crate::handler::healthcheck::{RpHealthcheckError, RpHealthcheckSuccess};

pub(crate) mod common;
mod init_tunnel;
mod proxy;
mod healthcheck;

thread_local! {
    // <session_id, shared_secret>
    static NTOR_SHARED_SECRETS: Mutex<HashMap<String, Vec<u8>>> = Mutex::new(HashMap::new());
}

pub struct ReverseHandler {
    config: HandlerConfig,
    jwt_secret: Vec<u8>,
    ntor_static_secret: [u8; 32],
}

impl ReverseHandler {
    pub fn new(config: RPConfig) -> Self {
        let ntor_secret = config.handler.ntor_static_secret.clone();
        let jwt_secret = config.handler.jwt_virtual_connection_secret.clone();

        ReverseHandler {
            config: config.handler,
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
        // Attach the correlation ID to the tracing span
        let correlation_id = ctx.get_correlation_id();
        let span = span!(Level::TRACE, "track", %correlation_id);
        let _enter = span.enter();

        // validate request body
        let request_body = match InitTunnelHandler::validate_request_body(
            ctx,
            self.config.backend_url.clone(),
        ).await {
            Ok(res) => res,
            Err(res) => return res
        };

        debug!("Parsed body: {:?}", request_body);

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
            let mut claims = JWTClaims::new(Some(self.config.jwt_exp_in_hours));
            claims.ntor_session_id = Some(ntor_session_id.clone());
            utils::jwt::create_jwt_token(claims, &self.jwt_secret)
        };

        let fp_rp_jwt = {
            let claims = JWTClaims::new(Some(self.config.jwt_exp_in_hours));
            utils::jwt::create_jwt_token(claims, &self.jwt_secret)
        };

        let response = InitEncryptedTunnelResponse {
            public_key: init_session_response.public_key(),
            t_b_hash: init_session_response.t_b_hash(),
            int_rp_jwt,
            fp_rp_jwt,
        };

        // InitTunnelHandler::send_result_to_be(self.config.backend_url.clone(), true).await;
        info!(
            log_type=LogTypes::HANDLE_INIT_TUNNEL_REQUEST,
            "Save new nTor session: {}",
            ntor_session_id
        );
        NTOR_SHARED_SECRETS.with(|memory| {
            let mut guard: MutexGuard<HashMap<String, Vec<u8>>> = memory.lock().unwrap();
            guard.insert(ntor_session_id, ntor_server.get_shared_secret().unwrap_or_default());
        });

        APIHandlerResponse {
            status: StatusCode::OK,
            body: Some(response.to_bytes()),
        }
    }

    pub async fn handle_proxy_request(&self, ctx: &mut Layer8Context) -> APIHandlerResponse {
        // Attach the correlation ID to the tracing span
        let correlation_id = ctx.get_correlation_id();
        let span = span!(Level::TRACE, "track", %correlation_id);
        let _enter = span.enter();

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

        info!(
            log_type=LogTypes::HANDLE_INIT_TUNNEL_REQUEST,
            "Decrypted request body and forward to backend",
        );
        debug!("Decrypted request: {:?}", wrapped_request);

        // reconstruct user request
        let wrapped_response = match ProxyHandler::rebuild_user_request(
            self.config.backend_url.clone(),
            wrapped_request,
        ).await {
            Ok(res) => res,
            Err(res) => return res,
        };

        debug!("Wrapped Backend response: {:?}", wrapped_response);

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

    pub async fn handle_healthcheck(&self, ctx: &mut Layer8Context) -> APIHandlerResponse {
        if let Some(error) = ctx.param("error") {
            if error == "true" {
                let response_bytes = RpHealthcheckError {
                    rp_healthcheck_error: "this is placeholder for a custom error".to_string()
                }.to_bytes();

                ctx.insert_response_header("x-rp-healthcheck-error", "response-header-error");
                return APIHandlerResponse {
                    status: StatusCode::IM_A_TEAPOT,
                    body: Some(response_bytes),
                };
            }
        }

        let response_bytes = RpHealthcheckSuccess {
            rp_healthcheck_success: "this is placeholder for a custom body".to_string(),
        }.to_bytes();

        ctx.insert_response_header("x-rp-healthcheck-success", "response-header-success");

        return APIHandlerResponse {
            status: StatusCode::OK,
            body: Some(response_bytes),
        };
    }
}