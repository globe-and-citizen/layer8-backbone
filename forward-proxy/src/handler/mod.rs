use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use pingora::http::StatusCode;
use reqwest::Client;
use pingora_router::ctx::{Layer8Context, Layer8ContextTrait};
use pingora_router::handler::{APIHandlerResponse, DefaultHandlerTrait, RequestBodyTrait};
use crate::handler::types::response::{ErrorResponse, FpHealthcheckError, FpHealthcheckSuccess, InitTunnelResponseFromRP, InitTunnelResponseToINT};
use pingora_router::handler::ResponseBodyTrait;
use serde::Deserialize;
use tracing::{debug, error, info};
use crate::handler::types::request::InitTunnelRequest;
use utils;
use utils::jwt::JWTClaims;
use crate::config::HandlerConfig;
use crate::handler::consts::LogTypes;

pub mod types;
pub mod consts;

pub struct ForwardHandler {
    pub config: HandlerConfig,
    jwts_storage: Arc<Mutex<HashMap<String, IntFPSession>>>,
}

impl DefaultHandlerTrait for ForwardHandler {}

#[derive(Debug)]
struct NTorServerCertificate {
    server_id: String,
    public_key: Vec<u8>,
}

#[derive(Clone, Debug, Default)]
pub struct IntFPSession {
    pub rp_base_url: String,
    pub fp_rp_jwt: String,
}

