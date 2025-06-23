use std::fmt::Debug;
use serde::{Deserialize, Serialize};
use pingora_router::handler::ResponseBodyTrait;
use serde_json::Error;

#[derive(Serialize, Deserialize, Debug)]
pub struct ErrorResponse {
    pub error: String
}

impl ResponseBodyTrait for ErrorResponse {
    fn from_json_err(err: Error) -> Option<Self> {
        Some(ErrorResponse {
            error: err.to_string()
        })
    }
}