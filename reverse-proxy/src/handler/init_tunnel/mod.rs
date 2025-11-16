pub(crate) mod handler;

use pingora_router::handler::{RequestBodyTrait, ResponseBodyTrait};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct InitEncryptedTunnelRequest {
    pub public_key: Vec<u8>,
}

impl RequestBodyTrait for InitEncryptedTunnelRequest {}

#[derive(Serialize, Deserialize, Debug)]
pub struct InitEncryptedTunnelResponse {
    pub public_key: Vec<u8>,
    pub t_b_hash: Vec<u8>,

    #[serde(rename = "jwt1")]
    pub int_rp_jwt: String,

    #[serde(rename = "jwt2")]
    pub fp_rp_jwt: String,
}

impl ResponseBodyTrait for InitEncryptedTunnelResponse {}
