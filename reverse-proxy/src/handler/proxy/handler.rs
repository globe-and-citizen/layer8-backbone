use pingora_router::ctx::{Layer8Context, Layer8ContextTrait};
use reqwest::header::HeaderMap;
use pingora_router::handler::{APIHandlerResponse, DefaultHandlerTrait, ResponseBodyTrait};
use ntor::common::{EncryptedMessage, NTorParty};
use ntor::server::NTorServer;
use reqwest::Client;
use pingora::http::StatusCode;
use tracing::{debug, error, info};
use utils::bytes_to_json;
use utils::jwt::JWTClaims;
use crate::handler::common::consts::{HeaderKeys, LogTypes};
use crate::handler::common::types::ErrorResponse;
use crate::handler::proxy::{L8ResponseObject, L8RequestObject};

/// Struct containing only associated methods (no instance methods or fields)
pub struct ProxyHandler {}

impl DefaultHandlerTrait for ProxyHandler {}

impl ProxyHandler {
    /// Validates a JWT token from the request headers.
    ///
    /// This function retrieves a JWT token from the specified header and verifies its authenticity
    /// using the provided secret key.
    ///
    /// # Arguments
    ///
    /// * `ctx` - The Layer8 context containing request headers
    /// * `header_key` - The name of the header containing the JWT token
    /// * `jwt_secret` - The secret key used to verify the JWT token
    ///
    /// # Returns
    ///
    /// * `Ok(JWTClaims)` - The verified JWT claims extracted from the token
    /// * `Err(APIHandlerResponse)` - An error response if the header is missing, empty, or token verification fails
    fn validate_jwt_token(
        ctx: &mut Layer8Context,
        header_key: &str,
        jwt_secret: &Vec<u8>,
    ) -> Result<JWTClaims, APIHandlerResponse> {
        match ctx.get_request_header().get(header_key) {
            None => {
                Err(APIHandlerResponse {
                    status: StatusCode::BAD_REQUEST,
                    cookies: None,
                    body: Some(ErrorResponse {
                        error: format!("Missing {} header", header_key.to_string()),
                    }.to_bytes()),
                })
            }
            Some(token) => {
                if token.is_empty() {
                    return Err(APIHandlerResponse {
                        status: StatusCode::BAD_REQUEST,
                        cookies: None,
                        body: Some(ErrorResponse {
                            error: format!("Empty {} header", header_key.to_string()),
                        }.to_bytes()),
                    });
                }

                // verify token
                match utils::jwt::verify_jwt_token(token, jwt_secret) {
                    Ok(data) => Ok(data.claims),
                    Err(err) => {
                        error!(
                            correlation_id=ctx.get_correlation_id(),
                            log_type=LogTypes::HANDLE_PROXY_REQUEST,
                            "Error verifying {} token: {:?}",
                            header_key,
                            err
                        );
                        Err(APIHandlerResponse {
                            status: StatusCode::BAD_REQUEST,
                            cookies: None,
                            body: Some(ErrorResponse {
                                error: err.to_string(),
                            }.to_bytes()),
                        })
                    }
                }
            }
        }
    }

    /// Validates the request headers and extracts the `ntor_session_id` from JWT claims.
    ///
    /// This function verifies two JWT tokens from the request headers:
    /// - `FP_RP_JWT`: Frontend-proxy JWT token (validation only)
    /// - `INT_RP_JWT_KEY`: Internal reverse-proxy JWT token (contains `ntor_session_id` in claims)
    ///
    /// # Arguments
    ///
    /// * `ctx` - The Layer8 context containing request headers
    /// * `jwt_secret` - The secret key used to verify JWT tokens
    ///
    /// # Returns
    ///
    /// * `Ok(String)` - The ntor_session_id extracted from INT_RP_JWT_KEY claims
    /// * `Err(APIHandlerResponse)` - An error response if token validation fails or ntor_session_id is missing
    pub(crate) fn validate_request_headers(
        ctx: &mut Layer8Context,
        jwt_secret: &Vec<u8>,
    ) -> Result<String, APIHandlerResponse>
    {
        match ProxyHandler::validate_jwt_token(ctx, HeaderKeys::FP_RP_JWT, jwt_secret) {
            Ok(_claims) => {
                // todo!() nothing to validate at the moment
            }
            Err(err) => return Err(err)
        }

        match ProxyHandler::validate_jwt_token(ctx, HeaderKeys::INT_RP_JWT, jwt_secret) {
            Ok(claims) => {
                // extract ntor_session_id from claims
                match claims.ntor_session_id {
                    Some(ntor_session_id) => Ok(ntor_session_id),
                    None => Err(APIHandlerResponse {
                        status: StatusCode::BAD_REQUEST,
                        cookies: None,
                        body: Some(ErrorResponse {
                            error: "Missing ntor_session_id in JWT claims".to_string(),
                        }.to_bytes()),
                    }),
                }
            }
            Err(err) => Err(err)
        }
    }

    /// Validates and deserializes the request body from bincode format.
    ///
    /// This function expects the request body to be encoded in bincode format
    /// and deserializes it into an `EncryptedMessage` structure.
    ///
    /// # Arguments
    ///
    /// * `ctx` - The Layer8 context containing the request body to validate
    ///
    /// # Returns
    ///
    /// * `Ok(EncryptedMessage)` - The deserialized encrypted message from the request body
    /// * `Err(APIHandlerResponse)` - An error response if deserialization fails
    pub(crate) fn validate_request_body(
        ctx: &mut Layer8Context
    ) -> Result<EncryptedMessage, APIHandlerResponse>
    {
        let correlation_id = ctx.get_correlation_id();

        // deserialize from bincode
        match utils::bincode_to_type(ctx.get_request_body().as_slice()) {
            Ok(res) => Ok(res),
            Err(err) => {
                error!(
                    %correlation_id,
                    log_type=LogTypes::HANDLE_PROXY_REQUEST,
                    "Error parsing request body: {}",
                    err
                );
                Err(APIHandlerResponse {
                    status: StatusCode::BAD_REQUEST,
                    cookies: None,
                    body: Some(
                        ErrorResponse {
                            error: format!("Error parsing request body: {}", err),
                        }.to_bytes(),
                    ),
                })
            }
        }
    }

