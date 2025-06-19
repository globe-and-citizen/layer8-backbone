pub(crate) mod handler;

use serde::{Deserialize, Serialize};
use pingora_router::handler::{RequestBodyTrait, ResponseBodyTrait};

#[derive(Serialize, Deserialize, Debug)]
pub struct InitEncryptedTunnelRequest {
    pub int_request_body: String,
}

impl RequestBodyTrait for InitEncryptedTunnelRequest {}

#[derive(Serialize, Deserialize, Debug)]
pub struct InitTunnelRequestToBackend {
    pub success: bool,
}

impl RequestBodyTrait for InitTunnelRequestToBackend {}

#[derive(Serialize, Deserialize, Debug)]
pub struct InitEncryptedTunnelResponse {
    pub rp_response_body: String
}

impl ResponseBodyTrait for InitEncryptedTunnelResponse {}
