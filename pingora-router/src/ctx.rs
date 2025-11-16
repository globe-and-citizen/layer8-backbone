use crate::utils::get_request_body;
use pingora::http::{Method, RequestHeader, StatusCode};
use pingora::proxy::Session;
use std::collections::HashMap;
use std::time::Instant;
use uuid;

/*
 *  Each type in this crate serves a specific purpose and may be updated as requirements evolve.
 */

/// `Layer8ContextRequestSummary` is expected to contain all request's metadata
#[derive(Debug, Clone, Default)]
pub struct Layer8ContextRequestSummary {
    pub method: Method,
    pub scheme: String,
    pub host: String,
    pub path: String,
    pub params: HashMap<String, String>,
}

impl Layer8ContextRequestSummary {
    pub(crate) fn from(session: &Session) -> Self {
        let method = session.req_header().method.clone();
        let scheme = session
            .req_header()
            .uri
            .scheme()
            .map(|s| s.to_string())
            .unwrap_or_else(|| "".to_string());
        let host = session
            .req_header()
            .uri
            .host()
            .map(|h| h.to_string())
            .unwrap_or_else(|| "".to_string());
        let path = session.req_header().uri.path().to_string();
        let query = session.req_header().uri.query();

        let mut params = HashMap::new();
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
            scheme,
            host,
            path,
            params,
        }
    }
}

/// `Layer8ContextRequest` is expected to contain all relevant request information
/// needed for processing and handler access
#[derive(Debug, Clone, Default)]
pub struct Layer8ContextRequest {
    pub summary: Layer8ContextRequestSummary,
    pub header: Layer8Header,
    body: Vec<u8>,
}

impl Layer8ContextRequest {
    pub fn get_client_base_url(&self) -> String {
        format!("{}://{}", self.summary.scheme, self.summary.host)
    }
}

/// `Layer8ContextResponse` is expected to store data to be returned to the client and
/// shared across handlers during request processing
#[derive(Debug, Clone, Default)]
pub struct Layer8ContextResponse {
    pub status: StatusCode,
    pub header: Layer8Header,
    body: Vec<u8>,
}

/// `Layer8Context` is the main context object passed to handlers during request processing.
///
/// It encapsulates:
/// - `request`: All relevant request information (method, path, headers, body, params).
/// - `response`: Data to be returned to the client and shared across handlers (headers, body).
/// - `memory`: Arbitrary key-value data for sharing state between handlers during processing.
///
/// This struct is designed to provide a unified interface for accessing and modifying
/// request and response data, as well as sharing state across middleware and handlers.
/// All fields are private and should be accessed or modified only through dedicated `get` and `set` methods.
#[derive(Debug, Clone)]
pub struct Layer8Context {
    /// `request`: contains all relevant request information needed for processing and handler access
    pub request: Layer8ContextRequest,
    /// `response`: stores data to be returned to the client and shared across handlers
    /// during request processing
    pub response: Layer8ContextResponse,
    /// `memory`: stores arbitrary key-value data that needs to be shared across handlers
    /// during request processing.
    /// Accessed via `get(&self, key: &str)` and `set(&mut self, key: String, value: String)` methods
    memory: HashMap<String, String>,
    pub latency_start: Instant, // todo: remove if not needed
}

impl Default for Layer8Context {
    fn default() -> Self {
        Self {
            request: Default::default(),
            response: Default::default(),
            memory: Default::default(),
            latency_start: Instant::now(),
        }
    }
}

impl Layer8Context {
    pub async fn update(&mut self, session: &mut Session) -> pingora::Result<bool> {
        self.request.summary = Layer8ContextRequestSummary::from(session);

        self.set_request_header(session.req_header().clone());

        // take anything as needed later

        Ok(true)
    }

    pub async fn read_request_body(&mut self, session: &mut Session) -> pingora::Result<bool> {
        match get_request_body(session).await {
            Ok(body) => self.request.body = body,
            Err(err) => return Err(err),
        };
        Ok(true)
    }
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
            self.request
                .header
                .insert(key.to_string(), val.to_str().unwrap_or("").to_string());
        }
    }

    fn get_request_header(&self) -> &Layer8Header {
        &self.request.header
    }

    fn insert_response_header(&mut self, key: &str, val: &str) {
        self.response
            .header
            .insert(key.to_lowercase().to_string(), val.to_string());
    }

    fn remove_response_header(&mut self, key: &str) -> Option<String> {
        self.response.header.remove(key)
    }

    fn get_response_header(&self) -> &Layer8Header {
        &self.response.header
    }

    fn set_request_body(&mut self, body: Vec<u8>) {
        self.request.body = body
    }

    fn extend_request_body(&mut self, body: Vec<u8>) {
        self.request.body.extend(body)
    }

    fn get_request_body(&self) -> Vec<u8> {
        self.request.body.clone()
    }

    fn set_response_body(&mut self, body: Vec<u8>) {
        self.response.body = body
    }

    fn extend_response_body(&mut self, body: Vec<u8>) {
        self.response.body.extend(body);
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

    fn set_correlation_id(&mut self) -> String {
        let correlation_id: String;
        if let Some(cid) = self.get_request_header().get("x-correlation-id") {
            correlation_id = cid.clone();
        } else if let Some(cid) = self.get_request_header().get("x-request-id") {
            correlation_id = cid.clone();
        } else {
            correlation_id = uuid::Uuid::new_v4().to_string();
        }

        self.set("x-correlation-id".to_string(), correlation_id.clone());
        correlation_id
    }

    fn get_correlation_id(&self) -> String {
        self.get("x-correlation-id")
            .unwrap_or(&"".to_string())
            .clone()
    }

    fn get_latency_ms(&self) -> i64 {
        self.latency_start.elapsed().as_nanos() as i64
    }
}

/// This trait appears to be redundant and could potentially be removed,
/// as it is only implemented for `Layer8Context` and not used elsewhere.
/// Considering...
pub trait Layer8ContextTrait {
    fn method(&self) -> Method;
    fn path(&self) -> String;
    fn params(&self) -> &HashMap<String, String>;
    fn param(&self, key: &str) -> Option<&String>;
    fn set_request_header(&mut self, header: RequestHeader);
    fn get_request_header(&self) -> &Layer8Header;
    fn insert_response_header(&mut self, key: &str, val: &str);
    fn remove_response_header(&mut self, key: &str) -> Option<String>;
    fn get_response_header(&self) -> &Layer8Header;
    fn set_request_body(&mut self, body: Vec<u8>);
    fn extend_request_body(&mut self, body: Vec<u8>);
    fn get_request_body(&self) -> Vec<u8>;
    fn set_response_body(&mut self, body: Vec<u8>);
    fn extend_response_body(&mut self, body: Vec<u8>);
    fn get_response_body(&self) -> Vec<u8>;
    fn get(&self, key: &str) -> Option<&String>;
    fn set(&mut self, key: String, value: String);
    fn set_request_summary(&mut self, summary: Layer8ContextRequestSummary);
    fn set_correlation_id(&mut self) -> String;
    fn get_correlation_id(&self) -> String;
    fn get_latency_ms(&self) -> i64;
}

/// `Layer8Header` is a type alias for a map of HTTP header key-value pairs used
/// throughout the proxy context.
///
/// - Keys and values are both `String`.
/// - Only string-representable header values are currently supported.
/// - This may need to be updated in the future to support non-string header values
/// (e.g., binary or multi-valued headers).
/// - Used for both request and response headers in `Layer8Context`.
pub type Layer8Header = HashMap<String, String>;
