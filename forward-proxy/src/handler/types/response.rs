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
pub struct InitEncryptedTunnelResponse { // this struct should match ReverseProxy's Response
    pub rp_response_body: String,
}

impl ResponseBodyTrait for InitEncryptedTunnelResponse {}

#[derive(Serialize, Deserialize, Debug)]
pub struct ProxyResponse {
    pub be_response_body: String,
    pub rp_response_body: String,
}

impl ResponseBodyTrait for ProxyResponse {}

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
