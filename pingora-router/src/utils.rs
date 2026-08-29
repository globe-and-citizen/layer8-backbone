use pingora::prelude::Session;

pub(crate) async fn get_request_body(session: &mut Session) -> pingora::Result<Vec<u8>> {
    let mut body = Vec::new();
    loop {
        match session.read_request_body().await {
            Ok(option) => {
                match option {
                    Some(chunk) => body.extend_from_slice(&chunk),
                    None => break,
                }
            }
            Err(err) => {
                return Err(err);
            }
        }
    }
    Ok(body)
}

use opentelemetry::propagation::Injector;
use pingora::prelude::RequestHeader;

pub struct PingoraHeaderInjector<'a> {
    pub request: &'a mut RequestHeader,
}

impl<'a> Injector for PingoraHeaderInjector<'a> {
    fn set(&mut self, key: &str, value: String) {
        self.request
            .insert_header(key.to_owned(), value)
            .expect("failed to insert OTel header");
    }
}

pub struct PingoraHeaderExtractor<'a> {
    pub request: &'a pingora::http::RequestHeader,
}

impl opentelemetry::propagation::Extractor for PingoraHeaderExtractor<'_> {
    fn get(&self, key: &str) -> Option<&str> {
        self.request
            .headers
            .get(key)
            .and_then(|value| value.to_str().ok())
    }

    fn keys(&self) -> Vec<&str> {
        self.request
            .headers
            .keys()
            .map(|name| name.as_str())
            .collect()
    }
}