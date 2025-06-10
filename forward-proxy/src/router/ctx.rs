use std::collections::HashMap;
use pingora::http::{Method, RequestHeader};
use pingora::proxy::Session;
use crate::router::others::Layer8Header;

#[derive(Debug, Clone, Default)]
pub struct Layer8ContextRequestSummary {
    pub method: Method,
    pub path: String,
    pub params: HashMap<String, String>,
}

impl Layer8ContextRequestSummary {
    pub(crate) fn from(session: &Session) -> Self {
        let method = session.req_header().method.clone();
        let path = session.req_header().uri.path().to_string();
        let query = session.req_header().uri.query();

        let mut params = std::collections::HashMap::new();
        if let Some(query) = query {
            for pair in query.split('&') {
                let mut iter = pair.splitn(2, '=');
                if let (Some(key), Some(value)) = (iter.next(), iter.next()) {
                    params.insert(key.to_string(), value.to_string());
                }
            }
        }

        Layer8ContextRequestSummary {
            method,
            path,
            params,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Layer8ContextRequest {
    summary: Layer8ContextRequestSummary,
    header: Layer8Header,
    body: Vec<u8>,
}

#[derive(Debug, Clone, Default)]
pub struct Layer8ContextResponse {
    header: Layer8Header,
    body: Vec<u8>,
}

#[derive(Debug, Clone, Default)]
pub struct Layer8Context {
    request: Layer8ContextRequest,
    response: Layer8ContextResponse,
    memory: HashMap<String, String>,
}

impl Layer8ContextTrait for Layer8Context {
    fn method(&self) -> Method {
        self.request.summary.method.clone()
    }
    fn path(&self) -> String {
        self.request.summary.path.clone()
    }

    fn params(&self) -> &HashMap<String, String> {
        &self.request.summary.params
    }

    fn param(&self, key: &str) -> Option<&String> {
        self.request.summary.params.get(key)
    }

    fn set_request_header(&mut self, header: RequestHeader) {
        for (key, val) in header.headers.iter() {
            self.request.header.insert(key.to_string(), val.to_str().unwrap_or("").to_string());
        };
    }

    fn get_request_header(&self) -> &Layer8Header {
        &self.request.header
    }

    fn insert_response_header(&mut self, key: &str, val: &str) {
        self.response.header.insert(key.to_string(), val.to_string());
    }

    fn get_response_header(&self) -> &Layer8Header {
        &self.response.header
    }

    fn set_request_body(&mut self, body: Vec<u8>) {
        self.request.body = body
    }

    fn get_request_body(&self) -> Vec<u8> {
        self.request.body.clone()
    }

    fn set_response_body(&mut self, body: Vec<u8>) {
        self.response.body = body
    }

    fn get_response_body(&self) -> Vec<u8> {
        self.response.body.clone()
    }

    fn get(&self, key: &str) -> Option<&String> {
        self.memory.get(key)
    }

    fn set(&mut self, key: String, value: String) {
        self.memory.insert(key, value);
    }

    fn set_request_summary(&mut self, summary: Layer8ContextRequestSummary) {
        self.request.summary = summary
    }
}

pub(crate) trait Layer8ContextTrait {
    fn method(&self) -> Method;
    fn path(&self) -> String;
    fn params(&self) -> &HashMap<String, String>;
    fn param(&self, key: &str) -> Option<&String>;
    fn set_request_header(&mut self, header: RequestHeader);
    fn get_request_header(&self) -> &Layer8Header;
    fn insert_response_header(&mut self, key: &str, val: &str);
    fn get_response_header(&self) -> &Layer8Header;
    fn set_request_body(&mut self, body: Vec<u8>);
    fn get_request_body(&self) -> Vec<u8>;
    fn set_response_body(&mut self, body: Vec<u8>);
    fn get_response_body(&self) -> Vec<u8>;
    fn get(&self, key: &str) -> Option<&String>;
    fn set(&mut self, key: String, value: String);
    fn set_request_summary(&mut self, summary: Layer8ContextRequestSummary);
}