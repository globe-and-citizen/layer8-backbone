pub(crate) mod handler;

use std::collections::HashMap;
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
pub struct Layer8RequestObject {
    pub method: String,
    pub uri: String,
    pub headers: HashMap<String, serde_json::Value>,
    pub body: Vec<u8>
}
impl RequestBodyTrait for Layer8RequestObject {}

#[derive(Serialize, Deserialize, Debug)]
pub struct Layer8ResponseObject {
    pub status: u16,
    pub headers: HashMap<String, serde_json::Value>,
    pub body: Vec<u8>
}

impl ResponseBodyTrait for Layer8ResponseObject {}