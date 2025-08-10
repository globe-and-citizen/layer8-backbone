use serde::{Deserialize, Serialize};
use pingora_router::handler::ResponseBodyTrait;
use serde_json::Error;

#[derive(Serialize, Deserialize, Debug)]
pub struct ErrorResponse {
    pub(crate) error: String,
}

impl ResponseBodyTrait for ErrorResponse {
    fn from_json_err(err: Error) -> Option<Self> {
        Some(ErrorResponse {
            error: err.to_string()
        })
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct InitTunnelResponseFromRP { // this struct should match ReverseProxy's Response
    pub public_key: Vec<u8>,
    pub t_b_hash: Vec<u8>,
    #[serde(rename = "jwt1")] // a little bit of obfuscation
    pub int_rp_jwt: String,
    #[serde(rename = "jwt2")]
    pub fp_rp_jwt: String,
}

impl ResponseBodyTrait for InitTunnelResponseFromRP {}

#[derive(Serialize, Deserialize, Debug)]
pub struct InitTunnelResponseToINT { // this struct should match Interceptor's expected Response
    pub ephemeral_public_key: Vec<u8>,
    pub t_b_hash: Vec<u8>,
    #[serde(rename = "jwt1")]
    pub int_rp_jwt: String,
    #[serde(rename = "jwt2")]
    pub int_fp_jwt: String,
    #[serde(rename = "public_key")]
    pub ntor_static_public_key: Vec<u8>,
    #[serde(rename = "server_id")]
    pub ntor_server_id: String
}

impl ResponseBodyTrait for InitTunnelResponseToINT {}

#[derive(Serialize, Deserialize, Debug)]
pub struct FpHealthcheckSuccess {
    pub(crate) fp_healthcheck_success: String,
}

impl ResponseBodyTrait for FpHealthcheckSuccess {}

#[derive(Serialize, Deserialize, Debug)]
pub struct FpHealthcheckError {
    pub(crate) fp_healthcheck_error: String,
}

impl ResponseBodyTrait for FpHealthcheckError {}
