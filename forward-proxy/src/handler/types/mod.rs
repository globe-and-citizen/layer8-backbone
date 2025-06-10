use std::fmt::Debug;
use serde::{Deserialize, Serialize};

pub mod response;
pub mod request;

pub trait ResponseBodyTrait: Serialize + for<'de> Deserialize<'de> + Debug {
    fn to_bytes(&self) -> Vec<u8> {
        serde_json::to_vec(self).unwrap()
    }
}

pub trait RequestBodyTrait: Serialize + for<'de> Deserialize<'de> + Debug {
    fn from_bytes(bytes: Vec<u8>) -> Result<Box<Self>, serde_json::Error> {
        serde_json::from_slice(&bytes)
    }
}
