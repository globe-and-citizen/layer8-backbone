use std::collections::HashMap;
use pingora::http::StatusCode;
use crate::router::ctx::{Layer8Context};
use futures::future::BoxFuture;

pub type Layer8Header = HashMap<String, String>; // Header value might not be able to be presented as String

pub type APIHandler<T> = Box<dyn for<'a> Fn(&'a T, &'a mut Layer8Context) -> BoxFuture<'a, APIHandlerResponse> + Send + Sync>;

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
