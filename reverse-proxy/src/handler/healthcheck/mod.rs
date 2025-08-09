use serde::{Deserialize, Serialize};
use pingora_router::handler::ResponseBodyTrait;

#[derive(Serialize, Deserialize, Debug)]
pub struct RpHealthcheckSuccess {
    pub(crate) rp_healthcheck_success: String,
}

impl ResponseBodyTrait for RpHealthcheckSuccess {}

#[derive(Serialize, Deserialize, Debug)]
pub struct RpHealthcheckError {
    pub(crate) rp_healthcheck_error: String,
}

impl ResponseBodyTrait for RpHealthcheckError {}