use serde::{Deserialize, Serialize};
use pingora_router::handler::RequestBodyTrait;

#[derive(Serialize, Deserialize, Debug)]
pub struct InitTunnelRequest {
    pub public_key: Vec<u8>,
}

impl RequestBodyTrait for InitTunnelRequest {}

#[derive(Serialize, Deserialize, Debug)]
pub struct ProxyRequest {
    pub int_request_body: String,
    pub spa_request_body: String,
}

impl RequestBodyTrait for ProxyRequest {}