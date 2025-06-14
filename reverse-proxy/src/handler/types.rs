use std::fmt::Debug;
use serde::{Deserialize, Serialize};
use pingora_router::handler::{RequestBodyTrait, ResponseBodyTrait};
use serde_json::Error;

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

impl ResponseBodyTrait for ErrorResponse {
    fn from_json_err(err: Error) -> Option<Self> {
        Some(ErrorResponse {
            error: err.to_string()
        })
    }
}