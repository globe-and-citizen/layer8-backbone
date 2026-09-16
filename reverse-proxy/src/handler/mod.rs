use crate::config::{HandlerConfig, RPConfig};
use crate::handler::common::consts::LogTypes;
use crate::handler::healthcheck::{RpHealthcheckError, RpHealthcheckSuccess};
use init_tunnel::handler::InitTunnelHandler;
use init_tunnel::InitEncryptedTunnelResponse;
use ntor::common::{InitSessionMessage, NTorParty};
use ntor::server::NTorServer;
use pingora::http::StatusCode;
use pingora_router::ctx::{Layer8Context, Layer8ContextTrait};
use pingora_router::handler::{APIHandlerResponse, ResponseBodyTrait};
use proxy::handler::ProxyHandler;
use tracing::{error, info, Instrument};
use utils::jwt::JWTClaims;
use utils::new_uuid;

pub mod common;
pub mod init_tunnel;
pub mod proxy;
use crate::handler::common::types::ErrorResponse;
pub use init_tunnel::InMemorySecretsStorage;

mod healthcheck;

pub struct ReverseHandler {
    config: HandlerConfig,
    jwt_secret: Vec<u8>,
    ntor_static_secret: [u8; 32],
    reqwest_client: reqwest::Client,
}

impl ReverseHandler {
    pub fn new(config: RPConfig) -> Result<Self, reqwest::Error> {
        let ntor_secret = config.handler.ntor_static_secret;
        let jwt_secret = config.handler.jwt_virtual_connection_secret.clone();

        let reqwest_client = reqwest::Client::builder()
            .pool_max_idle_per_host(100)
            .tcp_keepalive(std::time::Duration::from_secs(60))
            .build()?;

        Ok(ReverseHandler {
            config: config.handler,
            jwt_secret,
            ntor_static_secret: ntor_secret,
            reqwest_client,
        })
    }

    /// Retrieves the nTor shared secret for a given session ID.
    ///
    /// # Arguments
    ///
    /// * `session_id` - The session identifier to look up in the shared secrets storage.
    ///
    /// # Returns
    ///
    /// Returns `Ok(Vec<u8>)` containing the shared secret if found, or `Err(String)`
    /// with HTTP 401 Unauthorized status if the session ID is invalid or expired.
    pub fn get_ntor_shared_secret(&self, session_id: &str) -> Result<Vec<u8>, String> {
        let shared_secret = InMemorySecretsStorage::get(session_id);

        match shared_secret {
            Some(secret) => Ok(secret),
            None => Err("Session ID not found".to_string()),
        }
    }

    /// Handles the initialization of an encrypted tunnel with nTor protocol.
    ///
    /// This method processes tunnel initialization requests by:
    /// 1. Validating the request body and extracting the client's public key
    /// 2. Initializing an nTor server instance with configured server ID and static secret
    /// 3. Creating an nTor session from the client's public key
    /// 4. Generating session ID and JWT tokens for internal and frontend use
    /// 5. Storing the shared secret by sessionID in thread-local storage for later proxy requests
    /// 6. Returning the server's public key, ntor t_b_hash, and JWT tokens to the client
    ///
    /// # Arguments
    ///
    /// * `ctx` - A mutable reference to the Layer8Context containing the HTTP request data.
    ///
    /// # Returns
    ///
    /// Returns an `APIHandlerResponse` with:
    /// - `StatusCode::OK` (200) and encrypted tunnel initialization response on success
    /// - `StatusCode::BAD_REQUEST` (400) if the public key length is invalid or request validation fails
    /// - Other error status codes if request body validation fails
    ///
    /// # Errors
    ///
    /// This function may return error responses from request body validation or invalid public key length.
    pub async fn handle_init_tunnel(&self, ctx: &mut Layer8Context) -> APIHandlerResponse {
        // validate request body
        let request_body = match InitTunnelHandler::validate_request_body(ctx).await {
            Ok(res) => res,
            Err(res) => return res,
        };

        // initialize NTorServer object with configured server ID and static secret
        let mut ntor_server = NTorServer::new_with_secret(
            self.config.ntor_server_id.clone(),
            self.ntor_static_secret,
        );

        // create nTor session from client's public key
        let init_session_response = {
            let init_session_msg = InitSessionMessage::from(request_body.public_key);
            ntor_server.accept_init_session_request(&init_session_msg)
        };

        // generate new sessionID
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

        ctx.info(|| {
            info!(
                log_type = LogTypes::HANDLE_INIT_TUNNEL_REQUEST,
                "Save new nTor session: {}", ntor_session_id
            );
        });

        InMemorySecretsStorage::insert(
            ntor_session_id,
            ntor_server.get_shared_secret().unwrap_or_default(),
        );

        APIHandlerResponse {
            status: StatusCode::OK,
            cookies: None,
            body: Some(response.to_bytes()),
        }
    }

