pub(crate) mod handler;

use pingora_router::handler::{RequestBodyTrait, ResponseBodyTrait};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct ProxyRequest {
    pub int_request_body: String,
    pub spa_request_body: String
}

impl RequestBodyTrait for ProxyRequest {}

#[derive(Serialize, Deserialize, Debug)]
pub struct ProxyRequestToBackend {
    pub spa_request_body: String
}

impl RequestBodyTrait for ProxyRequestToBackend {}

#[derive(Serialize, Deserialize, Debug)]
pub struct ProxyResponseFromBackend {
    pub be_response_body: String
}

impl ResponseBodyTrait for ProxyResponseFromBackend {}

#[derive(Serialize, Deserialize, Debug)]
pub struct ProxyResponse {
    pub be_response_body: String,
    pub rp_response_body: String
}

impl ResponseBodyTrait for ProxyResponse {}
