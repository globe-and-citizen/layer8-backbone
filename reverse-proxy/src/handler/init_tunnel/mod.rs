pub(crate) mod handler;

use serde::{Deserialize, Serialize};
use pingora_router::handler::{RequestBodyTrait, ResponseBodyTrait};

#[derive(Serialize, Deserialize, Debug)]
pub struct InitEncryptedTunnelRequest {
    pub public_key: Vec<u8>,
}

impl RequestBodyTrait for InitEncryptedTunnelRequest {}

#[derive(Serialize, Deserialize, Debug)]
pub struct InitTunnelRequestToBackend {
    pub success: bool,
}

impl RequestBodyTrait for InitTunnelRequestToBackend {}

#[derive(Serialize, Deserialize, Debug)]
pub struct InitEncryptedTunnelResponse {
    pub public_key: Vec<u8>,
    pub t_b_hash: Vec<u8>,

    #[serde(rename = "jwt1")]
    pub int_rp_jwt: String,

    #[serde(rename = "jwt2")]
    pub fp_rp_jwt: String
}

impl ResponseBodyTrait for InitEncryptedTunnelResponse {}
