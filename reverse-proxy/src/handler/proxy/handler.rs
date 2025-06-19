use pingora_router::ctx::Layer8Context;
use reqwest::header::HeaderMap;
use pingora_router::handler::{APIHandlerResponse, RequestBodyTrait, ResponseBodyTrait};
use log::{error, info};
use reqwest::Client;
use pingora::http::StatusCode;
use crate::handler::common::consts::HeaderKeys::RpHeaderResponseKey;
use crate::handler::common::consts::PROXY_TO_BACKEND_PATH;
use crate::handler::common::handler::CommonHandler;
use crate::handler::common::types::ErrorResponse;
use crate::handler::proxy::{ProxyRequestToBackend, ProxyResponse, ProxyResponseFromBackend};

pub struct ProxyHandler {}

impl ProxyHandler {
    /// - get spa_request_body from received request body
    /// - send spa body to backend with ReverseProxy header
    pub(crate) async fn proxy_request_to_backend(
        ctx: &mut Layer8Context,
        headers: HeaderMap,
        body: ProxyRequestToBackend,
    ) -> APIHandlerResponse
    {
        let new_body_string = String::from_utf8_lossy(&body.to_bytes()).to_string();

        let log_meta = format!("[FORWARD {}]", PROXY_TO_BACKEND_PATH.as_str());
        info!("{log_meta} request to BE headers: {:?}", headers);
        info!("{log_meta} request to BE body: {}", new_body_string);

        let client = Client::new();
        let response = client.post(PROXY_TO_BACKEND_PATH.as_str())
            .header("Content-Type", "application/json")
            .headers(headers)
            .body(new_body_string)
            .send()
            .await;

        match response {
            Ok(reqw_response) if reqw_response.status().is_success() => {
                let headers = reqw_response.headers().clone();
                info!("{log_meta} response from BE headers: {:?}", reqw_response.headers());
                return match reqw_response.json::<ProxyResponseFromBackend>().await {
                    Ok(res) => {
                        info!("{log_meta} response from BE body: {:?}", res);
                        // create new response body from backend's response
                        let proxy_response = ProxyResponse {
                            be_response_body: res.be_response_body,
                            rp_response_body: "body added in ReverseProxy".to_string(),
                        }.to_bytes();

                        CommonHandler::create_response_headers(
                            headers,
                            ctx,
                            RpHeaderResponseKey.placeholder_value(),
                            proxy_response.len(),
                        );

                        APIHandlerResponse {
                            status: StatusCode::OK,
                            body: Some(proxy_response),
                        }
                    }
                    Err(err) => {
                        error!("Parsing backend body error: {:?}", err);
                        APIHandlerResponse {
                            status: StatusCode::INTERNAL_SERVER_ERROR,
                            body: None,
                        }
                    }
                };
            }
            Ok(res) => {
                // Handle 4xx/5xx errors
                let status = res.status();
                error!("{log_meta} BE Response: {:?}", res);

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
                error!("{log_meta} Error: {:?}", err);
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
