use pingora_router::ctx::{Layer8Context, Layer8ContextTrait};
use reqwest::header::HeaderMap;
use crate::handler::common::consts::HeaderKeys::{FpHeaderRequestKey, IntHeaderRequestKey, RpHeaderRequestKey, RpHeaderResponseKey};
use utils::{to_reqwest_header};

/// Struct containing only associated methods (no instance methods or fields).
/// The contents are quite drafting, but the idea is to handle common operations
pub struct CommonHandler {}

impl CommonHandler {
    /// Add response headers to `ctx` to respond to FP:
    /// - *Copy* Backend's response header in `headers` - *update* `Content-Length`
    /// - *Add* custom ReverseProxy's response headers `custom_header`
    pub fn create_response_headers(
        headers: HeaderMap,
        ctx: &mut Layer8Context,
        custom_header: &str,
        content_length: usize,
    ) {
        for (key, val) in headers.iter() {
            if let (k, Ok(v)) = (key.to_string(), val.to_str()) {
                ctx.insert_response_header(k.as_str(), v);
            }
        }

        ctx.insert_response_header(
            RpHeaderResponseKey.as_str(),
            custom_header,
        );

        ctx.insert_response_header("Content-Length", &*content_length.to_string())
    }

    /// Create request header to send/forward to BE:
    /// - *Copy* origin request headers from ForwardProxy `ctx`
    /// - *Add* custom ReverseProxy's request headers `custom_header`
    /// - *Set* universal Content-Type and Content-Length
    pub fn create_forward_request_headers(
        ctx: &mut Layer8Context,
        custom_header: &str,
        content_length: usize,
    ) -> HeaderMap {
        // copy all origin header to new request
        let origin_headers = ctx.get_request_header().clone();
        let mut reqwest_header = to_reqwest_header(origin_headers);

        // add forward proxy header `fp_request_header`
        reqwest_header.insert(
            RpHeaderRequestKey.as_str(),
            custom_header.parse().unwrap(),
        );

        reqwest_header.insert("Content-Length", content_length.to_string().parse().unwrap());
        reqwest_header.insert("Content-Type", "application/json".parse().unwrap());
        reqwest_header.remove(IntHeaderRequestKey.as_str());
        reqwest_header.remove(FpHeaderRequestKey.as_str());

        reqwest_header
    }
}