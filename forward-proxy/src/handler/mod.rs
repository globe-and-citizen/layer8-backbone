use log::{debug, error, info};
use pingora::http::StatusCode;
use reqwest::{Client, Response};
use pingora_router::ctx::{Layer8Context, Layer8ContextTrait};
use pingora_router::handler::{APIHandlerResponse, DefaultHandlerTrait, RequestBodyTrait};
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
    async fn get_public_key(
        &self,
        backend_url: String,
        ctx: &mut Layer8Context
    ) -> Result<NTorPublicKey, APIHandlerResponse>
    {
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

    /// Add response headers to `ctx` to respond to Interceptor:
    /// - *Copy* ReverseProxy's response header in `headers`
    /// - *Add* custom ForwardProxy's response headers `custom_header`
    fn create_response_headers(
        headers: HeaderMap,
        ctx: &mut Layer8Context,
        custom_header: &str
    ) {
        for (key, val) in headers.iter() {
            if let (k, Ok(v)) = (key.to_string(), val.to_str()) {
                ctx.insert_response_header(k.as_str(), v);
            }
        }

        ctx.insert_response_header(
            FpHeaderResponseKey.as_str(),
            custom_header
        )
    }

    /// Create request header to send/forward to ReverseProxy:
    /// - *Copy* origin request headers from Interceptor `ctx`
    /// - *Add* custom ForwardProxy's request headers `custom_header`
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
            FpHeaderRequestKey.as_str(),
            custom_header.parse().unwrap(),
        );

        reqwest_header.insert("Content-Length", content_length.to_string().parse().unwrap());
        reqwest_header.insert("Content-Type", "application/json".parse().unwrap());

        reqwest_header
    }

    /// forward manipulated `init-encrypted-tunnel` request to ReverseProxy and handle success response
    async fn init_tunnel_forward_to_rp(
        ctx: &mut Layer8Context,
        headers: HeaderMap,
        body: Vec<u8>,
    ) -> APIHandlerResponse
    {
        let body_string = utils::bytes_to_string(&body);
        let log_meta = format!("[FORWARD {}]", RP_INIT_ENCRYPTED_TUNNEL_PATH.as_str());
        info!("{log_meta} request headers to RP: {:?}", headers);
        info!("{log_meta} request body to RP: {:?}", body_string);

        let client = Client::new();
        let response = client.post(RP_INIT_ENCRYPTED_TUNNEL_PATH.as_str())
            .headers(headers)
            .body(body_string)
            .send()
            .await;

        match response {
            Ok(res) if res.status().is_success() => {
                let headers = res.headers().clone();
                let rp_response_body = res.bytes().await.unwrap_or_default();
                info!("{log_meta} response headers from RP: {:?}", headers);
                info!("{log_meta} response body from RP: {}", utils::bytes_to_string(&rp_response_body.to_vec()));

                // validate reverse proxy response format, is it necessary?
                return match utils::bytes_to_json::<InitEncryptedTunnelResponse>(rp_response_body.to_vec()) {
                    Err(e) => {
                        error!("Error parsing RP response: {:?}", e);
                        APIHandlerResponse {
                            status: StatusCode::INTERNAL_SERVER_ERROR,
                            body: None,
                        }
                    }
                    _ => {
                        // forward ReverseProxy's headers
                        ForwardHandler::create_response_headers(headers, ctx, FpHeaderResponseKey.placeholder_value());

                        APIHandlerResponse {
                            status: StatusCode::OK,
                            body: Some(rp_response_body.to_vec()), // forward reverse proxy's response
                        }
                    }
                };
            }
            _ => {}
        };

        ForwardHandler::handle_failed_forward_response(log_meta, response).await
    }

    /// handle failed forward requests (to ReverseProxy)
    async fn handle_failed_forward_response(
        log_meta: String,
        response: Result<Response, reqwest::Error>
    ) -> APIHandlerResponse {
        match response {
            Ok(res) => {
                // Handle 4xx/5xx errors
                let status = res.status();
                error!("{log_meta} RP Response: {:?}", res);

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
        }
    }

    pub async fn handle_init_encrypted_tunnel(&self, ctx: &mut Layer8Context) -> APIHandlerResponse {
        // validate request body
        let received_body = match ForwardHandler::parse_request_body::<
            InitEncryptedTunnelRequest,
            ErrorResponse
        >(&ctx.get_request_body())
        {
            Ok(res) => res.to_bytes(),
            Err(err) => {
                let body = match err {
                    None => None,
                    Some(e) => Some(e.to_bytes())
                };

                return APIHandlerResponse {
                    status: StatusCode::BAD_REQUEST,
                    body,
                };
            }
        };

        // get public key to initialize encrypted tunnel
        let backend_url = match ctx.param("backend_url") {
            Some(url) => url.clone(),
            None => "".to_string()
        };

        //todo handle result_public_key
        let _result_public_key = self.get_public_key(backend_url.to_string(), ctx).await;
        // println!("public_key: {:?}", result_public_key);

        // copy origin headers and add ForwardProxy header
        let new_headers = ForwardHandler::create_forward_request_headers(
            ctx,
            FpHeaderRequestKey.placeholder_value(),
            received_body.len()
        );

        // forward origin request body
        ForwardHandler::init_tunnel_forward_to_rp(ctx, new_headers, received_body).await
    }

    /// forward manipulated `proxy` request to ReverseProxy and handle success response
    async fn proxy_forward_to_rp(
        ctx: &mut Layer8Context,
        headers: HeaderMap,
        body: Vec<u8>,
    ) -> APIHandlerResponse
    {
        let body_string = utils::bytes_to_string(&body);
        let log_meta = format!("[FORWARD {}]", RP_PROXY_PATH.as_str());
        debug!("{log_meta} request headers to RP: {:?}", headers);
        debug!("{log_meta} request body to RP: {}", body_string);

        let client = Client::new();
        let response = client
            .post(RP_PROXY_PATH.as_str())
            .headers(headers)
            .body(body_string)
            .send()
            .await;

        match response {
            Ok(res) if res.status().is_success() => {
                let headers = res.headers().clone();
                let rp_response_bytes = res.bytes().await.unwrap_or_default();
                debug!("{log_meta} response headers from RP: {:?}", headers);
                debug!("{log_meta} response body from RP: {}", utils::bytes_to_string(&rp_response_bytes.to_vec()));

                // validate reverse proxy's response body format, is it necessary?
                match utils::bytes_to_json::<ProxyResponse>(rp_response_bytes.to_vec()) {
                    Err(err) => {
                        error!("Reverse Proxy's response mismatch: {:}", err);
                        return APIHandlerResponse {
                            status: StatusCode::INTERNAL_SERVER_ERROR,
                            body: None,
                        };
                    }
                    Ok(_) => {}
                }

                // forward ReverseProxy's headers
                ForwardHandler::create_response_headers(headers, ctx, FpHeaderResponseKey.placeholder_value());

                return APIHandlerResponse {
                    status: StatusCode::OK,
                    body: Some(rp_response_bytes.to_vec()), // forward ReverseProxy's response
                };
            }
            _ => {}
        };

        ForwardHandler::handle_failed_forward_response(log_meta, response).await
    }

    pub async fn handle_proxy(&self, ctx: &mut Layer8Context) -> APIHandlerResponse {
        // validate request body
        let received_body = match ForwardHandler::
        parse_request_body::<ProxyRequest, ErrorResponse>(&ctx.get_request_body())
        {
            Ok(body) => body.to_bytes(),
            Err(err) => {
                let body = match err {
                    None => None,
                    Some(body) => Some(body.to_bytes())
                };

                return APIHandlerResponse {
                    status: StatusCode::BAD_REQUEST,
                    body,
                };
            }
        };

        let new_headers = ForwardHandler::create_forward_request_headers(
            ctx, FpHeaderRequestKey.placeholder_value(), received_body.len());

        // send new request to ReverseProxy
        ForwardHandler::proxy_forward_to_rp(ctx, new_headers, received_body).await
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