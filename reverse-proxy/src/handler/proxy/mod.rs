pub(crate) mod handler;
use pingora_router::handler::{RequestBodyTrait, ResponseBodyTrait};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct EncryptedMessage {
    pub nonce: Vec<u8>,
    pub data: Vec<u8>
}

impl RequestBodyTrait for EncryptedMessage {}
impl ResponseBodyTrait for EncryptedMessage {}

#[derive(Serialize, Deserialize, Debug)]
pub struct WrappedUserRequest {
    pub method: String,
    pub uri: String,
    pub headers: String,
    pub body: Vec<u8>
}
impl RequestBodyTrait for WrappedUserRequest {}

#[derive(Serialize, Deserialize, Debug)]
pub struct WrappedBackendResponse {
    pub status: u16,
    pub headers: String,
    pub body: Vec<u8>
}

impl ResponseBodyTrait for WrappedBackendResponse {}