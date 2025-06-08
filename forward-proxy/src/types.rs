use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct ResponseBody {
    data: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct FpRequestBodyInit {
    fp_request_body_init: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct FpResponseBodyInit {
    pub(crate) fp_response_body_init: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct FpRequestBodyProxied {
    fp_request_body_proxied: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct FpResponseBodyProxied {
    pub(crate) fp_response_body_proxied: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct FpHealthcheckSuccess {
    fp_healthcheck_success: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct FpHealthcheckError {
    fp_healthcheck_error: String,
}
