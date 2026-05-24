use pingora_router::handler::RequestBodyTrait;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct InitTunnelRequest {
    pub public_key: Vec<u8>,
}

impl RequestBodyTrait for InitTunnelRequest {}
