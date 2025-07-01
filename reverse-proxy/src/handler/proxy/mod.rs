pub(crate) mod handler;
use std::collections::HashMap;

use pingora_router::handler::{RequestBodyTrait, ResponseBodyTrait};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct EncryptedMessage {
    pub nonce: Vec<u8>,
    pub data: Vec<u8>,
}

impl RequestBodyTrait for EncryptedMessage {}
impl ResponseBodyTrait for EncryptedMessage {}

#[derive(Serialize, Deserialize, Debug)]
pub struct WrappedUserRequest {
    pub uri: String,
    pub method: String,
    pub headers: HashMap<String, String>,
    pub url_params: HashMap<String, String>,
    pub body: Option<Vec<u8>>,
}
impl RequestBodyTrait for WrappedUserRequest {}

#[derive(Serialize, Deserialize, Debug)]
pub struct WrappedBackendResponse {
    pub status: u16,
    pub status_text: String,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl ResponseBodyTrait for WrappedBackendResponse {}
