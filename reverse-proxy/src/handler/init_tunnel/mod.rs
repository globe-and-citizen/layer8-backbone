pub mod handler;

use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard};
pub use self::handler::{InitTunnelHandler}; // Re-exporting the handler for easier access

use serde::{Deserialize, Serialize};
use pingora_router::handler::{RequestBodyTrait, ResponseBodyTrait};

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct InitEncryptedTunnelRequest {
    pub public_key: Vec<u8>,
}

impl RequestBodyTrait for InitEncryptedTunnelRequest {}

#[derive(Serialize, Deserialize, Debug)]
pub struct InitTunnelRequestToBackend {
    pub success: bool,
}

impl RequestBodyTrait for InitTunnelRequestToBackend {}

#[derive(Serialize, Deserialize, Debug)]
pub struct InitEncryptedTunnelResponse {
    pub public_key: Vec<u8>,
    pub t_b_hash: Vec<u8>,
    pub int_rp_jwt: String,
    pub fp_rp_jwt: String
}

impl ResponseBodyTrait for InitEncryptedTunnelResponse {}

thread_local! {
    // <session_id, shared_secret>
    static NTOR_SHARED_SECRETS: Mutex<HashMap<String, Vec<u8>>> = Mutex::new(HashMap::new());
}

pub struct InMemorySecretsStorage;

impl InMemorySecretsStorage {
    pub fn insert(session_id: String, shared_secret: Vec<u8>) {
        NTOR_SHARED_SECRETS.with(|memory| {
            let mut guard: MutexGuard<HashMap<String, Vec<u8>>> = memory.lock().unwrap();
            guard.insert(session_id, shared_secret);
        });
    }

    pub fn get(session_id: String) -> Option<Vec<u8>> {
        NTOR_SHARED_SECRETS.with(|memory| {
            let guard = memory.lock().unwrap();
            guard.get(&session_id).cloned()
        })
    }
}