    /// Handles proxy requests with nTor encryption/decryption.
    ///
    /// This method processes encrypted proxy requests from clients by:
    /// 1. Validating the request headers and extracting the nTor session ID from JWT
    /// 2. Retrieving the shared secret associated with the session
    /// 3. Validating and decrypting the request body using nTor
    /// 4. Reconstructing and forwarding the user request to the backend
    /// 5. Encrypting the backend response and returning it to the client
    ///
    /// # Arguments
    ///
    /// * `ctx` - A mutable reference to the Layer8Context containing the HTTP request data.
    ///
    /// # Returns
    ///
    /// Returns an `APIHandlerResponse` with:
    /// - `StatusCode::OK` (200) and encrypted response body on success
    /// - `StatusCode::UNAUTHORIZED` (401) if the JWT is invalid or session ID not found
    /// - `StatusCode::BAD_REQUEST` (400) if request validation fails
    /// - Other error status codes if decryption, backend communication, or encryption fails
    ///
    /// # Errors
    ///
    /// This function may return error responses from header validation, secret retrieval,
    /// request body validation, decryption operations, backend request processing, or encryption failures.
    pub async fn handle_proxy_request(&self, ctx: &mut Layer8Context) -> APIHandlerResponse {
        let mut parent_span = tracing::Span::current();
        if parent_span.is_none() {
            parent_span = ctx.get_request_span().clone();
        }

        // Synchronous request parsing and decryption block
        let (_session_id, shared_secret, wrapped_request) = {
            let request_handler_span =
                tracing::info_span!(parent: parent_span.clone(), "FP.request.validate_and_decrypt");
            let _request_handler_guard = request_handler_span.enter();

            // validate request headers (nTor session ID)
            let session_id = match ProxyHandler::validate_request_headers(ctx, &self.jwt_secret) {
                Ok(session_id) => session_id,
                Err(err) => {
                    ctx.error(|| {
                        error!(
                            log_type = LogTypes::HANDLE_PROXY_REQUEST,
                            "Failed to validate request headers: {}", err
                        );
                    });

                    return APIHandlerResponse {
                        status: StatusCode::UNAUTHORIZED,
                        cookies: None,
                        body: Some(
                            ErrorResponse {
                                error: "Failed to validate request headers".to_string(),
                            }
                            .to_bytes(),
                        ),
                    };
                }
            };

            let shared_secret = match self.get_ntor_shared_secret(&session_id) {
                Ok(secret) => secret,
                Err(err) => {
                    ctx.error(|| {
                        error!(
                            log_type = LogTypes::HANDLE_PROXY_REQUEST,
                            "Failed to retrieve nTor shared secret: {}", err
                        );
                    });

                    return APIHandlerResponse {
                        status: StatusCode::UNAUTHORIZED,
                        cookies: None,
                        body: Some(ErrorResponse { error: err }.to_bytes()),
                    };
                }
            };

            // validate request body
            let request_body = match ProxyHandler::parse_request_body(ctx) {
                Ok(res) => res,
                Err(res) => {
                    ctx.error(|| {
                        error!(
                            log_type = LogTypes::HANDLE_PROXY_REQUEST,
                            "Failed to parse request body: {}", res
                        );
                    });

                    return APIHandlerResponse {
                        status: StatusCode::BAD_REQUEST,
                        cookies: None,
                        body: Some(
                            ErrorResponse {
                                error: "Failed to parse request body".to_string(),
                            }
                            .to_bytes(),
                        ),
                    };
                }
            };

            // decrypt request body using nTor shared secret
            let wrapped_request = match ProxyHandler::decrypt_request_body(
                request_body,
                self.config.ntor_server_id.clone(),
                &shared_secret,
            ) {
                Ok(req) => req,
                Err(res) => {
                    ctx.error(|| {
                        error!(
                            log_type = LogTypes::HANDLE_PROXY_REQUEST,
                            "Failed to decrypt request body: {}", res
                        );
                    });

                    return APIHandlerResponse {
                        status: StatusCode::BAD_REQUEST,
                        cookies: None,
                        body: Some(
                            ErrorResponse {
                                error: "Failed to decrypt request body".to_string(),
                            }
                            .to_bytes(),
                        ),
                    };
                }
            };

            (session_id, shared_secret, wrapped_request)
        };

        // Asynchronous backend round-trip block instrumented with a single span
        let be_request_span = tracing::info_span!(
            parent: parent_span.clone(),
            "BE.request",
            request_size_bytes = wrapped_request.body.len(),
            response_size_bytes = tracing::field::Empty,
        );

        let wrapped_response = match async {
            // reconstruct user request
            let (response, origin_url) = ProxyHandler::rebuild_user_request(
                &self.reqwest_client,
                ctx,
                self.config.backend_url.clone(),
                wrapped_request,
            )
            .await
            .map_err(|res| {
                ctx.error(|| {
                    error!(
                        log_type = LogTypes::HANDLE_PROXY_REQUEST,
                        "Failed to process backend request: {}", res
                    );
                });

                APIHandlerResponse {
                    status: StatusCode::BAD_GATEWAY,
                    cookies: None,
                    body: Some(
                        ErrorResponse {
                            error: "Failed to process backend request".to_string(),
                        }
                        .to_bytes(),
                    ),
                }
            })?;

            // wrap backend response into L8ResponseObject
            let wrapped_resp =
                ProxyHandler::wrap_backend_response(ctx, response, &origin_url).await;

            tracing::Span::current().record("response_size_bytes", wrapped_resp.body.len());

            Ok(wrapped_resp)
        }
        .instrument(be_request_span)
        .await
        {
            Ok(res) => res,
            Err(error_response) => return error_response,
        };

        // Synchronous response encryption and return block
        let handle_response_span =
            tracing::info_span!(parent: parent_span, "BE.response.wrap_and_encrypt");
        let _handle_response_guard = handle_response_span.enter();

        // get cookies from backend response if exist to set in the response to client
        let cookies: Option<String> = wrapped_response
            .headers
            .get("set-cookie")
            .and_then(|value| Some(value.to_string()));

        // encrypt backend response using nTor shared secret and return to client
        match ProxyHandler::encrypt_response_body(
            wrapped_response,
            self.config.ntor_server_id.clone(),
            &shared_secret,
        ) {
            Ok(encrypted_message) => APIHandlerResponse {
                status: StatusCode::OK,
                cookies,
                body: Some(encrypted_message.to_bytes()),
            },
            Err(err) => {
                ctx.error(|| {
                    error!(
                        log_type = LogTypes::HANDLE_PROXY_REQUEST,
                        "Failed to encrypt response body: {}", err
                    );
                });
                APIHandlerResponse {
                    status: StatusCode::INTERNAL_SERVER_ERROR,
                    cookies: None,
                    body: Some(
                        ErrorResponse {
                            error: "Failed to encrypt response body".to_string(),
                        }
                        .to_bytes(),
                    ),
                }
            }
        }
    }

