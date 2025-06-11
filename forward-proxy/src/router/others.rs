use std::collections::HashMap;
use pingora::http::StatusCode;
use crate::router::ctx::{Layer8Context};
use futures::future::BoxFuture;

/*
 *  Each type in this crate has a specific purpose and may be updated as requirements evolve.
 */

/// `Layer8Header` is a type alias for a map of HTTP header key-value pairs used
/// throughout the proxy context.
///
/// - Keys and values are both `String`.
/// - Only string-representable header values are currently supported.
/// - This may need to be updated in the future to support non-string header values
/// (e.g., binary or multi-valued headers).
/// - Used for both request and response headers in `Layer8Context`.
pub type Layer8Header = HashMap<String, String>;

/// `APIHandler` is a type alias for an asynchronous handler function used in the proxy's routing system.
///
/// - It represents a boxed function that takes a reference to a handler object (`T`)
///   and a mutable reference to a `Layer8Context`, returning a boxed future that resolves to an `APIHandlerResponse`.
/// - The handler function must be `Send` and `Sync` to support concurrent execution.
/// - This abstraction allows for flexible, composable, and type-safe API endpoint handlers.
///
/// # Type Parameters
/// - `T`: The handler type, typically containing shared state or logic for processing requests.
///
/// # Example
/// ```rust
/// struct MyHandler;
///
/// impl MyHandler {
///     async fn handle(&self, ctx: &mut Layer8Context) -> APIHandlerResponse {
///         // Example logic
///         APIHandlerResponse::new(StatusCode::OK, Some(b"Hello, world!".to_vec()))
///     }
/// }
///
/// ...
/// let handler: APIHandler<Arc<MyHandler>> = Box::new(|h, ctx| {
///     async move { h.handle(ctx).await }.boxed()
/// });
/// ```
pub type APIHandler<T> = Box<dyn for<'a> Fn(&'a T, &'a mut Layer8Context) -> BoxFuture<'a, APIHandlerResponse> + Send + Sync>;

/// `APIHandlerResponse` contains information returned by handlers and can be
/// shared across handlers during request processing.
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
