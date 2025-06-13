use std::fmt::Debug;
use serde::{Deserialize, Serialize};
use serde_json::Error;
use pingora_router::handler::{RequestBodyTrait, ResponseBodyTrait};

#[derive(Serialize, Deserialize, Debug)]
pub struct RequestBody {
    pub data: String,
}

impl RequestBodyTrait for RequestBody {}

#[derive(Serialize, Deserialize, Debug)]
pub struct InitEncryptedTunnelRequest {
    pub int_request_body: String,
    pub spa_request_body: String,
}

impl RequestBodyTrait for InitEncryptedTunnelRequest {}

#[derive(Serialize, Deserialize, Debug)]
pub struct InitEncryptedTunnelResponse {
    pub rp_response_body: String
}

impl ResponseBodyTrait for InitEncryptedTunnelResponse {}

#[derive(Serialize, Deserialize, Debug)]
pub struct ProxyRequest {
    pub int_request_body: String,
    pub spa_request_body: String
}

impl RequestBodyTrait for ProxyRequest {}

impl ProxyRequest {
    pub fn to_backend_body(&self) -> ProxyRequestToBackend {
        ProxyRequestToBackend {
            spa_request_body: self.spa_request_body.clone(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ProxyRequestToBackend {
    pub spa_request_body: String
}

impl RequestBodyTrait for ProxyRequestToBackend {}
impl ResponseBodyTrait for ProxyRequestToBackend {}

#[derive(Serialize, Deserialize, Debug)]
pub struct ProxyResponseFromBackend {
    pub be_response_body: String
}

impl RequestBodyTrait for ProxyResponseFromBackend {}
impl ResponseBodyTrait for ProxyResponseFromBackend {}

impl ProxyResponseFromBackend {
    pub fn to_proxy_response(&self, rp_response_body: String) -> ProxyResponse {
        ProxyResponse {
            be_response_body: self.be_response_body.clone(),
            rp_response_body,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ProxyResponse {
    pub be_response_body: String,
    pub rp_response_body: String
}

impl RequestBodyTrait for ProxyResponse {}
impl ResponseBodyTrait for ProxyResponse {}

#[derive(Serialize, Deserialize, Debug)]
pub struct ErrorResponse {
    pub error: String
}

impl ResponseBodyTrait for ErrorResponse {}

#[derive(Serialize, Deserialize, Debug)]
pub struct ResponseBody {
    pub rp_response_body_init_proxied: Option<String>,
    pub rp_request_body_proxied: Option<String>,
    pub fp_request_body_proxied: Option<String>,
    pub error: Option<String>,
}

impl ResponseBodyTrait for ResponseBody {
    fn from_json_err(err: Error) -> Option<ResponseBody> {
        Some(ResponseBody {
            rp_response_body_init_proxied: None,
            rp_request_body_proxied: None,
            fp_request_body_proxied: None,
            error: Some(err.to_string()),
        })
    }
}