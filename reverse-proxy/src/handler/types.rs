use std::fmt::Debug;
use serde::{Deserialize, Serialize};
use serde_json::Error;
use pingora_router::handler::{RequestBodyTrait, ResponseBodyTrait};

#[derive(Serialize, Deserialize, Debug)]
pub struct RequestBody {
    pub data: String,
}

impl RequestBodyTrait for RequestBody {}

#[derive(Serialize, Deserialize, Debug)]
pub struct ResponseBody {
    pub rp_response_body_init_proxied: Option<String>,
    pub rp_request_body_proxied: Option<String>,
    pub fp_request_body_proxied: Option<String>,
    pub error: Option<String>,
}

impl ResponseBodyTrait for ResponseBody {
    fn from_json_err(err: Error) -> Self {
        ResponseBody {
            rp_response_body_init_proxied: None,
            rp_request_body_proxied: None,
            fp_request_body_proxied: None,
            error: Some(err.to_string()),
        }
    }
}