use log::{error, info};
use pingora::http::StatusCode;
use reqwest::Client;
use pingora_router::ctx::{Layer8Context, Layer8ContextTrait};
use pingora_router::handler::APIHandlerResponse;
use crate::handler::types::response::{ErrorResponse, FpHealthcheckError, FpHealthcheckSuccess, FpResponseBodyInit, FpResponseBodyProxied};
use pingora_router::handler::ResponseBodyTrait;

pub mod types;
mod utils;

const LAYER8_URL: &str = "http://127.0.0.1:5001";
const RP_URL: &str = "http://127.0.0.1:6193";

pub struct ForwardHandler {
    // add later
}

impl ForwardHandler {
    pub async fn handle_init_tunnel(&self, ctx: &mut Layer8Context) -> APIHandlerResponse {
        let params = ctx.params();
        let backend_url = params.get("backend_url").unwrap();
        let secret_key = utils::get_secret_key();
        let token = utils::generate_standard_token(&secret_key).unwrap();
        let client = Client::new();

        let res = match client
            .get(format!("{}{}?backend_url={}", LAYER8_URL, "/sp-pub-key", backend_url))
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
        {
            Ok(res) => res,
            Err(e) => {
                let response_body = ErrorResponse {
                    error: format!("Failed to connect to layer8: {}", e)
                };

                return APIHandlerResponse {
                    status: StatusCode::INTERNAL_SERVER_ERROR,
                    body: Some(response_body.to_bytes()),
                };
            }
        };

        if !res.status().is_success() {
            let response_body = ErrorResponse {
                error: format!("Failed to get public key from layer8, status code: {}", res.status().as_u16()),
            };
            info!("Sending error response: {:?}", response_body);
            let response_body_bytes = response_body.to_bytes();
            ctx.insert_response_header("Connection", "close"); // Ensure connection closes

            return APIHandlerResponse {
                status: StatusCode::BAD_REQUEST,
                body: Some(response_body_bytes),
            };
        }

        let body_string = String::from_utf8_lossy(&ctx.get_request_body()).to_string();
        let client = Client::new();
        let response = client
            .post(format!("{}/init-tunnel", RP_URL))
            .header(
                "x-fp-request-header-init",
                "request-header-forward-proxy-init",
            )
            .header("Content-Type", "application/json")
            .body(body_string.clone())
            .send()
            .await;

        match response {
            Ok(res) if res.status().is_success() => {
                let response_body = res.text().await.unwrap_or_default();

                let response_bytes = FpResponseBodyInit {
                    fp_response_body_init: response_body.clone(),
                }.to_bytes();
                ctx.insert_response_header("x-fp-response-header-init", "response-header-forward-proxy-init");

                return APIHandlerResponse {
                    status: StatusCode::OK,
                    body: Some(response_bytes),
                };
            }
            Ok(res) => {
                // Handle 4xx/5xx errors
                let status = res.status();
                let error_body = res.text().await.unwrap_or_default();

                let response_body_bytes = ErrorResponse {
                    error: error_body,
                }.to_bytes();

                return APIHandlerResponse {
                    status: StatusCode::try_from(status.as_u16()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
                    body: Some(response_body_bytes),
                };
            }
            Err(e) => {
                error!("Failed to forward request to RP: {}", e);

                let response_body_bytes = ErrorResponse {
                    error: e.to_string(),
                }.to_bytes();

                return APIHandlerResponse {
                    status: StatusCode::INTERNAL_SERVER_ERROR,
                    body: Some(response_body_bytes),
                };
            }
        }
    }

    pub async fn handle_proxy(&self, ctx: &mut Layer8Context) -> APIHandlerResponse {
        // Handle proxy endpoint
        let client = Client::new();
        // let request_body = FpRequestBodyProxied {
        //     fp_request_body_proxied: body_string.clone(),
        // };

        let body_string = String::from_utf8_lossy(&ctx.get_request_body()).to_string();
        let response = client
            .post(format!("{}/proxy", RP_URL))
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
            }
        }

        let response_bytes = FpHealthcheckSuccess {
            fp_healthcheck_success: "this is placeholder for a custom body".to_string(),
        }.to_bytes();

        ctx.insert_response_header("x-fp-healthcheck-success", "response-header-success");

        return APIHandlerResponse {
            status: StatusCode::OK,
            body: Some(response_bytes),
        }
    }
}