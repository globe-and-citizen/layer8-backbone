use crate::handler::common::consts::{HeaderKeys, LogTypes};
use crate::handler::proxy::{L8RequestObject, L8ResponseObject};
use ntor::common::{EncryptedMessage, NTorParty};
use ntor::server::NTorServer;
use pingora_router::ctx::{Layer8Context, Layer8ContextTrait};
use pingora_router::handler::{DefaultHandlerTrait, ResponseBodyTrait};
use reqwest::header::HeaderMap;
use reqwest::{Client, Response};
use tracing::{info, trace};
use utils::jwt::JWTClaims;

/// Struct containing only associated methods (no instance methods or fields)
pub struct ProxyHandler {}

impl DefaultHandlerTrait for ProxyHandler {}

impl ProxyHandler {
    /// Validates a JWT token from the request headers.
    ///
    /// This function retrieves a JWT token from the specified header and verifies signature
    /// validity and expiration time using the provided secret key.
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
    /// * `Err(String)` - An error response if the header is missing, empty, or token verification fails
    pub fn validate_jwt_token(
        ctx: &mut Layer8Context,
        header_key: &str,
        jwt_secret: &Vec<u8>,
    ) -> Result<JWTClaims, String> {
        match ctx.get_request_header().get(header_key) {
            None => Err(format!("Missing {} header", header_key)),
            Some(token) => {
                if token.is_empty() {
                    return Err(format!("Empty {} header", header_key));
                }

                // verify token
                match utils::jwt::verify_jwt_token(token, jwt_secret) {
                    Ok(data) => Ok(data.claims),
                    Err(err) => Err(format!("Error verifying {} token: {}", header_key, err)),
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
    /// * `Err(String)` - An error response if token validation fails or ntor_session_id is missing
    pub fn validate_request_headers(
        ctx: &mut Layer8Context,
        jwt_secret: &Vec<u8>,
    ) -> Result<String, String> {
        match ProxyHandler::validate_jwt_token(ctx, HeaderKeys::FP_RP_JWT, jwt_secret) {
            Ok(_claims) => {
                // todo!() nothing to validate at the moment
            }
            Err(err) => return Err(err),
        }

        match ProxyHandler::validate_jwt_token(ctx, HeaderKeys::INT_RP_JWT, jwt_secret) {
            Ok(claims) => {
                // extract ntor_session_id from claims
                match claims.ntor_session_id {
                    Some(ntor_session_id) => Ok(ntor_session_id),
                    None => Err("ntor_session_id is missing from JWT claims".to_string()),
                }
            }
            Err(err) => Err(err),
        }
    }

    /// Deserializes the request body from bincode format.
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
    /// * `Err(String)` - An error response if deserialization fails
    pub fn parse_request_body(ctx: &mut Layer8Context) -> Result<EncryptedMessage, String> {
        match EncryptedMessage::from_bytes(&ctx.get_request_body()) {
            Ok(res) => Ok(*res),
            Err(err) => Err(format!("Error parsing request body: {}", err)),
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
    pub fn decrypt_request_body(
        request_body: EncryptedMessage,
        ntor_server_id: String,
        shared_secret: &[u8],
    ) -> Result<L8RequestObject, String> {
        let mut ntor_server = NTorServer::new(ntor_server_id);
        ntor_server.set_shared_secret(shared_secret.to_vec());

        // Decrypt the request body using nTor shared secret
        let decrypted_data = ntor_server
            .decrypt(request_body)
            .map_err(|err| format!("Decryption failed: {}", err))?;
        // let decrypted_data = request_body.data;

        // parse decrypted data into WrappedUserRequest
        let wrapped_request: L8RequestObject = utils::bytes_to_json(decrypted_data)
            .map_err(|err| format!("Failed to parse request body: {}", err))?;

        Ok(wrapped_request)
    }

    /// Reconstructs and sends the user request to the backend server.
    ///
    /// This function takes the decrypted request object, rebuilds it with proper headers
    /// (including cookies from the original ctx `Layer8Context`), and forwards it to the backend URL.
    /// It then processes the backend response and returns it as an `L8ResponseObject`.
    ///
    /// # Arguments
    ///
    /// * `ctx` - The Layer8 context providing request correlation ID and access to cookies from the
    /// original request
    /// * `backend_url` - The base backend URL to which the URI from the wrapped request is appended
    /// to form the complete backend address
    /// * `wrapped_request` - The deserialized and decrypted request object containing HTTP method,
    /// headers, body, and URI needed to reconstruct the original request
    ///
    /// # Returns
    ///
    /// * `Ok(L8ResponseObject)` - The response object received from the backend containing status,
    /// headers, body, and metadata
    /// * `Err(String)` - An error response if the request fails or backend is unreachable
    pub async fn rebuild_user_request(
        ctx: &Layer8Context,
        backend_url: String,
        wrapped_request: L8RequestObject,
    ) -> Result<(Response, String), String> {
        // Get correlation ID for logging
        let correlation_id = ctx.get_correlation_id();

        // Reconstruct headers for the backend request, starting with headers from the wrapped request
        let mut header_map = utils::hashmap_to_headermap(&wrapped_request.headers)
            .unwrap_or_else(|_| HeaderMap::new());

        // Append cookies from the original request context if present
        if let Some(cookies) = ctx.request.header.get(reqwest::header::COOKIE.as_str())
            && let Ok(cookie_hv) = reqwest::header::HeaderValue::from_str(cookies.as_ref())
        {
            header_map.append(reqwest::header::COOKIE, cookie_hv);
        };

        // Construct the full backend URL by appending the URI from the wrapped request to configured base backend URL
        let origin_url = format!("{}{}", backend_url, wrapped_request.uri);

        trace!(
            %correlation_id,
            log_type=LogTypes::HANDLE_PROXY_REQUEST,
            "Send reconstructed request to origin backend URL: {}",
            origin_url
        );

        let client = Client::new();
        let response = client
            .request(
                wrapped_request.method.parse().unwrap_or_default(),
                origin_url.as_str(),
            )
            .headers(header_map.clone())
            .body(wrapped_request.body)
            .send()
            .await;

        match response {
            Ok(success_res) => Ok((success_res, origin_url)),
            Err(err) => {
                let status = err
                    .status()
                    .unwrap_or(reqwest::StatusCode::INTERNAL_SERVER_ERROR);

                Err(format!(
                    "Error while building request to BE: status={}, error={}",
                    status, err
                ))
            }
        }
    }

    pub async fn wrap_backend_response(
        ctx: &Layer8Context,
        be_response: Response,
        origin_url: &str,
    ) -> L8ResponseObject {
        let status = be_response.status().as_u16();
        let status_text = be_response
            .status()
            .canonical_reason()
            .unwrap_or("OK")
            .to_string();
        let ok = be_response.status().is_success();
        let url = be_response.url().to_string();
        let redirected = be_response.url().as_str() != origin_url;

        let serialized_headers = utils::headermap_to_hashmap(be_response.headers());
        let serialized_body = be_response.bytes().await.unwrap_or_default().to_vec();

        // Get correlation ID for logging
        info!(
            correlation_id = ctx.get_correlation_id(),
            log_type = LogTypes::HANDLE_BACKEND_RESPONSE,
            "Received response from backend: status={}, url={}",
            status,
            url.as_str()
        );

        L8ResponseObject {
            status,
            status_text,
            headers: serialized_headers,
            body: serialized_body,
            ok,
            url,
            redirected,
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
    /// * `Err(String)` - An error response if encryption fails
    pub fn encrypt_response_body(
        response_body: L8ResponseObject,
        ntor_server_id: String,
        shared_secret: &[u8],
    ) -> Result<EncryptedMessage, String> {
        let mut ntor_server = NTorServer::new(ntor_server_id);
        ntor_server.set_shared_secret(shared_secret.to_vec());

        let data = response_body.to_bytes();

        // Encrypt the response body using nTor shared secret
        let encrypted_data = ntor_server
            .encrypt(data)
            .map_err(|err| format!("Encryption failed: {}", err).to_string())?;

        Ok(EncryptedMessage {
            nonce: encrypted_data.nonce,
            data: encrypted_data.data,
        })
    }
}
