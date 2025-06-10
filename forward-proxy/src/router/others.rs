use std::collections::HashMap;
use pingora::http::StatusCode;
use crate::router::ctx::Layer8ContextTrait;

pub type Layer8Header = HashMap<String, String>; // Header value might not be able to be presented as String

pub type APIHandler<T> = fn(&T, &mut dyn Layer8ContextTrait) -> APIHandlerResponse;

#[derive(Debug, Default)]
pub struct APIHandlerResponse {
    pub status: StatusCode,
    pub body: Option<Vec<u8>>,
}

impl APIHandlerResponse {
    pub fn new(status: StatusCode, body: Option<Vec<u8>>) -> Self {
        APIHandlerResponse { status, body }
    }
}
