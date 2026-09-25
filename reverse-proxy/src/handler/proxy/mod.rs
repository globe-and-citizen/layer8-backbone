pub mod handler;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, bincode::Encode, bincode::Decode)]
pub struct L8RequestObject {
    pub uri: String,
    pub method: String,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}
impl L8RequestObject {
    #[allow(dead_code)]
    pub fn to_bincode_bytes(&self) -> Result<Vec<u8>, bincode::error::EncodeError> {
        bincode::encode_to_vec(self, bincode::config::standard())
    }

    pub fn from_bincode_bytes(bytes: &[u8]) -> Result<Self, bincode::error::DecodeError> {
        let (obj, _len) = bincode::decode_from_slice(bytes, bincode::config::standard())?;
        Ok(obj)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, bincode::Encode, bincode::Decode)]
pub struct L8ResponseObject {
    pub status: u16,
    pub status_text: String,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
    pub ok: bool,
    pub url: String,
    pub redirected: bool,
    /* Other fields are ignored because reqwest does not support */
}

impl L8ResponseObject {
    pub fn to_bincode_bytes(&self) -> Result<Vec<u8>, bincode::error::EncodeError> {
        bincode::encode_to_vec(self, bincode::config::standard())
    }

    #[allow(dead_code)]
    pub fn from_bincode_bytes(bytes: &[u8]) -> Result<Self, bincode::error::DecodeError> {
        let (obj, _len) = bincode::decode_from_slice(bytes, bincode::config::standard())?;
        Ok(obj)
    }
}
