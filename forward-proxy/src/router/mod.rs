pub mod ctx;
pub mod others;

use std::collections::HashMap;
use pingora::http::{Method, StatusCode};
use crate::router::ctx::{Layer8ContextTrait};
use crate::router::others::{APIHandler, APIHandlerResponse};


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

    pub fn call_handler(&self, ctx: &mut dyn Layer8ContextTrait) -> APIHandlerResponse {
        let method = ctx.method();
        let path = ctx.path();

        if method == Method::OPTIONS {
            return APIHandlerResponse::new(StatusCode::NO_CONTENT, None);
        }

        if let Some(handlers) = self.get_handlers(&method, &path) {
            let mut response = APIHandlerResponse::new(StatusCode::OK, None);
            for handler in handlers.iter() {
                response = handler(&self.handler, ctx);
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


