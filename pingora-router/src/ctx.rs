use crate::utils;
use crate::utils::get_request_body;
use opentelemetry::global;
use opentelemetry::trace::TraceContextExt;
use pingora::http::{Method, RequestHeader, StatusCode};
use pingora::proxy::Session;
use std::collections::HashMap;
use tracing::error;
use tracing_opentelemetry::OpenTelemetrySpanExt;
use uuid;

/*
 *  Each type in this crate serves a specific purpose and may be updated as requirements evolve.
 */

#[derive(Debug, Default, Clone)]
pub struct Layer8ContextConfig {
    pub use_otel: bool,
    pub mtls_enabled: bool,
}

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
    request_span: tracing::Span,
    /// Measures the time taken to establish a connection to the upstream server.
    ///
    /// upstream_peer() --(start)--> TCP + TLS/mTLS --> connected_to_upstream() --(stop)-->
    /// upstream.connect.latency_ms
    ///
    /// This measures upstream connection establishment latency and is useful for
    /// detecting deployment and network differences.
    ///
    /// Includes:
    /// - Network latency
    /// - TCP connection establishment
    /// - TLS/mTLS handshake
    /// - Connection establishment overhead
    ///
    /// Note:
    /// This is not pure network latency because the measurement also includes
    /// TCP/TLS/mTLS handshake and processing overhead.
    upstream_connect_span: tracing::Span,
    /// Measures the time from when the upstream request is ready to be sent
    /// until the first response body data is received from the upstream server.
    ///
    /// upstream_request_filter() --(start)--> request sent --> upstream server
    /// --> response processing --> first response data --> upstream_response_filter() --(stop)-->
    /// upstream.ttfb.latency_ms
    ///
    /// This measures upstream Time To First Byte (TTFB) and is useful for
    /// detecting request/response latency differences between deployments.
    ///
    /// Includes:
    /// - Request transmission latency
    /// - Upstream server processing time
    /// - Response transmission latency until the first response data
    ///
    /// Note:
    /// This is not pure network latency because the measurement also includes
    /// upstream server processing and proxy/protocol overhead.
    upstream_ttfb_span: tracing::Span,
    /// Measures the time taken to receive the complete upstream response
    /// after the first response body data has been received.
    ///
    /// upstream_response_filter() --(start)--> response body chunks
    /// --> upstream_response_body_filter() --(end_of_stream)--> upstream.response.download.latency_ms
    ///
    /// This measures the duration of downloading the upstream response body
    /// and is useful for detecting response transfer and deployment/network
    /// differences.
    ///
    /// Includes:
    /// - Response transmission latency
    /// - Network latency while receiving the response
    /// - Upstream response streaming time
    /// - Network/proxy buffering and flow-control overhead
    ///
    /// Note:
    /// This is not pure network latency because the measurement can also include
    /// upstream server streaming behavior and proxy processing overhead.
    upstream_response_span: tracing::Span,
}

impl Default for Layer8Context {
    fn default() -> Self {
        Self {
            request: Default::default(),
            response: Default::default(),
            memory: Default::default(),
            request_span: tracing::Span::none(),
            upstream_connect_span: tracing::Span::none(),
            upstream_ttfb_span: tracing::Span::none(),
            upstream_response_span: tracing::Span::none(),
        }
    }
}

