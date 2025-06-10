use async_trait::async_trait;
use bytes::Bytes;
use pingora::prelude::{HttpPeer, ProxyHttp, Session};
use reqwest::Client;
use pingora::http;
use log::{error, info};
use pingora::http::{ResponseHeader, StatusCode};
use crate::handler::types::response::{ErrorResponse, FpHealthcheckError, FpHealthcheckSuccess, FpResponseBodyInit, FpResponseBodyProxied};
use crate::handler::types::ResponseBodyTrait;
use crate::utils;
use crate::router::Router;
use crate::router::ctx::{Layer8Context, Layer8ContextRequestSummary, Layer8ContextTrait};
use crate::router::others::{APIHandlerResponse};
use crate::utils::{get_request_body};

const LAYER8_URL: &str = "http://127.0.0.1:5001";
const RP_URL: &str = "http://127.0.0.1:6193";

// async fn handle_init_tunnel(session: &mut Session, body_string: String) -> pingora::Result<bool> {
// let query_params = session.req_header().uri.query();
// let params: Vec<&str> = query_params.unwrap().split("=").collect();
// let backend_url = params.get(1).unwrap();
async fn handle_init_tunnel(ctx: &mut Layer8Context) -> APIHandlerResponse {
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
            // let response_body = serde_json::json!({
            //             "error": format!("Failed to connect to layer8: {}", e)
            //         });
            //
            // let mut header = http::ResponseHeader::build(500, None)?;
            // header.insert_header("Content-Type", "application/json")?;
            //
            // // Single response with headers and body
            // session
            //     .write_response_header(Box::new(header), false)
            //     .await?;
            // session
            //     .write_response_body(
            //         Some(bytes::Bytes::from(response_body.to_string())),
            //         true,
            //     )
            //     .await?;
            // return Ok(true);

            let response_body = ErrorResponse {
                error: format!("Failed to connect to layer8: {}", e)
            };
            ctx.insert_response_header("Content-Type", "application/json");

            return APIHandlerResponse {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                body: Some(response_body.to_bytes()),
            };
        }
    };

    if !res.status().is_success() {
        // let response_body = serde_json::json!({
        //             "error": format!("Failed to get public key from layer8, status code: {}", res.status().as_u16())
        //         });
        // info!("Sending error response: {}", response_body);
        //
        // let mut header = http::ResponseHeader::build(
        //     res.status().as_u16().try_into().unwrap_or(400),
        //     None,
        // )?;
        // header.insert_header("Content-Type", "application/json")?;
        // header.insert_header("Connection", "close")?; // Ensure connection closes
        // header.insert_header("Content-Length", response_body.to_string().len())?;
        // // Single response with headers and body
        // session
        //     .write_response_header(Box::new(header), false)
        //     .await?;
        // session
        //     .write_response_body(Some(bytes::Bytes::from(response_body.to_string())), true)
        //     .await?;
        // return Ok(true);

        let response_body = ErrorResponse {
            error: format!("Failed to get public key from layer8, status code: {}", res.status().as_u16()),
        };
        info!("Sending error response: {:?}", response_body);
        let response_body_bytes = response_body.to_bytes();
        ctx.insert_response_header("Content-Type", "application/json");
        ctx.insert_response_header("Connection", "close"); // Ensure connection closes
        ctx.insert_response_header("Content-Length", &response_body_bytes.len().to_string()); // todo this header can be set later

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
            // let mut header = http::ResponseHeader::build(200, None)?;
            // header.insert_header(
            //     "x-fp-response-header-init",
            //     "response-header-forward-proxy-init",
            // )?;
            // header.insert_header("Content-Type", "application/json")?;
            // // header.insert_header("Content-Length", response_body.to_string().len())?;
            // header.insert_header("Content-Length", 2048)?;
            //
            // let response_json = FpResponseBodyInit {
            //     fp_response_body_init: response_body.clone(),
            // };
            //
            // session
            //     .write_response_header(Box::new(header), false)
            //     .await?;
            // session
            //     .write_response_body(
            //         Some(bytes::Bytes::from(
            //             serde_json::to_string(&response_json).unwrap(),
            //         )),
            //         true,
            //     )
            //     .await?;
            // return Ok(true);

            let response_json = FpResponseBodyInit {
                fp_response_body_init: response_body.clone(),
            };
            ctx.insert_response_header("x-fp-response-header-init", "response-header-forward-proxy-init");
            ctx.insert_response_header("Content-Type", "application/json");
            // header.insert_header("Content-Length", response_body.to_string().len())?;
            ctx.insert_response_header("Content-Length", &2048.to_string());

            return APIHandlerResponse {
                status: StatusCode::OK,
                body: Some(response_json.to_bytes()),
            };
        }
        Ok(res) => {
            // Handle 4xx/5xx errors
            let status = res.status();
            let error_body = res.text().await.unwrap_or_default();

            // let mut header = http::ResponseHeader::build(status.as_u16(), None)?;
            // header.insert_header("Content-Type", "application/json")?;
            // header.insert_header("Content-Length", error_body.to_string().len())?;
            //
            // session
            //     .write_response_header(Box::new(header), false)
            //     .await?;
            // session
            //     .write_response_body(Some(bytes::Bytes::from(error_body)), true)
            //     .await?;
            // return Ok(true);

            let response_body_bytes = ErrorResponse {
                error: error_body,
            }.to_bytes();
            ctx.insert_response_header("Content-Type", "application/json");
            ctx.insert_response_header("Content-Length", &response_body_bytes.len().to_string());

            return APIHandlerResponse {
                status: StatusCode::try_from(status.as_u16()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
                body: Some(response_body_bytes),
            };
        }
        Err(e) => {
            error!("Failed to forward request to RP: {}", e);
            // let mut header = http::ResponseHeader::build(500, None)?;
            // header.insert_header("Content-Type", "application/json")?;
            // header.insert_header("Content-Length", e.to_string().len())?;
            //
            // session
            //     .write_response_header(Box::new(header), false)
            //     .await?;
            // session
            //     .write_response_body(
            //         Some(bytes::Bytes::from(
            //             serde_json::json!({
            //                         "error": format!("Failed to forward request: {}", e)
            //                     })
            //                 .to_string(),
            //         )),
            //         true,
            //     )
            //     .await?;
            // return Ok(true);

            let response_body_bytes = ErrorResponse {
                error: e.to_string(),
            }.to_bytes();

            ctx.insert_response_header("Content-Type", "application/json");
            ctx.insert_response_header("Content-Length", &response_body_bytes.len().to_string());

            return APIHandlerResponse {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                body: Some(response_body_bytes),
            };
        }
    }
}

