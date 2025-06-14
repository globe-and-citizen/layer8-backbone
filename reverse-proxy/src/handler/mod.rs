use std::collections::HashMap;
use log::{debug, error};
use pingora::http::StatusCode;
use reqwest::{Client, Response};
use pingora_router::ctx::{Layer8Context, Layer8ContextTrait};
use pingora_router::handler::{APIHandlerResponse, DefaultHandlerTrait, ResponseBodyTrait};
use crate::handler::consts::PROXY_TO_BACKEND_PATH;
use crate::handler::consts::HeaderKeys::{RpHeaderResponseKey, SpaHeaderRequestKey};
use crate::handler::types::{ErrorResponse, InitEncryptedTunnelRequest, InitEncryptedTunnelResponse, ProxyRequest, ProxyResponseFromBackend};

pub mod types;
mod consts;
mod utils;

pub struct ReverseHandler {}

impl DefaultHandlerTrait for ReverseHandler {}

impl ReverseHandler {
    pub async fn handle_init_tunnel(&self, ctx: &mut Layer8Context) -> APIHandlerResponse {
        // validate request body
        let fp_request_bytes = ctx.get_request_body();
        match ReverseHandler::parse_request_body::<InitEncryptedTunnelRequest, ErrorResponse>(&fp_request_bytes) {
            Ok(_) => {}
            Err(err) => {
                let body = match err {
                    None => None,
                    Some(err_response) => Some(err_response.to_bytes())
                };

                return APIHandlerResponse {
                    status: StatusCode::BAD_REQUEST,
                    body,
                }
            }
        }

        // todo validate request headers

        // todo send success/failure flag to backend

        // set ReverseProxy's response header
        ctx.insert_response_header(RpHeaderResponseKey.as_str(), RpHeaderResponseKey.placeholder_value());

        // ReverseProxy's response body
        let response = InitEncryptedTunnelResponse {
            rp_response_body: "body added in ReverseProxy".to_string(),
        };

        APIHandlerResponse {
            status: StatusCode::OK,
            body: Some(response.to_bytes()),
        }
    }

    /// - get spa_request_body from received request body
    /// - send spa body to backend with ReverseProxy header
    async fn send_proxy_to_backend(&self, headers: HashMap<String, String>, body: ProxyRequest)
        -> Result<Response, reqwest::Error>
    {
        let new_headers = utils::to_reqwest_header(headers);
        let new_body = body.to_backend_body();
        let new_body_string = String::from_utf8_lossy(&new_body.to_bytes()).to_string();
        debug!("proxy to backend body: {}", new_body_string);

        let client = Client::new();
        client.post(PROXY_TO_BACKEND_PATH.as_str())
            .header("Content-Type", "application/json")
            .headers(new_headers)
            .body(new_body_string)
            .send()
            .await
    }

    pub async fn handle_proxy_request(&self, ctx: &mut Layer8Context) -> APIHandlerResponse {
        // validate request body
        let body = ctx.get_request_body();
        let request_body = match ReverseHandler::parse_request_body::<ProxyRequest, ErrorResponse>(&body) {
            Ok(proxy_request) => proxy_request,
            Err(err) => {
                let err_body = match err {
                    None => None,
                    Some(err_response) => {
                        error!("Error parsing request body: {}", err_response.error);
                        Some(err_response.to_bytes())
                    }
                };

                return APIHandlerResponse {
                    status: StatusCode::BAD_REQUEST,
                    body: err_body,
                }
            }
        };
        debug!("/proxy request body: {:?}", request_body);

        // todo validate request headers
        let spa_request_header = match ctx.get_request_header().get(SpaHeaderRequestKey.as_str()) {
            None => "".to_string(),
            Some(spa) => spa.to_string()
        };

        let be_request_header = HashMap::from([
            (SpaHeaderRequestKey.as_str().to_string(), spa_request_header),
            (RpHeaderResponseKey.as_str().to_string(), RpHeaderResponseKey.placeholder_value().to_string())
        ]);

        return match self.send_proxy_to_backend(be_request_header, request_body).await {
            Ok(reqw_response) if reqw_response.status().is_success() => {
                debug!("req_response: {:?}", reqw_response);
                let be_response: ProxyResponseFromBackend = match reqw_response.json().await {
                    Ok(res) => res,
                    Err(err) => {
                        debug!("Parsing backend body error: {:?}", err);
                        return APIHandlerResponse {
                            status: StatusCode::INTERNAL_SERVER_ERROR,
                            body: None,
                        }
                    }
                };

                // create new response body from backend's response
                let proxy_response = be_response.to_proxy_response("body added in ReverseProxy".to_string());

                APIHandlerResponse {
                    status: StatusCode::OK,
                    body: Some(proxy_response.to_bytes()),
                }
            }
            Ok(res) => {
                // Handle 4xx/5xx errors
                let status = res.status();

                let error_body = match res.content_length() {
                    None => "internal-server-error".to_string(),
                    Some(_) => {
                        res.text().await.unwrap_or_else(|_e| "".to_string())
                    }
                };

                let response_bytes = ErrorResponse {
                    error: error_body
                }.to_bytes();

                APIHandlerResponse {
                    status: StatusCode::try_from(status.as_u16()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
                    body: Some(response_bytes),
                }
            }
            Err(err) => {
                error!("Error forwarding request to backend: {:?}", err);
                let status = err.status().unwrap_or(reqwest::StatusCode::INTERNAL_SERVER_ERROR);
                let err_body = ErrorResponse {
                    error: format!("Backend error: {}", status),
                };

                APIHandlerResponse {
                    status: StatusCode::BAD_GATEWAY,
                    body: Some(err_body.to_bytes()),
                }
            }
        }
    }
}