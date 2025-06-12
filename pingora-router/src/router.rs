use std::collections::HashMap;
use pingora::http::{Method, StatusCode};
use crate::ctx::{Layer8Context, Layer8ContextTrait};
use crate::handler::{APIHandler, APIHandlerResponse};

/// `Router` is a generic struct that manages HTTP route registration and handler dispatching.
///
/// # Type Parameters
/// - `T`: The type of the main handler object, typically shared across all route handlers.
///
/// # Fields
/// - `handler`: The main handler instance shared with all route handlers.
/// - `_groups`: Reserved for future use (e.g., route grouping or middleware).
/// - `posts`, `gets`, `puts`, `deletes`: Maps of HTTP method and path to arrays of handler functions.
///
/// # Usage
/// Register handlers for specific HTTP methods and paths using the `post`, `get`, `put`, and `delete` methods.
/// Call `call_handler` to dispatch a request to the appropriate handler(s) based on method and path.
///
/// # Example
/// ```rust
/// let mut router = Router::new(handler);
/// router.post("/example".to_string(), Box::new([example_handler]));
/// ```
pub struct Router<T> {
    handler: T,
    _groups: Vec<String>, // placeholder for later use
    posts: HashMap<String, Box<[APIHandler<T>]>>,
    gets: HashMap<String, Box<[APIHandler<T>]>>,
    puts: HashMap<String, Box<[APIHandler<T>]>>,
    deletes: HashMap<String, Box<[APIHandler<T>]>>,
}

impl<T> Router<T> {
    pub fn new(handler: T) -> Self {
        Router {
            handler,
            _groups: Vec::new(),
            posts: HashMap::new(),
            gets: HashMap::new(),
            puts: HashMap::new(),
            deletes: HashMap::new(),
        }
    }

    /// Checks if the router contains a handler for the given HTTP method and path.
    ///
    /// # Arguments
    ///
    /// * `method` - The HTTP method to check (e.g., GET, POST).
    /// * `path` - The request path to check.
    ///
    /// # Returns
    ///
    /// `true` if a handler exists for the specified method and path, or if the method is OPTIONS;
    /// otherwise, `false`.
    pub fn contains(&self, method: &Method, path: &str) -> bool {
        match *method {
            Method::POST => self.posts.contains_key(path),
            Method::GET => self.gets.contains_key(path),
            Method::PUT => self.puts.contains_key(path),
            Method::DELETE => self.deletes.contains_key(path),
            Method::OPTIONS => true,
            _ => false,
        }
    }

    fn get_handlers(&self, method: &Method, path: &str) -> Option<&Box<[APIHandler<T>]>> {
        match *method {
            Method::POST => self.posts.get(path),
            Method::GET => self.gets.get(path),
            Method::PUT => self.puts.get(path),
            Method::DELETE => self.deletes.get(path),
            _ => return None,
        }
    }

    pub async fn call_handler(&self, ctx: &mut Layer8Context) -> APIHandlerResponse {
        let method = ctx.method();
        let path = ctx.path();

        if method == Method::OPTIONS {
            return APIHandlerResponse::new(StatusCode::NO_CONTENT, None);
        }

        if let Some(handlers) = self.get_handlers(&method, &path) {
            let mut response = APIHandlerResponse::new(StatusCode::OK, None);
            for handler in handlers.iter() {
                response = handler(&self.handler, ctx).await;
                if response.status != StatusCode::OK {
                    return response;
                }

                if response.body != None {
                    ctx.set_response_body(response.body.clone().unwrap());
                }
            }

            response
        } else {
            return APIHandlerResponse::new(StatusCode::NOT_FOUND, None);
        }
    }

    fn get_base_path(&self, path: &str) -> String {
        path.split('?').next().unwrap_or(path).to_string()
    }

    pub fn post(&mut self, path: String, handlers: Box<[APIHandler<T>]>) {
        self.posts.insert(self.get_base_path(&path), handlers);
    }

    pub fn get(&mut self, path: String, handlers: Box<[APIHandler<T>]>) {
        self.gets.insert(self.get_base_path(&path), handlers);
    }

    pub fn put(&mut self, path: String, handlers: Box<[APIHandler<T>]>) {
        self.puts.insert(self.get_base_path(&path), handlers);
    }

    pub fn delete(&mut self, path: String, handlers: Box<[APIHandler<T>]>) {
        self.deletes.insert(self.get_base_path(&path), handlers);
    }
}