    /// Decrypts the request body using nTor encryption.
    ///
    /// This function takes an encrypted message and decrypts it using the nTor server
    /// with the provided shared secret. The decrypted data is then parsed into an
    /// `L8RequestObject`.
    ///
    /// # Arguments
    ///
    /// * `request_body` - The encrypted message containing nonce and encrypted data
    /// * `ntor_server_id` - The nTor server identifier for decryption
    /// * `shared_secret` - The shared secret key used for decryption
    ///
    /// # Returns
    ///
    /// * `Ok(L8RequestObject)` - The decrypted and parsed request object
    /// * `Err(APIHandlerResponse)` - An error response if decryption or parsing fails
    pub(crate) fn decrypt_request_body(
        request_body: EncryptedMessage,
        ntor_server_id: String,
        shared_secret: Vec<u8>,
    ) -> Result<L8RequestObject, APIHandlerResponse>
    {
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
                    cookies: None,
                    body: Some(format!("Decryption failed: {}", err).as_bytes().to_vec()),
                };
            })?;
        // let decrypted_data = request_body.data;

        // parse decrypted data into WrappedUserRequest
        let wrapped_request: L8RequestObject = bytes_to_json(decrypted_data).map_err(|err| {
            return APIHandlerResponse {
                status: StatusCode::BAD_REQUEST,
                cookies: None,
                body: Some(
                    format!("Failed to parse request body: {}", err)
                        .as_bytes()
                        .to_vec(),
                ),
            };
        })?;

        Ok(wrapped_request)
    }

    /// Reconstructs the user request and sends it to the backend origin server.
    ///
    /// This function takes the decrypted request object, rebuilds it with proper headers
    /// (including cookies from the original context), and forwards it to the backend URL.
    /// It then processes the backend response and returns it as an `L8ResponseObject`.
    ///
    /// # Arguments
    ///
    /// * `ctx` - The Layer8 context containing the original request and correlation ID
    /// * `backend_url` - The base URL of the backend origin server
    /// * `wrapped_request` - The decrypted request object containing method, URI, headers, and body
    ///
    /// # Returns
    ///
    /// * `Ok(L8ResponseObject)` - The response from the backend including status, headers, and body
    /// * `Err(APIHandlerResponse)` - An error response if the backend request fails
    pub(crate) async fn rebuild_user_request(
        ctx: &Layer8Context,
        backend_url: String,
        wrapped_request: L8RequestObject,
    ) -> Result<L8ResponseObject, APIHandlerResponse>
    {
        let correlation_id = ctx.get_correlation_id();
        let mut header_map = utils::hashmap_to_headermap(&wrapped_request.headers)
            .unwrap_or_else(|_| HeaderMap::new());

        if let Some(cookies) = ctx.request.header.get(reqwest::header::COOKIE.as_str()) {
            if let Ok(cookie_hv) = reqwest::header::HeaderValue::from_str(cookies.as_ref()) {
                header_map.append(reqwest::header::COOKIE, cookie_hv);
            }
        };

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
        let response = client.request(
            wrapped_request.method.parse().unwrap_or_default(),
            origin_url.as_str(),
        )
            .headers(header_map.clone())
            .body(wrapped_request.body)
            .send()
            .await;

        match response {
            Ok(success_res) => {
                let status = success_res.status().as_u16();
                let status_text = success_res.status()
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
                let status = err.status().unwrap_or(reqwest::StatusCode::INTERNAL_SERVER_ERROR);
                let err_body = ErrorResponse {
                    error: format!("Backend error: {}", status),
                };

                Err(APIHandlerResponse {
                    status: StatusCode::BAD_GATEWAY,
                    cookies: None,
                    body: Some(err_body.to_bytes()),
                })
            }
        }
    }

    /// Encrypts the response body using nTor encryption.
    ///
    /// This function takes a response object and encrypts it using the nTor server
    /// with the provided shared secret. The encrypted data is returned as an
    /// `EncryptedMessage` containing the nonce and encrypted payload.
    ///
    /// # Arguments
    ///
    /// * `response_body` - The response object to be encrypted
    /// * `ntor_server_id` - The nTor server identifier for encryption
    /// * `shared_secret` - The shared secret key used for encryption
    ///
    /// # Returns
    ///
    /// * `Ok(EncryptedMessage)` - The encrypted message with nonce and encrypted data
    /// * `Err(APIHandlerResponse)` - An error response if encryption fails
    pub(crate) fn encrypt_response_body(
        response_body: L8ResponseObject,
        ntor_server_id: String,
        shared_secret: Vec<u8>,
    ) -> Result<EncryptedMessage, APIHandlerResponse>
    {
        let mut ntor_server = NTorServer::new(ntor_server_id);
        ntor_server.set_shared_secret(shared_secret);

        let data = response_body.to_bytes();

        // Encrypt the response body using nTor shared secret
        let encrypted_data = ntor_server.encrypt(data).map_err(|err| {
            return APIHandlerResponse {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                cookies: None,
                body: Some(format!("Encryption failed: {}", err).as_bytes().to_vec()),
            };
        })?;

        Ok(EncryptedMessage {
            nonce: encrypted_data.nonce,
            data: encrypted_data.data,
        })
    }
}
