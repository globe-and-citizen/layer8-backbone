use serde::{Deserialize, Serialize};
use pingora_router::handler::RequestBodyTrait;

#[derive(Serialize, Deserialize, Debug)]
pub struct InitEncryptedTunnelRequest {
    pub int_request_body: String,
}

impl RequestBodyTrait for InitEncryptedTunnelRequest {}

#[derive(Serialize, Deserialize, Debug)]
pub struct ProxyRequest {
    pub int_request_body: String,
    pub spa_request_body: String,
}

impl RequestBodyTrait for ProxyRequest {}