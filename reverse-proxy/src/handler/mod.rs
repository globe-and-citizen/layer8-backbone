use log::{debug, error, info};
use pingora::http::StatusCode;
use reqwest::{Client};
use pingora_router::ctx::{Layer8Context, Layer8ContextTrait};
use pingora_router::handler::{APIHandlerResponse, DefaultHandlerTrait, RequestBodyTrait, ResponseBodyTrait};
use reqwest::header::HeaderMap;
use crate::handler::consts::{INIT_TUNNEL_TO_BACKEND_PATH, PROXY_TO_BACKEND_PATH};
use crate::handler::consts::HeaderKeys::{RpHeaderRequestKey, RpHeaderResponseKey};
use crate::handler::types::{ErrorResponse, InitEncryptedTunnelRequest, InitEncryptedTunnelResponse, InitTunnelRequestToBackend, ProxyRequest, ProxyRequestToBackend, ProxyResponse, ProxyResponseFromBackend};

pub mod types;
mod consts;
mod utils;

pub struct ReverseHandler {}

impl DefaultHandlerTrait for ReverseHandler {}

impl ReverseHandler {
    /// Add response headers to `ctx` to respond to FP:
    /// - *Copy* Backend's response header in `headers` - *update* `Content-Length`
    /// - *Add* custom ReverseProxy's response headers `custom_header`
    fn create_response_headers(
        headers: HeaderMap,
        ctx: &mut Layer8Context,
        custom_header: &str,
        content_length: usize
    ) {
        for (key, val) in headers.iter() {
            if let (k, Ok(v)) = (key.to_string(), val.to_str()) {
                ctx.insert_response_header(k.as_str(), v);
            }
        }

        ctx.insert_response_header(
            RpHeaderResponseKey.as_str(),
            custom_header,
        );

        ctx.insert_response_header("Content-Length", &*content_length.to_string())
    }

    /// Create request header to send/forward to BE:
    /// - *Copy* origin request headers from ForwardProxy `ctx`
    /// - *Add* custom ReverseProxy's request headers `custom_header`
    /// - *Set* universal Content-Type and Content-Length
    fn create_forward_request_headers(
        ctx: &mut Layer8Context,
        custom_header: &str,
        content_length: usize
    ) -> HeaderMap {
        // copy all origin header to new request
        let origin_headers = ctx.get_request_header().clone();
        let mut reqwest_header = utils::to_reqwest_header(origin_headers);

        // add forward proxy header `fp_request_header`
        reqwest_header.insert(
            RpHeaderRequestKey.as_str(),
            custom_header.parse().unwrap(),
        );

        reqwest_header.insert("Content-Length", content_length.to_string().parse().unwrap());
        reqwest_header.insert("Content-Type", "application/json".parse().unwrap());

        reqwest_header
    }

    async fn init_tunnel_result_to_be(result: bool) {
        let body = InitTunnelRequestToBackend {
            success: result,
        };
        let log_meta = format!("[FORWARD {}]", INIT_TUNNEL_TO_BACKEND_PATH.as_str());
        info!("{log_meta} request to BE body: {:?}", body);

        let client = Client::new();
        match client.post(INIT_TUNNEL_TO_BACKEND_PATH.as_str())
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
        {
            Ok(res) => {
                info!("{log_meta} Response sending init-tunnel result to BE: {:?}", res)
            }
            Err(err) => {
                error!("{log_meta} Error sending init-tunnel result to BE: {:?}", err)
            }
        }
    }

    pub async fn handle_init_tunnel(&self, ctx: &mut Layer8Context) -> APIHandlerResponse {
        // validate request body
        let request_body = match ReverseHandler::parse_request_body::
            <InitEncryptedTunnelRequest, ErrorResponse>(&ctx.get_request_body())
        {
            Ok(res) => res.to_bytes(),
            Err(err) => {
                let body = match err {
                    None => None,
                    Some(err_response) => Some(err_response.to_bytes())
                };

                ReverseHandler::init_tunnel_result_to_be(false).await;

                return APIHandlerResponse {
                    status: StatusCode::BAD_REQUEST,
                    body,
                };
            }
        };
        debug!("[REQUEST /init-tunnel] Parsed body: {:?}", request_body);

        // todo validate request headers

        // set ReverseProxy's response header
        ctx.insert_response_header(RpHeaderResponseKey.as_str(), RpHeaderResponseKey.placeholder_value());

        // ReverseProxy's response body
        let response = InitEncryptedTunnelResponse {
            rp_response_body: "body added in ReverseProxy".to_string(),
        };

        ReverseHandler::init_tunnel_result_to_be(true).await;

        APIHandlerResponse {
            status: StatusCode::OK,
            body: Some(response.to_bytes()),
        }
    }

    /// - get spa_request_body from received request body
    /// - send spa body to backend with ReverseProxy header
    async fn proxy_request_to_backend(
        &self,
        ctx: &mut Layer8Context,
        headers: HeaderMap,
        body: ProxyRequestToBackend
    ) -> APIHandlerResponse {

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
                debug!("{log_meta} response from BE: {:?}", reqw_response);
                return match reqw_response.json::<ProxyResponseFromBackend>().await {
                    Ok(res) => {
                        // create new response body from backend's response
                        let proxy_response = ProxyResponse {
                            be_response_body: res.be_response_body,
                            rp_response_body: "body added in ReverseProxy".to_string(),
                        }.to_bytes();

                        ReverseHandler::create_response_headers(
                            headers,
                            ctx,
                            RpHeaderResponseKey.placeholder_value(),
                            proxy_response.len()
                        );

                        APIHandlerResponse {
                            status: StatusCode::OK,
                            body: Some(proxy_response),
                        }
                    },
                    Err(err) => {
                        error!("Parsing backend body error: {:?}", err);
                        APIHandlerResponse {
                            status: StatusCode::INTERNAL_SERVER_ERROR,
                            body: None,
                        }
                    }
                }
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
                };
            }
        };

        let new_body = ProxyRequestToBackend {
            spa_request_body: request_body.spa_request_body,
        };

        // todo validate request headers
        let new_header = ReverseHandler::create_forward_request_headers(ctx, RpHeaderRequestKey.placeholder_value(), new_body.to_bytes().len());

        self.proxy_request_to_backend(ctx, new_header, new_body).await
    }
}