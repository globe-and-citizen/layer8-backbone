use crate::handler::common::consts::{HeaderKeys, LogTypes};
use crate::handler::common::types::ErrorResponse;
use crate::handler::proxy::{EncryptedMessage, L8RequestObject, L8ResponseObject};
use ntor::common::NTorParty;
use ntor::server::NTorServer;
use pingora::http::StatusCode;
use pingora_router::ctx::{Layer8Context, Layer8ContextTrait};
use pingora_router::handler::{APIHandlerResponse, DefaultHandlerTrait, ResponseBodyTrait};
use reqwest::Client;
use reqwest::header::HeaderMap;
use tracing::{debug, error, info};
use utils::bytes_to_json;
use utils::jwt::JWTClaims;

/// Struct containing only associated methods (no instance methods or fields)
pub struct ProxyHandler {}

impl DefaultHandlerTrait for ProxyHandler {}

impl ProxyHandler {
    fn validate_jwt_token(
        ctx: &mut Layer8Context,
        header_key: &str,
        jwt_secret: &Vec<u8>,
    ) -> Result<JWTClaims, APIHandlerResponse> {
        match ctx.get_request_header().get(header_key) {
            None => {
                return Err(APIHandlerResponse {
                    status: StatusCode::BAD_REQUEST,
                    body: Some(
                        ErrorResponse {
                            error: format!("Missing {} header", header_key),
                        }
                        .to_bytes(),
                    ),
                });
            }
            Some(token) => {
                if token.is_empty() {
                    return Err(APIHandlerResponse {
                        status: StatusCode::BAD_REQUEST,
                        body: Some(
                            ErrorResponse {
                                error: format!("Empty {} header", header_key),
                            }
                            .to_bytes(),
                        ),
                    });
                }

                // verify token
                match utils::jwt::verify_jwt_token(token, jwt_secret) {
                    Ok(data) => Ok(data.claims),
                    Err(err) => {
                        error!(
                            correlation_id = ctx.get_correlation_id(),
                            log_type = LogTypes::HANDLE_PROXY_REQUEST,
                            "Error verifying {} token: {:?}",
                            header_key,
                            err
                        );
                        Err(APIHandlerResponse {
                            status: StatusCode::BAD_REQUEST,
                            body: Some(
                                ErrorResponse {
                                    error: err.to_string(),
                                }
                                .to_bytes(),
                            ),
                        })
                    }
                }
            }
        }
    }

    /// Validates the request headers and get ntor_session_id in return.
    pub(crate) fn validate_request_headers(
        ctx: &mut Layer8Context,
        jwt_secret: &Vec<u8>,
    ) -> Result<String, APIHandlerResponse> {
        // verify fp_rp_jwt header
        match ProxyHandler::validate_jwt_token(ctx, HeaderKeys::FP_RP_JWT, jwt_secret) {
            Ok(_claims) => {
                // todo!() nothing to validate at the moment
            }
            Err(err) => return Err(err),
        }

        return match ProxyHandler::validate_jwt_token(ctx, HeaderKeys::INT_RP_JWT_KEY, jwt_secret) {
            Ok(claims) => {
                // extract ntor_session_id from claims
                match claims.ntor_session_id {
                    Some(ntor_session_id) => Ok(ntor_session_id),
                    None => Err(APIHandlerResponse {
                        status: StatusCode::BAD_REQUEST,
                        body: Some(
                            ErrorResponse {
                                error: "Missing ntor_session_id in JWT claims".to_string(),
                            }
                            .to_bytes(),
                        ),
                    }),
                }
            }
            Err(err) => return Err(err),
        };
    }

    pub(crate) fn validate_request_body(
        ctx: &mut Layer8Context,
    ) -> Result<EncryptedMessage, APIHandlerResponse> {
        let correlation_id = ctx.get_correlation_id();

        match ProxyHandler::parse_request_body::<EncryptedMessage, ErrorResponse>(
            &ctx.get_request_body(),
        ) {
            Ok(res) => Ok(res),
            Err(err) => {
                let body = match err {
                    None => None,
                    Some(err_response) => {
                        error!(
                            %correlation_id,
                            log_type=LogTypes::HANDLE_PROXY_REQUEST,
                            "Error parsing request body: {}",
                            err_response.error
                        );
                        Some(err_response.to_bytes())
                    }
                };
                Err(APIHandlerResponse {
                    status: StatusCode::BAD_REQUEST,
                    body,
                })
            }
        }
    }

