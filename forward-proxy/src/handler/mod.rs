use std::collections::HashMap;
use std::error::Error;
use std::sync::{Arc, Mutex};
use log::{error, info};
use pingora::http::StatusCode;
use reqwest::{Client};
use pingora_router::ctx::{Layer8Context, Layer8ContextTrait};
use pingora_router::handler::{APIHandlerResponse, DefaultHandlerTrait, RequestBodyTrait};
use crate::handler::types::response::{ErrorResponse, FpHealthcheckError, FpHealthcheckSuccess, InitTunnelResponseFromRP, InitTunnelResponseToINT};
use pingora_router::handler::ResponseBodyTrait;
use serde::Deserialize;
use crate::handler::types::request::{InitTunnelRequest};
use utils;
use utils::jwt::JWTClaims;

pub mod types;
pub mod consts;

pub struct ForwardConfig {
    pub jwt_secret: Vec<u8>,
    pub jwt_exp_in_hours: i64,
    pub auth_access_token: String,
}

pub struct ForwardHandler {
    pub config: ForwardConfig,
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
    pub fn new(config: ForwardConfig) -> Self {
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

        return match client
            .get(format!("{}{}", consts::LAYER8_GET_CERTIFICATE_PATH.as_str(), backend_url))
            .header("Authorization", format!("Bearer {}", self.config.auth_access_token))
            .send()
            .await
        {
            Ok(res) => {
                if !res.status().is_success() {
                    let response_body = ErrorResponse {
                        error: format!("Failed to get public key from layer8, status code: {}", res.status().as_u16()),
                    };
                    error!("Sending error response: {:?}", response_body);

                    ctx.insert_response_header("Connection", "close"); // Ensure connection closes???

                    Err(APIHandlerResponse {
                        status: StatusCode::BAD_REQUEST,
                        body: Some(response_body.to_bytes()),
                    })
                } else {

                    #[derive(Deserialize, Debug)]
                    struct AuthServerResponse {
                        pub x509_certificate: String
                    }

                    match res.json::<AuthServerResponse>().await {
                        Ok(cert) => {
                            let pub_key = match utils::cert::extract_x509_pem(cert.x509_certificate.clone()) {
                                Ok(pub_key) => pub_key,
                                Err(err) => {
                                    error!("Failed to parse x509 certificate: {:?}", err);
                                    return Err(APIHandlerResponse {
                                        status: StatusCode::INTERNAL_SERVER_ERROR,
                                        body: None,
                                    })
                                }
                            };

                            info!("AuthenticationServer response: {:?}", cert);

                            Ok(NTorServerCertificate {
                                server_id: backend_url, // todo consider server_id value
                                public_key: pub_key
                            })
                        }
                        Err(err) => {
                            error!("Failed to parse authentication server response: {:?}", err);
                            return Err(APIHandlerResponse {
                                status: StatusCode::INTERNAL_SERVER_ERROR,
                                body: None,
                            })
                        }
                    }
                }
            }
            Err(e) => {
                let response_body = ErrorResponse {
                    error: format!("Failed to connect to layer8: {}", e)
                };

                Err(APIHandlerResponse {
                    status: StatusCode::INTERNAL_SERVER_ERROR,
                    body: Some(response_body.to_bytes()),
                })
            }
        };
    }

    /// Verify `int_fp_jwt` and return `fp_rp_jwt`
    pub fn verify_int_fp_jwt(
        &self,
        token: &str,
    ) -> Result<IntFPSession, String> {
        return match utils::jwt::verify_jwt_token(token, &self.config.jwt_secret) {
            Ok(claims) => {
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
            Err(err) => {
                let body = match err {
                    None => None,
                    Some(e) => Some(e.to_bytes())
                };

                return APIHandlerResponse {
                    status: StatusCode::BAD_REQUEST,
                    body,
                };
            }
        };

        // get public key to initialize encrypted tunnel
        {
            // it's safe to use unwrap here because this param was already checked in `request_filter`
            let backend_url = ctx.param("backend_url").unwrap().to_string();

            let server_certificate = match self.get_public_key(backend_url.to_string(), ctx).await {
                Ok(cert) => cert,
                Err(err) => return err
            };
            info!("Server certificate: {:?}", server_certificate);

            ctx.set(
                consts::NTOR_SERVER_ID.to_string(),
                // consts::NTOR_SERVER_ID_TMP_VALUE.to_string(), // replace with real value
                server_certificate.server_id
            );
            ctx.set(
                consts::NTOR_STATIC_PUBLIC_KEY.to_string(),
                // hex::encode(consts::NTOR_STATIC_PUBLIC_KEY_TMP_VALUE), // replace with real value
                hex::encode(server_certificate.public_key)
            );
        }

        APIHandlerResponse {
            status: StatusCode::OK,
            body: Some(received_body),
        }
    }

    pub fn handle_init_tunnel_response(&self, ctx: &mut Layer8Context) -> APIHandlerResponse {
        let ntor_server_id = ctx.get(&*consts::NTOR_SERVER_ID.to_string()).unwrap().clone();
        let ntor_static_public_key = hex::decode(
            ctx.get(&*consts::NTOR_STATIC_PUBLIC_KEY.to_string()).clone().unwrap()
        ).unwrap();

        let response_body = ctx.get_response_body();

        return match utils::bytes_to_json::<InitTunnelResponseFromRP>(response_body) {
            Err(e) => {
                error!("Error parsing RP response: {:?}", e);
                APIHandlerResponse {
                    status: StatusCode::INTERNAL_SERVER_ERROR,
                    body: None,
                }
            }
            Ok(res_from_rp) => {
                let int_fp_jwt = {
                    let mut claims = JWTClaims::new(Some(self.config.jwt_exp_in_hours));
                    claims.uuid = Some(utils::new_uuid());
                    utils::jwt::create_jwt_token(claims, &self.config.jwt_secret)
                };

                let int_fp_session = IntFPSession {
                    rp_base_url: ctx.param("backend_url").unwrap().to_string(),
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
        let error = ctx.param("error").unwrap();

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