use crate::ctx::Layer8Context;
use futures::future::BoxFuture;
use pingora::http::StatusCode;
use serde::de::Deserialize;
use serde::ser::Serialize;
use std::fmt::Debug;

/*
 *  Each type in this crate has a specific purpose and may be updated as requirements evolve.
 */

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
/// use std::sync::Arc;
/// use futures::FutureExt;
/// use pingora::http::StatusCode;
/// use pingora_router::ctx::Layer8Context;
/// use pingora_router::handler::{APIHandler, APIHandlerResponse};
///
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
pub type APIHandler<T> = Box<
    dyn for<'a> Fn(&'a T, &'a mut Layer8Context) -> BoxFuture<'a, APIHandlerResponse> + Send + Sync,
>;

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

/// `ResponseBodyTrait` provides a default method to serialize the response body to bytes.
///
/// *Implementation* for response body types is **optional** and provided mainly for
/// *convenience* at this stage.
///
/// But it *can be extended* to specify requirements as needed later.
pub trait ResponseBodyTrait: Serialize + for<'de> Deserialize<'de> + Debug {
    fn to_bytes(&self) -> Vec<u8> {
        serde_json::to_vec(self).unwrap()
    }

    fn from_bytes(bytes: &[u8]) -> Result<Box<Self>, serde_json::Error> {
        serde_json::from_slice(bytes)
    }

    /// Override this method to handle error serialization if your handler implements
    /// the `DefaultHandler` trait.
    fn from_json_err(_err: serde_json::Error) -> Option<Self> {
        None
    }
}

/// `RequestBodyTrait` provides a default method to deserialize the request body bytes
/// to json format.
///
/// *Implementation* for request body types is **optional** and provided mainly for
/// *convenience* at this stage.
///
/// But it *can be extended* to specify requirements as needed later.
pub trait RequestBodyTrait: Serialize + for<'de> Deserialize<'de> + Debug {
    fn to_bytes(&self) -> Vec<u8> {
        serde_json::to_vec(self).unwrap()
    }

    fn from_bytes(bytes: &[u8]) -> Result<Box<Self>, serde_json::Error> {
        serde_json::from_slice(bytes)
    }
}

/// `DefaultHandlerTrait` provides default request body parsing.
/// Can be extended to specify requirements for integration with the router or to provide
/// additional default methods in the future.
///
/// *Implementation* for your handler is **optional** and provided mainly for
/// *convenience* at this stage.
///
/// This trait defines a generic method `parse_request_body` that attempts to deserialize a
/// request body of type `T: impl RequestBodyTrait` from a byte vector.
/// If deserialization succeeds, it returns the parsed body, no error, and a 200 OK status.
/// If deserialization fails, it returns no body, an error response of type `E: impl
/// ResponseBodyTrait` (constructed from the JSON error), and a 400 Bad Request status.
pub trait DefaultHandlerTrait {
    fn parse_request_body<T: RequestBodyTrait, E: ResponseBodyTrait>(
        data: &[u8],
    ) -> Result<T, Option<E>> {
        match T::from_bytes(data) {
            Ok(body) => Ok(*body),
            Err(e) => Err(E::from_json_err(e)),
        }
    }
}
