pub(crate) mod handler;

use pingora_router::handler::{RequestBodyTrait, ResponseBodyTrait};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug)]
pub struct EncryptedMessage {
    pub nonce: Vec<u8>,
    pub data: Vec<u8>,
}

impl RequestBodyTrait for EncryptedMessage {}
impl ResponseBodyTrait for EncryptedMessage {}

#[derive(Serialize, Deserialize, Debug)]
pub struct L8RequestObject {
    pub method: String,
    pub uri: String,
    pub headers: HashMap<String, serde_json::Value>,
    pub body: Vec<u8>,
}
impl RequestBodyTrait for L8RequestObject {}

#[derive(Serialize, Deserialize, Debug)]
pub struct L8ResponseObject {
    pub status: u16,
    pub status_text: String,
    pub headers: HashMap<String, serde_json::Value>,
    pub body: Vec<u8>,
    pub ok: bool,
    pub url: String,
    pub redirected: bool,
    /* Other fields are ignored because reqwest does not support */
}

impl ResponseBodyTrait for L8ResponseObject {}
