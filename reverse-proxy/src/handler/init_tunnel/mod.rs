pub mod handler;
use pingora_router::handler::{RequestBodyTrait, ResponseBodyTrait};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct InitEncryptedTunnelRequest {
    pub public_key: Vec<u8>,
}

impl RequestBodyTrait for InitEncryptedTunnelRequest {}

#[derive(Serialize, Deserialize, Debug)]
pub struct InitEncryptedTunnelResponse {
    pub public_key: Vec<u8>,
    pub t_b_hash: Vec<u8>,
    pub int_rp_jwt: String,
    pub fp_rp_jwt: String,
}

impl ResponseBodyTrait for InitEncryptedTunnelResponse {}

use dashmap::DashMap;
use secrecy::{ExposeSecret, SecretVec};
use std::{collections::HashMap, sync::LazyLock};

static NTOR_SHARED_SECRETS: LazyLock<DashMap<String, SecretVec<u8>>> = LazyLock::new(DashMap::new);

pub struct InMemorySecretsStorage;

impl InMemorySecretsStorage {
    #[allow(dead_code)]
    pub fn init(initial_data: HashMap<String, Vec<u8>>) {
        NTOR_SHARED_SECRETS.clear();

        for (session_id, secret) in initial_data {
            NTOR_SHARED_SECRETS.insert(session_id, SecretVec::new(secret));
        }
    }

    pub fn insert(session_id: String, shared_secret: Vec<u8>) {
        NTOR_SHARED_SECRETS.insert(session_id, SecretVec::new(shared_secret));
    }

    pub fn get(session_id: &str) -> Option<Vec<u8>> {
        NTOR_SHARED_SECRETS
            .get(session_id)
            .map(|s| s.value().expose_secret().to_vec())
    }
}