// async fn handle_proxy(session: &mut Session, body_string: String) -> pingora::Result<bool> {
async fn handle_proxy(ctx: &mut Layer8Context) -> APIHandlerResponse {
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

            // let mut header = http::ResponseHeader::build(200, None)?;
            // header.insert_header(
            //     "x-fp-response-header-proxied",
            //     "response-header-forward-proxied",
            // )?;
            // 
            // if let Some(rp_header) = headers.get("x-rp-response-header-proxied") {
            //     header.insert_header(
            //         "x-rp-response-header-proxied",
            //         rp_header.to_str().unwrap_or(""),
            //     )?;
            // }
            // 
            // header.insert_header("Content-Type", "application/json")?;
            // // header.insert_header("Content-Length", response_body.to_string().len())?;
            // header.insert_header("Content-Length", 2048)?;
            // 
            // let response_json = FpResponseBodyProxied {
            //     fp_response_body_proxied: response_body.clone(),
            // };
            // 
            // session
            //     .write_response_header(Box::new(header), false)
            //     .await?;
            // session
            //     .write_response_body(
            //         Some(bytes::Bytes::from(
            //             serde_json::to_string(&response_json).unwrap(),
            //         )),
            //         true,
            //     )
            //     .await?;
            // return Ok(true);

            let response_body_bytes = FpResponseBodyProxied {
                fp_response_body_proxied: response_body.clone(),
            }.to_bytes();
            ctx.insert_response_header("x-fp-response-header-proxied", "response-header-forward-proxied");

            if let Some(rp_header) = headers.get("x-rp-response-header-proxied") {
                ctx.insert_response_header("x-rp-response-header-proxied", rp_header.to_str().unwrap_or(""));
            }

            ctx.insert_response_header("Content-Type", "application/json");
            // header.insert_header("Content-Length", response_body.to_string().len())?;
            ctx.insert_response_header("Content-Length", &response_body_bytes.len().to_string());

            return APIHandlerResponse {
                status: StatusCode::OK,
                body: Some(response_body_bytes),
            };
        }
        Ok(res) => {
            // Handle 4xx/5xx errors
            let status = res.status();
            let error_body = res.text().await.unwrap_or_default();

            // let mut header = http::ResponseHeader::build(status.as_u16(), None)?;
            // header.insert_header("Content-Type", "application/json")?;
            // header.insert_header("Content-Length", error_body.to_string().len())?;
            //
            // session
            //     .write_response_header(Box::new(header), false)
            //     .await?;
            // session
            //     .write_response_body(Some(bytes::Bytes::from(error_body)), true)
            //     .await?;
            // return Ok(true);

            let response_bytes = ErrorResponse {
                error: error_body
            }.to_bytes();

            ctx.insert_response_header("Content-Type", "application/json");
            ctx.insert_response_header("Content-Length", &response_bytes.len().to_string());

            return APIHandlerResponse {
                status: StatusCode::try_from(status.as_u16()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
                body: Some(response_bytes),
            };
        }
        Err(e) => {
            error!("Failed to proxy request: {}", e);
            // let mut header = http::ResponseHeader::build(500, None)?;
            // header.insert_header("Content-Type", "application/json")?;
            // header.insert_header("Content-Length", e.to_string().len())?;
            //
            // session
            //     .write_response_header(Box::new(header), false)
            //     .await?;
            // session
            //     .write_response_body(
            //         Some(bytes::Bytes::from(
            //             serde_json::json!({
            //                         "error": format!("Failed to proxy request: {}", e)
            //                     })
            //                 .to_string(),
            //         )),
            //         true,
            //     )
            //     .await?;
            // return Ok(true);

            let response_bytes = ErrorResponse {
                error: e.to_string()
            }.to_bytes();

            ctx.insert_response_header("Content-Type", "application/json");
            ctx.insert_response_header("Content-Length", &response_bytes.len().to_string());

            return APIHandlerResponse {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                body: Some(response_bytes),
            };
        }
    }
}