    /// Handles health check requests for the reverse proxy.
    ///
    /// This method processes health check requests and returns appropriate responses based on
    /// optional error parameters. It supports both success and error scenarios for monitoring
    /// and diagnostics purposes.
    ///
    /// # Arguments
    ///
    /// * `ctx` - A mutable reference to the Layer8Context containing the HTTP request data.
    ///
    /// # Returns
    ///
    /// Returns an `APIHandlerResponse` with:
    /// - `StatusCode::IM_A_TEAPOT` (418) and error details if the `error=true` query parameter is present
    /// - `StatusCode::OK` (200) and success details otherwise
    ///
    /// Both responses include appropriate response headers (`x-rp-healthcheck-error` or `x-rp-healthcheck-success`).
    pub async fn handle_healthcheck(&self, ctx: &mut Layer8Context) -> APIHandlerResponse {
        if let Some(error) = ctx.param("error")
            && error == "true"
        {
            let response_bytes = RpHealthcheckError {
                rp_healthcheck_error: "this is placeholder for a custom error".to_string(),
            }
            .to_bytes();

            ctx.insert_response_header("x-rp-healthcheck-error", "response-header-error");
            return APIHandlerResponse {
                status: StatusCode::IM_A_TEAPOT,
                cookies: None,
                body: Some(response_bytes),
            };
        }

        let response_bytes = RpHealthcheckSuccess {
            rp_healthcheck_success: "this is placeholder for a custom body".to_string(),
        }
        .to_bytes();

        ctx.insert_response_header("x-rp-healthcheck-success", "response-header-success");

        APIHandlerResponse {
            status: StatusCode::OK,
            cookies: None,
            body: Some(response_bytes),
        }
    }
}
