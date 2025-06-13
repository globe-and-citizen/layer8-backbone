use log::{error, info};
use pingora::http::StatusCode;
use reqwest::Client;
use pingora_router::ctx::{Layer8Context, Layer8ContextTrait};
use pingora_router::handler::{APIHandlerResponse};
use crate::handler::types::response::{ErrorResponse, FpHealthcheckError, FpHealthcheckSuccess, InitEncryptedTunnelResponse, FpResponseBodyProxied};
use pingora_router::handler::ResponseBodyTrait;
use crate::handler::consts::ForwardHeaderKeys::{FpHeaderRequestKey, FpHeaderResponseKey};
use crate::handler::consts::{LAYER8_GET_CERTIFICATE_PATH, RP_INIT_ENCRYPTED_TUNNEL_PATH, RP_PROXY_PATH};

pub mod types;
mod utils;
mod consts;

pub struct ForwardHandler {
    // add later
}

type NTorPublicKey = Vec<u8>;

impl ForwardHandler {
    async fn get_public_key(&self, backend_url: String, ctx: &mut Layer8Context) -> Result<NTorPublicKey, APIHandlerResponse> {
        let secret_key = utils::get_secret_key();
        let token = utils::generate_standard_token(&secret_key).unwrap();
        let client = Client::new();

        return match client
            .get(format!("{}{}", LAYER8_GET_CERTIFICATE_PATH.as_str(), backend_url))
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
        {
            Ok(res) => {
                if !res.status().is_success() {
                    let response_body = ErrorResponse {
                        error: format!("Failed to get public key from layer8, status code: {}", res.status().as_u16()),
                    };
                    info!("Sending error response: {:?}", response_body);

                    ctx.insert_response_header("Connection", "close"); // Ensure connection closes???

                    Err(APIHandlerResponse {
                        status: StatusCode::BAD_REQUEST,
                        body: Some(response_body.to_bytes()),
                    })
                } else {
                    // todo extract public key from response
                    Ok(vec![])
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

    pub async fn handle_init_encrypted_tunnel(&self, ctx: &mut Layer8Context) -> APIHandlerResponse {
        let backend_url = match ctx.param("backend_url") {
            Some(url) => url.clone(),
            None => "".to_string()
        };

        let _public_key = self.get_public_key(backend_url.to_string(), ctx).await;
        // todo use public key

        let body_string = String::from_utf8_lossy(&ctx.get_request_body()).to_string();

        // copy all origin header to new request
        let origin_headers = ctx.get_request_header().clone();
        let mut reqwest_header_map = utils::to_reqwest_header(origin_headers);

        // add forward proxy header
        reqwest_header_map.insert(
            FpHeaderRequestKey.as_str(),
            FpHeaderRequestKey.placeholder_value().parse().unwrap(),
        );

        let client = Client::new();
        let response = client.post(RP_INIT_ENCRYPTED_TUNNEL_PATH.as_str())
            .headers(reqwest_header_map.into())
            .body(body_string.clone())
            .send()
            .await;

        return match response {
            Ok(res) if res.status().is_success() => {
                let rp_response_body = res.bytes().await.unwrap_or_default();

                match utils::bytes_to_json::<InitEncryptedTunnelResponse>(rp_response_body.to_vec()) {
                    Err(e) => {
                        error!("Error parsing RP response: {:?}", e);
                        APIHandlerResponse {
                            status: StatusCode::INTERNAL_SERVER_ERROR,
                            body: None,
                        }
                    }
                    _ => {
                        // add forward proxy response header
                        ctx.insert_response_header(
                            FpHeaderResponseKey.as_str(),
                            FpHeaderResponseKey.placeholder_value(),
                        );

                        APIHandlerResponse {
                            status: StatusCode::OK,
                            body: Some(rp_response_body.to_vec()),
                        }
                    }
                }
            }
            Ok(res) => {
                // Handle 4xx/5xx errors
                let status = res.status();
                let error_body = res.text().await.unwrap_or_default();

                let response_body_bytes = ErrorResponse {
                    error: error_body,
                }.to_bytes();

                APIHandlerResponse {
                    status: StatusCode::try_from(status.as_u16()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
                    body: Some(response_body_bytes),
                }
            }
            Err(e) => {
                error!("Failed to forward request to RP: {}", e);

                let response_body_bytes = ErrorResponse {
                    error: e.to_string(),
                }.to_bytes();

                APIHandlerResponse {
                    status: StatusCode::INTERNAL_SERVER_ERROR,
                    body: Some(response_body_bytes),
                }
            }
        };
    }

    pub async fn handle_proxy(&self, ctx: &mut Layer8Context) -> APIHandlerResponse {
        // Handle proxy endpoint
        let client = Client::new();

        let body_string = String::from_utf8_lossy(&ctx.get_request_body()).to_string();
        let response = client
            .post(RP_PROXY_PATH.as_str())
            .header(
                "x-fp-request-header-proxied",
                "request-header-forward-proxied",
            )
            // .json(&request_body)
            .body(body_string.clone())
            .send()
            .await;

        match response {
            Ok(res) if res.status().is_success() => {
                let headers = res.headers().clone();
                let response_body = res.text().await.unwrap_or_default();

                let response_body_bytes = FpResponseBodyProxied {
                    fp_response_body_proxied: response_body.clone(),
                }.to_bytes();

                ctx.insert_response_header("x-fp-response-header-proxied", "response-header-forward-proxied");

                if let Some(rp_header) = headers.get("x-rp-response-header-proxied") {
                    ctx.insert_response_header("x-rp-response-header-proxied", rp_header.to_str().unwrap_or(""));
                }

                return APIHandlerResponse {
                    status: StatusCode::OK,
                    body: Some(response_body_bytes),
                };
            }
            Ok(res) => {
                // Handle 4xx/5xx errors
                let status = res.status();
                let error_body = res.text().await.unwrap_or_default();

                let response_bytes = ErrorResponse {
                    error: error_body
                }.to_bytes();

                return APIHandlerResponse {
                    status: StatusCode::try_from(status.as_u16()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
                    body: Some(response_bytes),
                };
            }
            Err(e) => {
                error!("Failed to proxy request: {}", e);

                let response_bytes = ErrorResponse {
                    error: e.to_string()
                }.to_bytes();

                return APIHandlerResponse {
                    status: StatusCode::INTERNAL_SERVER_ERROR,
                    body: Some(response_bytes),
                };
            }
        }
    }

    pub async fn handle_healthcheck(&self, ctx: &mut Layer8Context) -> APIHandlerResponse {
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