// async fn handle_healthcheck(session: &mut Session) -> pingora::Result<bool> {
//     let query_params = session.req_header().uri.query();
//     let params: Vec<&str> = query_params.unwrap().split("=").collect();
//     let error = params.get(1).unwrap();
async fn handle_healthcheck(ctx: &mut Layer8Context) -> APIHandlerResponse {
    let error = ctx.param("error").unwrap();

    if error == "true" {
        // let mut header = http::ResponseHeader::build(418, None)?;
        // header.insert_header("Content-Type", "application/json")?;
        // header.insert_header("x-fp-healthcheck-error", "response-header-error")?;
        // let response_body = serde_json::json!({
        //             "fp_healthcheck_error": "this is placeholder for a custom error"
        //         });
        // header.insert_header("Content-Length", response_body.to_string().len())?;
        // session
        //     .write_response_header(Box::new(header), false)
        //     .await?;
        // session
        //     .write_response_body(Some(bytes::Bytes::from(response_body.to_string())), true)
        //     .await?;
        // return Ok(true);

        let response_bytes = FpHealthcheckError {
            fp_healthcheck_error: "this is placeholder for a custom error".to_string()
        }.to_bytes();

        ctx.insert_response_header("Content-Type", "application/json");
        ctx.insert_response_header("x-fp-healthcheck-error", "response-header-error");
        ctx.insert_response_header("Content-Length", &response_bytes.len().to_string());

        return APIHandlerResponse {
            status: StatusCode::IM_A_TEAPOT,
            body: Some(response_bytes),
        }
    }

    // let mut header = http::ResponseHeader::build(200, None)?;
    // header.insert_header("Content-Type", "application/json")?;
    // header.insert_header("x-fp-healthcheck-success", "response-header-success")?;
    // let response_body = serde_json::json!({
    //             "fp_healthcheck_success": "this is placeholder for a custom body"
    //         });
    // header.insert_header("Content-Length", response_body.to_string().len())?;
    // session
    //     .write_response_header(Box::new(header), false)
    //     .await?;
    // session
    //     .write_response_body(Some(bytes::Bytes::from(response_body.to_string())), true)
    //     .await?;
    // return Ok(true);

    let response_bytes = FpHealthcheckSuccess {
        fp_healthcheck_success: "this is placeholder for a custom body".to_string(),
    }.to_bytes();

    ctx.insert_response_header("Content-Type", "application/json");
    ctx.insert_response_header("x-fp-healthcheck-success", "response-header-success");
    ctx.insert_response_header("Content-Length", &response_bytes.len().to_string());

    return APIHandlerResponse {
        status: StatusCode::OK,
        body: Some(response_bytes),
    }
}

