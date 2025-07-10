use std::any::Any;
use pingora_router::ctx::{Layer8Context, Layer8ContextTrait};
use reqwest::header::HeaderMap;
use pingora_router::handler::{APIHandlerResponse, DefaultHandlerTrait, ResponseBodyTrait};
use log::{debug, error, info};
use ntor::common::NTorParty;
use ntor::server::NTorServer;
use reqwest::Client;
use pingora::http::StatusCode;
use utils::bytes_to_json;
use crate::handler::common::consts::{HeaderKeys, BACKEND_HOST};
use crate::handler::common::types::ErrorResponse;
use crate::handler::proxy::{EncryptedMessage, L8ResponseObject, L8RequestObject};

/// Struct containing only associated methods (no instance methods or fields)
pub struct ProxyHandler {}

impl DefaultHandlerTrait for ProxyHandler {}

impl ProxyHandler {
    /// Validates the request headers for the nTor session ID.
    pub(crate) fn validate_request_headers(
        ctx: &mut Layer8Context
    ) -> Result<String, APIHandlerResponse>
    {
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
                    });
                }
                info!("nTor session ID: {}", session_id);

                // todo validate session_id format

                Ok(session_id.to_string())
            }
        };
    }

    pub(crate) fn validate_request_body(
        ctx: &mut Layer8Context
    ) -> Result<EncryptedMessage, APIHandlerResponse>
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
        compression: Option<utils::compression::CompressorVariant>,
    ) -> Result<L8RequestObject, APIHandlerResponse> {
        let mut ntor_server = NTorServer::new(ntor_server_id);
        ntor_server.set_shared_secret(shared_secret.clone());

        // Decrypt the request body using nTor shared secret
        let mut decrypted_data = ntor_server
            .decrypt(ntor::common::EncryptedMessage {
                nonce: <[u8; 12]>::try_from(request_body.nonce).unwrap(),
                data: request_body.data,
            })
            .map_err(|err| {
                return APIHandlerResponse {
                    status: StatusCode::BAD_REQUEST,
                    body: Some(format!("Decryption failed: {}", err).as_bytes().to_vec()),
                };
            })?;

        // decompress the data if compression is specified
        if let Some(variant) = compression {
            log::warn!(
                "Size of decrypted data before decompression: {}",
                decrypted_data.len()
            );
            decrypted_data = utils::compression::decompress_data(&variant, &decrypted_data);
            log::warn!(
                "Size of decrypted data after decompression: {}",
                decrypted_data.len()
            );
        }

        // parse decrypted data into WrappedUserRequest
        let wrapped_request: L8RequestObject = bytes_to_json(decrypted_data).map_err(|err| {
            log::error!("Failed to parse request body: {}", err);
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
        wrapped_request: L8RequestObject
    ) -> Result<L8ResponseObject, APIHandlerResponse>
    {
        let header_map = utils::hashmap_to_headermap(&wrapped_request.headers)
            .unwrap_or_else(|_| HeaderMap::new());
        debug!("[FORWARD {}] Reconstructed request headers: {:?}", wrapped_request.uri, header_map);

        let origin_url = format!("{}{}", BACKEND_HOST, wrapped_request.uri);
        debug!("[FORWARD {}] Request URL: {}", wrapped_request.uri, origin_url);

        let client = Client::new();

        let response = client.request(
            wrapped_request.method.parse().unwrap(),
            origin_url.as_str(),
        )
            .headers(header_map.clone())
            .body(wrapped_request.body)
            .send()
            .await;

        return match response {
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

                debug!(
                    "[FORWARD {}] Response from backend headers: {:?}",
                    wrapped_request.uri,
                    serialized_headers
                );
                debug!(
                    "[FORWARD {}] Response from backend body: {}",
                    wrapped_request.uri,
                    utils::bytes_to_string(&serialized_body)
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
        };
    }

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
                body: Some(format!("Encryption failed: {}", err).as_bytes().to_vec()),
            };
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
