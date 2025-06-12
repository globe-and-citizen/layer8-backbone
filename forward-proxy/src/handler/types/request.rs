use serde::{Deserialize, Serialize};
use pingora_router::handler::RequestBodyTrait;

#[derive(Serialize, Deserialize, Debug)]
struct FpRequestBodyInit {
    fp_request_body_init: String,
}

impl RequestBodyTrait for FpRequestBodyInit {}

#[derive(Serialize, Deserialize, Debug)]
struct FpRequestBodyProxied {
    fp_request_body_proxied: String,
}

impl RequestBodyTrait for FpRequestBodyProxied {}