use std::str::FromStr;
use pingora_router::ctx::{Layer8Context, Layer8ContextTrait};
use reqwest::header::{HeaderMap, HeaderName};
use pingora_router::handler::{APIHandlerResponse, DefaultHandlerTrait, ResponseBodyTrait};
use log::{debug, error, info};
use ntor::common::NTorParty;
use ntor::server::NTorServer;
use reqwest::Client;
use pingora::http::StatusCode;
use utils::bytes_to_json;
use crate::handler::common::consts::{HeaderKeys, BACKEND_HOST};
use crate::handler::common::types::ErrorResponse;
use crate::handler::proxy::{EncryptedMessage, WrappedBackendResponse, WrappedUserRequest};

/// Struct containing only associated methods (no instance methods or fields)
pub struct ProxyHandler {}

impl DefaultHandlerTrait for ProxyHandler {}

impl ProxyHandler {
    /// Validates the request headers for the nTor session ID.
    pub(crate) fn validate_request_headers(ctx: &mut Layer8Context) -> Result<String, APIHandlerResponse> {

        return match ctx.get_request_header().get(HeaderKeys::NTorSessionIDKey.as_str()) {
            None => {
                Err(APIHandlerResponse {
                    status: StatusCode::BAD_REQUEST,
                    body: Some(ErrorResponse {
                        error: "Missing nTor session ID header".to_string(),
                    }.to_bytes()),
                })
            }
            Some(session_id) => {
                if session_id.is_empty() {
                    return Err(APIHandlerResponse {
                        status: StatusCode::BAD_REQUEST,
                        body: Some(ErrorResponse {
                            error: "Empty nTor session ID header".to_string(),
                        }.to_bytes()),
                    })
                }
                info!("nTor session ID: {}", session_id);

                // todo validate session_id format

                Ok(session_id.to_string())
            }
        }
    }

    pub(crate) fn validate_request_body(ctx: &mut Layer8Context)
        -> Result<EncryptedMessage, APIHandlerResponse>
    {
        match ProxyHandler::parse_request_body::<
            EncryptedMessage,
            ErrorResponse
        >(&ctx.get_request_body()) {
            Ok(res) => Ok(res),
            Err(err) => {
                let body = match err {
                    None => None,
                    Some(err_response) => {
                        error!("Error parsing request body: {}", err_response.error);
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
    ) -> Result<WrappedUserRequest, APIHandlerResponse> {

        let mut ntor_server = NTorServer::new(ntor_server_id);
        ntor_server.set_shared_secret(shared_secret.clone());

        // Decrypt the request body using nTor shared secret
        let decrypted_data = ntor_server.decrypt(ntor::common::EncryptedMessage {
            nonce: <[u8; 12]>::try_from(request_body.nonce).unwrap(),
            data: request_body.data,
        }).map_err(|err| {
            return APIHandlerResponse {
                status: StatusCode::BAD_REQUEST,
                body: Some(format!("Decryption failed: {}", err).as_bytes().to_vec()),
            }
        })?;
        // let decrypted_data = request_body.data;

        // parse decrypted data into WrappedUserRequest
        let wrapped_request: WrappedUserRequest = bytes_to_json(decrypted_data)
            .map_err(|err| {
                return APIHandlerResponse {
                    status: StatusCode::BAD_REQUEST,
                    body: Some(format!("Failed to parse request body: {}", err).as_bytes().to_vec()),
                }
            })?;

        Ok(wrapped_request)
    }

    pub(crate) async fn rebuild_user_request(
        wrapped_request: WrappedUserRequest,
    ) -> Result<WrappedBackendResponse, APIHandlerResponse> {
        let mut header_map = HeaderMap::new();
        for (key, value) in wrapped_request.headers {
            header_map.insert(
                HeaderName::from_str(&key).expect("Failed to parse header key"),
                value.parse().expect("Failed to parse header value"),
            );
        }

      
        let body = utils::bytes_to_string(&wrapped_request.body.unwrap_or_default());
        debug!("[FORWARD {}] Reconstructed request body: {}", wrapped_request.uri, body);

        let url = format!("{}{}", BACKEND_HOST, wrapped_request.uri);
        debug!("[FORWARD {}] Request URL: {}", wrapped_request.uri, url);

        let client = Client::new();

        let mut req_builder = client
            .request(wrapped_request.method.parse().unwrap(), url.as_str())
            .headers(header_map.clone());

        if !body.is_empty() {
            req_builder = req_builder.body(body);
        }

        let response = req_builder.send().await;

        return match response {
            Ok(success_res) => {
                let status = success_res.status().as_u16();
                let status_text = success_res
                    .status()
                    .canonical_reason()
                    .unwrap_or("")
                    .to_string();

                let headers = success_res
                    .headers()
                    .iter()
                    .map(|(k, v)| {
                        let key = k.as_str().to_string();
                        let value = v.to_str().unwrap_or("").to_string();
                        (key, value)
                    })
                    .collect::<Vec<_>>();

                let serialized_headers = utils::headermap_to_string(&success_res.headers());
                let serialized_body: Vec<u8> = success_res.bytes().await.unwrap_or_default().to_vec();

                debug!("[FORWARD {}] Response from backend headers: {}", wrapped_request.uri, serialized_headers);
                debug!("[FORWARD {}] Response from backend body: {}", wrapped_request.uri, utils::bytes_to_string(&serialized_body));

                Ok(WrappedBackendResponse {
                    status,
                    status_text,
                    headers,
                    body: serialized_body,
                })
            }
            Err(err) => {
                error!("[FORWARD] Error while building request to BE: {:?}", err);
                let status = err.status().unwrap_or(reqwest::StatusCode::INTERNAL_SERVER_ERROR);
                let err_body = ErrorResponse {
                    error: format!("Backend error: {}", status),
                };

                Err(APIHandlerResponse {
                    status: StatusCode::BAD_GATEWAY,
                    body: Some(err_body.to_bytes()),
                })
            }
        }
    }

    pub(crate) fn encrypt_response_body(
        response_body: WrappedBackendResponse,
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
                body: Some(format!("Encryption failed: {}", err).as_bytes().to_vec()),
            }
        })?;
        // let encrypted_data = ntor::common::EncryptedMessage {
        //     nonce: [0; 12], // Placeholder, replace with actual nonce generation
        //     data,
        // };

        Ok(EncryptedMessage {
            nonce: encrypted_data.nonce.to_vec(),
            data: encrypted_data.data,
        })
    }
}
