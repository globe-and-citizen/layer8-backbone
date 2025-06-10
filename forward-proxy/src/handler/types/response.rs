use serde::{Deserialize, Serialize};
use crate::handler::types::ResponseBodyTrait;

#[derive(Serialize, Deserialize, Debug)]
pub struct ErrorResponse {
    pub(crate) error: String,
}

impl ResponseBodyTrait for ErrorResponse {}

#[derive(Serialize, Deserialize, Debug)]
pub struct FpResponseBodyInit {
    pub fp_response_body_init: String,
}

impl ResponseBodyTrait for FpResponseBodyInit {}

#[derive(Serialize, Deserialize, Debug)]
pub struct FpResponseBodyProxied {
    pub fp_response_body_proxied: String,
}

impl ResponseBodyTrait for FpResponseBodyProxied {}

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