pub struct ForwardProxy<T> {
    router: Router<T>,
}

impl<T> ForwardProxy<T> {
    pub fn new(router: Router<T>) -> Self {
        ForwardProxy {
            router
        }
    }
}

#[async_trait]
impl<T: Sync> ProxyHttp for ForwardProxy<T> {
    type CTX = Layer8Context;

    fn new_ctx(&self) -> Self::CTX {
        Layer8Context::default()
    }

    async fn upstream_peer(
        &self,
        _session: &mut Session,
        _ctx: &mut Self::CTX,
    ) -> pingora::Result<Box<HttpPeer>> {
        Ok(Box::from(HttpPeer::new(
            String::from("127.0.0.1:6193"),
            false,
            String::from(""),
        )))
    }

    async fn request_filter(&self, session: &mut Session, ctx: &mut Self::CTX) -> pingora::Result<bool>
    where
        Self::CTX: Send + Sync,
    {
        // let request_url = session.req_header().uri.to_string();
        // let mut body = Vec::new();
        // loop {
        //     match session.read_request_body().await? {
        //         Some(chunk) => body.extend_from_slice(&chunk),
        //         None => break,
        //     }
        // }
        //
        // let body_string = String::from_utf8_lossy(&body).to_string();
        //
        // if request_url.contains("init-tunnel") {
        //     return handle_init_tunnel(session, body_string).await
        // } else if request_url.contains("proxy") {
        //     return handle_proxy(session, body_string).await
        // } else if request_url.contains("healthcheck") {
        //     return handle_healthcheck(session).await
        // }

        // create Context
        let request_summary = Layer8ContextRequestSummary::from(session);
        ctx.set_request_summary(request_summary);

        match get_request_body(session).await {
            Ok(body) => {
                ctx.set_request_body(body);
            }
            Err(err) => return Err(err)
        }

        let request_url = session.req_header().uri.to_string();
        let handler_response: APIHandlerResponse;

        if request_url.contains("init-tunnel") {
            handler_response = handle_init_tunnel(ctx).await
        } else if request_url.contains("proxy") {
            handler_response = handle_proxy(ctx).await
        } else if request_url.contains("healthcheck") {
            handler_response = handle_healthcheck(ctx).await
        } else {
            return Ok(false)
        }

        // set headers
        let mut header = ResponseHeader::build(handler_response.status, None)?;
        let response_header = ctx.get_response_header().clone();
        for (key, val) in response_header.iter() {
            header.insert_header(key.clone(), val.clone()).unwrap();
        };

        if let Some(body_bytes) = handler_response.body {
            header.insert_header("Content-length", &body_bytes.len().to_string()).unwrap();
            session.write_response_body(Some(Bytes::from(body_bytes)), true).await?;
        };

        session.write_response_header_ref(&header).await?;
        Ok(true)
    }

    async fn response_filter(
        &self,
        _session: &mut Session,
        upstream_response: &mut http::ResponseHeader,
        _ctx: &mut Self::CTX,
    ) -> pingora::Result<()> {
        upstream_response.insert_header("Access-Control-Allow-Origin", "*")?;
        upstream_response.insert_header("Access-Control-Allow-Methods", "GET, POST")?;
        upstream_response.insert_header("Access-Control-Allow-Headers", "Content-Type")?;
        Ok(())
    }
}
