use std::collections::HashMap;
use log::{debug, error};
use pingora::http::StatusCode;
use reqwest::Client;
use pingora_router::ctx::{Layer8Context, Layer8ContextTrait};
use pingora_router::handler::{APIHandlerResponse, DefaultHandlerTrait, ResponseBodyTrait};
use crate::handler::types::{RequestBody, ResponseBody};
use crate::proxy::{BACKEND_PORT};

pub mod types;

pub struct ReverseHandler {}

impl DefaultHandlerTrait for ReverseHandler {}

impl ReverseHandler {
    pub async fn handle_init_tunnel(&self, ctx: &mut Layer8Context) -> APIHandlerResponse {
        let response = ResponseBody {
            rp_response_body_init_proxied: Some("response body init, reverse proxy".to_string()),
            rp_request_body_proxied: None,
            fp_request_body_proxied: None,
            error: None,
        };

        ctx.insert_response_header("x-rp-response-header-init", "response-header-init-reverse-proxy");

        APIHandlerResponse {
            status: StatusCode::OK,
            body: Some(response.to_bytes()),
        }
    }

    pub async fn handle_proxy_request(&self, ctx: &mut Layer8Context) -> APIHandlerResponse {
        let body = ctx.get_request_body();

        ctx.insert_response_header("x-rp-response-header-added", "response-header-forward-proxied");

        let request_body = match ReverseHandler::parse_request_body::<RequestBody, ResponseBody>(&body) {
            Ok(request_body) => request_body,
            Err(err) => {
                let body = match err {
                    None => None,
                    Some(mut err_response) => {
                        error!("Error parsing request body: {:?}", err_response.error);
                        err_response.error = Some("Invalid request body".to_string());
                        Some(err_response.to_bytes())
                    }
                };

                return APIHandlerResponse {
                    status: StatusCode::BAD_REQUEST,
                    body,
                };
            }
        };

        debug!("Request body: {:?}", request_body.data);
        let request_url = ctx.path();
        debug!("Creating a new request to http://localhost:{}{}", BACKEND_PORT, request_url);

        let client = Client::new();
        let mut map = HashMap::new();
        map.insert("fp_request_body_proxied", request_body.data);

        let res = client
            .post(format!("http://localhost:{}{}", BACKEND_PORT, request_url))
            .header("x-fp-request-header-proxied", "request-header-forward-proxied")
            .header("x-rp-request-header-proxied", "request-header-reverse-proxy")
            .json(&map)
            .send()
            .await;

        match res {
            Ok(response) => {
                debug!(
                            "POST {}, Host: localhost:{}, response code: {}",
                            request_url,
                            BACKEND_PORT,
                            response.status()
                        );

                let mut response_body: ResponseBody = response.json().await.unwrap();
                response_body.rp_request_body_proxied = Some("data copied by reverse proxy".to_string());

                APIHandlerResponse {
                    status: StatusCode::OK,
                    body: Some(response_body.to_bytes()),
                }
            }
            Err(err) => {
                error!("Error forwarding request to backend: {:?}", err);
                let status = err.status().unwrap_or(reqwest::StatusCode::INTERNAL_SERVER_ERROR);
                let response_body = ResponseBody {
                    rp_response_body_init_proxied: None,
                    rp_request_body_proxied: None,
                    fp_request_body_proxied: None,
                    error: Some(format!("Backend error: {}", status)),
                };

                APIHandlerResponse {
                    status: StatusCode::BAD_GATEWAY,
                    body: Some(response_body.to_bytes()),
                }
            }
        }
    }
}