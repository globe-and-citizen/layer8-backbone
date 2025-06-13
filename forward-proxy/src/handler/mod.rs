use log::{error, info};
use pingora::http::StatusCode;
use reqwest::{Client};
use pingora_router::ctx::{Layer8Context, Layer8ContextTrait};
use pingora_router::handler::{APIHandlerResponse, DefaultHandlerTrait};
use crate::handler::types::response::{ErrorResponse, FpHealthcheckError, FpHealthcheckSuccess, InitEncryptedTunnelResponse, ProxyResponse};
use pingora_router::handler::ResponseBodyTrait;
use reqwest::header::HeaderMap;
use crate::handler::consts::ForwardHeaderKeys::{FpHeaderRequestKey, FpHeaderResponseKey};
use crate::handler::consts::{LAYER8_GET_CERTIFICATE_PATH, RP_INIT_ENCRYPTED_TUNNEL_PATH, RP_PROXY_PATH};
use crate::handler::types::request::{InitEncryptedTunnelRequest, ProxyRequest};

pub mod types;
mod utils;
mod consts;

pub struct ForwardHandler {
    // add later
}

impl DefaultHandlerTrait for ForwardHandler {}

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

    fn forward_headers(headers: HeaderMap, ctx: &mut Layer8Context) {
        for (key, val) in headers.iter() {
            if let (k, Ok(v)) = (key.to_string(), val.to_str()) {
                ctx.insert_response_header(k.as_str(), v);
            }
        }
    }

    pub async fn handle_init_encrypted_tunnel(&self, ctx: &mut Layer8Context) -> APIHandlerResponse {
        // validate request body
        let init_request_body_bytes = ctx.get_request_body();
        match ForwardHandler::parse_request_body::<
                InitEncryptedTunnelRequest,
                ErrorResponse
            >(&init_request_body_bytes) {
            Ok(_) => {}
            Err(err) => {
                let body = match err {
                    None => None,
                    Some(e) => Some(e.to_bytes())
                };

                return APIHandlerResponse {
                    status: StatusCode::BAD_REQUEST,
                    body
                }
            }
        }

        let backend_url = match ctx.param("backend_url") {
            Some(url) => url.clone(),
            None => "".to_string()
        };

        //todo uncomment below line and make use of public_key, for now - assuming we've got it done
        // let public_key = self.get_public_key(backend_url.to_string(), ctx).await;
        // println!("public_key: {:?}", public_key);

        // copy all origin header to new request
        let origin_headers = ctx.get_request_header().clone();
        let mut reqwest_header_map = utils::to_reqwest_header(origin_headers);

        // add forward proxy header `fp_request_header`
        reqwest_header_map.insert(
            FpHeaderRequestKey.as_str(),
            FpHeaderRequestKey.placeholder_value().parse().unwrap(),
        );

        // forward origin request body
        let body_string = String::from_utf8_lossy(&init_request_body_bytes).to_string();
        let client = Client::new();
        let response = client.post(RP_INIT_ENCRYPTED_TUNNEL_PATH.as_str())
            .headers(reqwest_header_map.into())
            .body(body_string.clone())
            .send()
            .await;

        return match response {
            Ok(res) if res.status().is_success() => {
                let headers = res.headers().clone();
                let rp_response_body = res.bytes().await.unwrap_or_default();

                // validate reverse proxy response format, is it necessary?
                match utils::bytes_to_json::<InitEncryptedTunnelResponse>(rp_response_body.to_vec()) {
                    Err(e) => {
                        error!("Error parsing RP response: {:?}", e);
                        APIHandlerResponse {
                            status: StatusCode::INTERNAL_SERVER_ERROR,
                            body: None,
                        }
                    }
                    _ => {
                        // forward ReverseProxy's headers
                        ForwardHandler::forward_headers(headers, ctx);

                        // add forward proxy response header `fp_response_header`
                        ctx.insert_response_header(
                            FpHeaderResponseKey.as_str(),
                            FpHeaderResponseKey.placeholder_value(),
                        );

                        APIHandlerResponse {
                            status: StatusCode::OK,
                            body: Some(rp_response_body.to_vec()), // forward reverse proxy's response
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
        // validate request body
        let origin_req_body_bytes = ctx.get_request_body();
        match ForwardHandler::parse_request_body::<ProxyRequest, ErrorResponse>(&origin_req_body_bytes) {
            Ok(_) => {}
            Err(err) => {
                let body = match err {
                    None => None,
                    Some(body) => Some(body.to_bytes())
                };

                return APIHandlerResponse {
                    status: StatusCode::BAD_REQUEST,
                    body,
                }
            }
        }

        // copy origin headers to new request
        let origin_headers = ctx.get_request_header().clone();
        let mut reqwest_header_map = utils::to_reqwest_header(origin_headers);

        // add forward proxy header `fp_request_header`
        reqwest_header_map.insert(
            FpHeaderRequestKey.as_str(),
            FpHeaderRequestKey.placeholder_value().parse().unwrap(),
        );

        let client = Client::new();
        let body_string = String::from_utf8_lossy(&origin_req_body_bytes).to_string();
        let response = client
            .post(RP_PROXY_PATH.as_str())
            .headers(reqwest_header_map)
            // .json(&request_body)
            .body(body_string.clone())
            .send()
            .await;

        match response {
            Ok(res) if res.status().is_success() => {
                let headers = res.headers().clone();
                let rp_response_bytes = res.bytes().await.unwrap_or_default();

                // validate reverse proxy's response body format, is it necessary?
                match utils::bytes_to_json::<ProxyResponse>(rp_response_bytes.to_vec()) {
                    Err(err) => {
                        error!("Reverse Proxy's response mismatch: {:}", err);
                        return APIHandlerResponse {
                            status: StatusCode::INTERNAL_SERVER_ERROR,
                            body: None
                        }
                    }
                    Ok(_) => {}
                }

                // forward ReverseProxy's headers
                ForwardHandler::forward_headers(headers, ctx);

                // add ForwardProxy's response header `fp_response_header`
                ctx.insert_response_header(FpHeaderResponseKey.as_str(), FpHeaderResponseKey.placeholder_value());

                return APIHandlerResponse {
                    status: StatusCode::OK,
                    body: Some(rp_response_bytes.to_vec()), // forward ReverseProxy's response
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