impl ForwardHandler {
    pub fn new(config: HandlerConfig) -> Self {
        ForwardHandler {
            config,
            jwts_storage: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    async fn get_public_key(
        &self,
        backend_url: String,
        ctx: &mut Layer8Context,
    ) -> Result<NTorServerCertificate, APIHandlerResponse>
    {
        let client = Client::new();

        //todo
        // the input backend_url is originally from interceptor request,
        // the interceptor only accepts URLs with http(s) scheme.
        // But the authentication server expects a URL without scheme
        let request_path = format!(
            "{}{}",
            self.config.auth_get_certificate_url,
            backend_url.replace("http://", "").replace("https://", "")
        );
        let res = client.get(&request_path)
            .header("Authorization", format!("Bearer {}", self.config.auth_access_token))
            .send()
            .await
            // unable to connect
            .map_err(|e| {
                let response_body = ErrorResponse {
                    error: format!("Failed to connect to layer8: {}", e)
                };

                APIHandlerResponse {
                    status: StatusCode::INTERNAL_SERVER_ERROR,
                    body: Some(response_body.to_bytes()),
                }
            })?;

        // connected but request failed
        if !res.status().is_success() {
            let response_body = ErrorResponse {
                error: format!("Failed to get public key from layer8, status code: {}", res.status().as_u16()),
            };
            error!(
                log_type=LogTypes::HANDLE_CLIENT_REQUEST,
                "Failed to get ntor certificate for {}: {:?}",
                request_path,
                response_body
            );

            ctx.insert_response_header("Connection", "close"); // Ensure connection closes???

            Err(APIHandlerResponse {
                status: StatusCode::BAD_REQUEST,
                body: Some(response_body.to_bytes()),
            })
        } else {
            #[derive(Deserialize, Debug)]
            struct AuthServerResponse {
                pub x509_certificate: String,
            }

            let cert: AuthServerResponse = res.json().await.map_err(|err| {
                error!(
                    log_type=LogTypes::HANDLE_CLIENT_REQUEST,
                    "Failed to parse authentication server response: {:?}",
                    err
                );
                APIHandlerResponse {
                    status: StatusCode::INTERNAL_SERVER_ERROR,
                    body: None,
                }
            })?;

            let pub_key = utils::cert::extract_x509_pem(cert.x509_certificate.clone())
                .map_err(|e| {
                    error!(
                        log_type=LogTypes::HANDLE_CLIENT_REQUEST,
                        "Failed to parse x509 certificate: {:?}",
                        e
                    );
                    APIHandlerResponse {
                        status: StatusCode::INTERNAL_SERVER_ERROR,
                        body: None,
                    }
                })?;

            debug!("AuthenticationServer response: {:?}", cert);
            info!(
                log_type=LogTypes::HANDLE_CLIENT_REQUEST,
                "Obtained ntor credentials for backend_url: {}",
                backend_url
            );

            Ok(NTorServerCertificate {
                server_id: backend_url, // todo I still prefer taking the server_id value from certificate's subject
                public_key: pub_key,
            })
        }
    }

    /// Verify `int_fp_jwt` and return `fp_rp_jwt`
    pub fn verify_int_fp_jwt(
        &self,
        token: &str,
    ) -> Result<IntFPSession, String> {
        return match utils::jwt::verify_jwt_token(token, &self.config.jwt_virtual_connection_key) {
            Ok(_claims) => {
                // todo check claims if needed

                match {
                    let jwts = self.jwts_storage.lock().unwrap();
                    jwts.get(token).cloned()
                } {
                    None => {
                        Err("token not found!".to_string())
                    }
                    Some(session) => Ok(session)
                }
            }
            Err(err) => Err(err.to_string())
        };
    }

    /// Validate request body and get ntor certificate for the given backend URL.
    pub async fn handle_init_tunnel_request(&self, ctx: &mut Layer8Context) -> APIHandlerResponse {
        // validate request body
        let received_body = match ForwardHandler::parse_request_body::<
            InitTunnelRequest,
            ErrorResponse
        >(&ctx.get_request_body())
        {
            Ok(res) => res.to_bytes(),
            Err(Some(e)) => {
                return APIHandlerResponse {
                    status: StatusCode::BAD_REQUEST,
                    body: Some(e.to_bytes()),
                };
            }
            Err(None) => {
                return APIHandlerResponse {
                    status: StatusCode::BAD_REQUEST,
                    body: None,
                };
            }
        };

        // get public key to initialize encrypted tunnel
        {
            // it's safe to use unwrap here because this param was already checked in `request_filter`
            let backend_url = ctx.param("backend_url").unwrap_or(&"".to_string()).to_string();

            let server_certificate = match self.get_public_key(backend_url.to_string(), ctx).await {
                Ok(cert) => cert,
                Err(err) => return err
            };
            debug!("Server certificate: {:?}", server_certificate);

            ctx.set(
                consts::CtxKeys::NTorServerId.to_string(),
                server_certificate.server_id,
            );
            ctx.set(
                consts::CtxKeys::NTorStaticPublicKey.to_string(),
                hex::encode(server_certificate.public_key),
            );
        }

        APIHandlerResponse {
            status: StatusCode::OK,
            body: Some(received_body),
        }
    }

    pub fn handle_init_tunnel_response(&self, ctx: &mut Layer8Context) -> APIHandlerResponse {
        let ntor_server_id = ctx.get(&consts::CtxKeys::NTorServerId.to_string()).unwrap_or(&"".to_string()).clone();
        let ntor_static_public_key = hex::decode(
            ctx.get(&consts::CtxKeys::NTorStaticPublicKey.to_string()).clone().unwrap_or(&"".to_string())
        ).unwrap_or_default();

        let response_body = ctx.get_response_body();

        return match utils::bytes_to_json::<InitTunnelResponseFromRP>(response_body) {
            Err(e) => {
                error!(
                    log_type=LogTypes::HANDLE_UPSTREAM_RESPONSE,
                    "Error parsing RP response: {:?}",
                    e
                );
                APIHandlerResponse {
                    status: StatusCode::INTERNAL_SERVER_ERROR,
                    body: None,
                }
            }
            Ok(res_from_rp) => {
                let int_fp_jwt = {
                    let mut claims = JWTClaims::new(Some(self.config.jwt_exp_in_hours));
                    claims.uuid = Some(utils::new_uuid());
                    utils::jwt::create_jwt_token(claims, &self.config.jwt_virtual_connection_key)
                };

                let int_fp_session = IntFPSession {
                    rp_base_url: ctx.param("backend_url").unwrap_or(&"".to_string()).to_string(),
                    fp_rp_jwt: res_from_rp.fp_rp_jwt,
                };

                let mut jwts = self.jwts_storage.lock().unwrap();
                jwts.insert(int_fp_jwt.clone(), int_fp_session);

                let res_to_int = InitTunnelResponseToINT {
                    ephemeral_public_key: res_from_rp.public_key,
                    t_b_hash: res_from_rp.t_b_hash,
                    int_rp_jwt: res_from_rp.int_rp_jwt,
                    int_fp_jwt,
                    ntor_static_public_key,
                    ntor_server_id,
                };

                APIHandlerResponse {
                    status: StatusCode::OK,
                    body: Some(res_to_int.to_bytes()),
                }
            }
        };
    }

    pub fn handle_healthcheck(&self, ctx: &mut Layer8Context) -> APIHandlerResponse {
        if let Some(error) = ctx.param("error") {
            if error == "true" {
                let response_bytes = FpHealthcheckError {
                    fp_healthcheck_error: "this is placeholder for a custom error".to_string()
                }.to_bytes();

                ctx.insert_response_header("x-fp-healthcheck-error", "response-header-error");
                return APIHandlerResponse {
                    status: StatusCode::IM_A_TEAPOT,
                    body: Some(response_bytes),
                };
            }
        }

        let response_bytes = FpHealthcheckSuccess {
            fp_healthcheck_success: "this is placeholder for a custom body".to_string(),
        }.to_bytes();

        ctx.insert_response_header("x-fp-healthcheck-success", "response-header-success");

        return APIHandlerResponse {
            status: StatusCode::OK,
            body: Some(response_bytes),
        };
    }
}