    pub(crate) fn decrypt_request_body(
        request_body: EncryptedMessage,
        ntor_server_id: String,
        shared_secret: Vec<u8>,
    ) -> Result<L8RequestObject, APIHandlerResponse> {
        let mut ntor_server = NTorServer::new(ntor_server_id);
        ntor_server.set_shared_secret(shared_secret.clone());

        // Decrypt the request body using nTor shared secret
        let decrypted_data = ntor_server
            .decrypt(ntor::common::EncryptedMessage {
                nonce: <[u8; 12]>::try_from(request_body.nonce).unwrap_or_default(),
                data: request_body.data,
            })
            .map_err(|err| {
                return APIHandlerResponse {
                    status: StatusCode::BAD_REQUEST,
                    body: Some(format!("Decryption failed: {}", err).as_bytes().to_vec()),
                };
            })?;
        // let decrypted_data = request_body.data;

        // parse decrypted data into WrappedUserRequest
        let wrapped_request: L8RequestObject = bytes_to_json(decrypted_data).map_err(|err| {
            return APIHandlerResponse {
                status: StatusCode::BAD_REQUEST,
                body: Some(
                    format!("Failed to parse request body: {}", err)
                        .as_bytes()
                        .to_vec(),
                ),
            };
        })?;

        Ok(wrapped_request)
    }

    pub(crate) async fn rebuild_user_request(
        ctx: &Layer8Context,
        backend_url: String,
        wrapped_request: L8RequestObject,
    ) -> Result<L8ResponseObject, APIHandlerResponse> {
        let correlation_id = ctx.get_correlation_id();
        let header_map = utils::hashmap_to_headermap(&wrapped_request.headers)
            .unwrap_or_else(|_| HeaderMap::new());
        debug!(
            %correlation_id,
            log_type=LogTypes::HANDLE_PROXY_REQUEST,
            backend_url=backend_url.as_str(),
            "Reconstructed request headers: {:?}",
            header_map
        );

        let origin_url = format!("{}{}", backend_url, wrapped_request.uri);

        let client = Client::new();
        info!(
            %correlation_id,
            log_type=LogTypes::HANDLE_PROXY_REQUEST,
            "Send reconstructed request to origin backend URL: {}",
            origin_url
        );
        let response = client
            .request(
                wrapped_request.method.parse().unwrap_or_default(),
                origin_url.as_str(),
            )
            .headers(header_map.clone())
            .body(wrapped_request.body)
            .send()
            .await;

        return match response {
            Ok(success_res) => {
                let status = success_res.status().as_u16();
                let status_text = success_res
                    .status()
                    .canonical_reason()
                    .unwrap_or("OK")
                    .to_string();
                let ok = success_res.status().is_success();
                let url = success_res.url().to_string();
                let redirected = success_res.url().as_str() != origin_url;

                let serialized_headers = utils::headermap_to_hashmap(&success_res.headers());
                let serialized_body = success_res.bytes().await.unwrap_or_default().to_vec();

                info!(
                    %correlation_id,
                    log_type=LogTypes::HANDLE_BACKEND_RESPONSE,
                    "Received response from backend: status={}, url={}",
                    status,
                    url.as_str()
                );

                Ok(L8ResponseObject {
                    status,
                    status_text,
                    headers: serialized_headers,
                    body: serialized_body,
                    ok,
                    url,
                    redirected,
                })
            }
            Err(err) => {
                error!(
                    %correlation_id,
                    log_type=LogTypes::HANDLE_PROXY_REQUEST,
                    "Error while building request to BE: {:?}",
                    err
                );
                let status = err
                    .status()
                    .unwrap_or(reqwest::StatusCode::INTERNAL_SERVER_ERROR);
                let err_body = ErrorResponse {
                    error: format!("Backend error: {}", status),
                };

                Err(APIHandlerResponse {
                    status: StatusCode::BAD_GATEWAY,
                    body: Some(err_body.to_bytes()),
                })
            }
        };
    }

    pub(crate) fn encrypt_response_body(
        response_body: L8ResponseObject,
        ntor_server_id: String,
        shared_secret: Vec<u8>,
    ) -> Result<EncryptedMessage, APIHandlerResponse> {
        let mut ntor_server = NTorServer::new(ntor_server_id);
        ntor_server.set_shared_secret(shared_secret);

        let data = response_body.to_bytes();

        // Encrypt the response body using nTor shared secret
        let encrypted_data = ntor_server.encrypt(data).map_err(|err| {
            return APIHandlerResponse {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                body: Some(format!("Encryption failed: {}", err).as_bytes().to_vec()),
            };
        })?;

        Ok(EncryptedMessage {
            nonce: encrypted_data.nonce.to_vec(),
            data: encrypted_data.data,
        })
    }
}