impl Layer8Context {
    pub async fn update(
        &mut self,
        session: &mut Session,
        config: Layer8ContextConfig,
    ) -> pingora::Result<bool> {
        self.request.summary = Layer8ContextRequestSummary::from(session);

        self.set_request_header(session.req_header().clone());

        let path = session.req_header().uri.path();
        let method = session.req_header().method.as_str();

        // Create a request lifecycle span with the HTTP method and path. This span
        // is used for tracing the full lifetime of the request inside the proxy and
        // is later attached to the context for downstream propagation.
        // This span is stored in the request context so it stays alive for the lifetime of the request.
        // It is dropped when the context is dropped or when it is explicitly released.
        self.request_span = tracing::info_span!(
            "request.lifecycle",
            otel.kind = "server", // Explicitly marks this as a Server entrypoint for OpenTelemetry / OTLP
            http.request.method = %method,
            http.request.path = %path,

            // Pre-declare dynamic response fields so span.record(...) works later
            http.response.status_code = tracing::field::Empty,
            error.type = tracing::field::Empty,
            otel.status_code = tracing::field::Empty,
            mtls.enabled = config.mtls_enabled,
            request.body.size = tracing::field::Empty,
            response.body.size = tracing::field::Empty,
        );

        // openTelemetry instrument
        if config.use_otel {
            // Extract tracing metadata from incoming request headers and make it the
            // parent of the current request span. This keeps the proxy request in the
            // same distributed trace as the upstream caller.
            let parent_context = global::get_text_map_propagator(|propagator| {
                propagator.extract(&utils::PingoraHeaderExtractor {
                    request: session.req_header(),
                })
            });

            if let Err(err) = self.request_span.set_parent(parent_context) {
                // Enter span so all logs emitted while processing this request inherit the same trace and span context.
                let _guard = self.request_span.enter();
                error!("telemetry: failed to set parent context: {:?}", err);
            }
        } else {
            self.set_correlation_id();
        }

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

    pub fn debug<F>(&self, f: F)
    where
        F: FnOnce(),
    {
        if tracing::enabled!(tracing::Level::DEBUG) {
            let _guard = self.request_span.enter();
            f();
        }
    }

    pub fn info<F>(&self, f: F)
    where
        F: FnOnce(),
    {
        let _guard = self.request_span.enter();
        f();
    }

    pub fn warn<F>(&self, f: F)
    where
        F: FnOnce(),
    {
        let _guard = self.request_span.enter();
        f();
    }

    pub fn error<F>(&self, f: F)
    where
        F: FnOnce(),
    {
        let _guard = self.request_span.enter();
        f();
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

    fn insert_request_header(&mut self, key: &str, val: &str) {
        self.request
            .header
            .insert(key.to_lowercase().to_string(), val.to_string());
    }

    fn remove_request_header(&mut self, key: &str) -> Option<String> {
        self.request.header.remove(&key.to_lowercase())
    }

    fn insert_response_header(&mut self, key: &str, val: &str) {
        self.response
            .header
            .insert(key.to_lowercase().to_string(), val.to_string());
    }

    fn remove_response_header(&mut self, key: &str) -> Option<String> {
        self.response.header.remove(&key.to_lowercase())
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

        self.set("l8-correlation-id".to_string(), correlation_id.clone());
        correlation_id
    }

    fn get_correlation_id(&self) -> String {
        self.get("l8-correlation-id")
            .unwrap_or(&"".to_string())
            .clone()
    }

    fn set_request_span(&mut self, span: tracing::Span) {
        self.request_span = span;
    }

    fn get_request_span(&self) -> &tracing::Span {
        &self.request_span
    }

    fn start_upstream_connect_span(&mut self) {
        self.upstream_connect_span =
            tracing::info_span!(parent: &self.request_span, "upstream.connection.establish");
    }

    fn get_upstream_connect_span(&self) -> &tracing::Span {
        &self.upstream_connect_span
    }

    fn end_upstream_connect_span(&mut self) {
        self.upstream_connect_span = tracing::Span::none()
    }
    fn start_upstream_ttfb_span(&mut self) {
        self.upstream_ttfb_span = tracing::info_span!(parent: &self.request_span, "upstream.ttfb");
    }

    fn get_upstream_ttfb_span(&self) -> &tracing::Span {
        &self.upstream_ttfb_span
    }

    fn end_upstream_ttfb_span(&mut self) {
        self.upstream_ttfb_span = tracing::Span::none()
    }

    fn start_upstream_response_span(&mut self) {
        self.upstream_response_span =
            tracing::info_span!(parent: &self.request_span, "upstream.response_body.download");
    }

    fn get_upstream_response_span(&self) -> &tracing::Span {
        &self.upstream_response_span
    }

    fn end_upstream_response_span(&mut self) {
        self.upstream_response_span = tracing::Span::none()
    }

    fn inject_otel_pingora_header(&mut self, span: &mut tracing::Span, header: &mut RequestHeader) {
        let otel_context = span.context();

        global::get_text_map_propagator(|propagator| {
            propagator.inject_context(
                &otel_context,
                &mut utils::PingoraHeaderInjector { request: header },
            );
        });
    }

    fn inject_otel_reqwest_headers(
        &mut self,
        span: &mut tracing::Span,
        header: &mut reqwest::header::HeaderMap,
    ) {
        let otel_context = span.context();
        opentelemetry::global::get_text_map_propagator(|prop| {
            prop.inject_context(
                &otel_context,
                &mut opentelemetry_http::HeaderInjector(header),
            );
        });
    }

    fn get_trace_id(&self) -> String {
        let cx = self.request_span.context();
        cx.span().span_context().trace_id().to_string()
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
    fn insert_request_header(&mut self, key: &str, val: &str);
    fn remove_request_header(&mut self, key: &str) -> Option<String>;
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
    fn set_request_span(&mut self, span: tracing::Span);
    fn get_request_span(&self) -> &tracing::Span;
    fn start_upstream_connect_span(&mut self);
    fn get_upstream_connect_span(&self) -> &tracing::Span;
    fn end_upstream_connect_span(&mut self);
    fn start_upstream_ttfb_span(&mut self);
    fn get_upstream_ttfb_span(&self) -> &tracing::Span;
    fn end_upstream_ttfb_span(&mut self);
    fn start_upstream_response_span(&mut self);
    fn get_upstream_response_span(&self) -> &tracing::Span;
    fn end_upstream_response_span(&mut self);
    fn inject_otel_pingora_header(&mut self, span: &mut tracing::Span, header: &mut RequestHeader);
    fn inject_otel_reqwest_headers(
        &mut self,
        span: &mut tracing::Span,
        req: &mut reqwest::header::HeaderMap,
    );
    /// This function isn't cheap, consider when using it
    fn get_trace_id(&self) -> String;
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
