use pingora::prelude::Session;

pub(crate) async fn get_request_body(session: &mut Session) -> pingora::Result<Vec<u8>> {
    let mut body = Vec::new();
    loop {
        match session.read_request_body().await {
            Ok(option) => match option {
                Some(chunk) => body.extend_from_slice(&chunk),
                None => break,
            },
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

use std::net::IpAddr;

/// Attempts to determine the "true" client IP by checking known CDN/edge
/// headers in priority order, then falling back to the direct TCP peer.
///
/// SECURITY: none of these headers are trustworthy unless you know the
/// direct TCP peer (session.client_addr()) is actually that provider's edge
/// IP range. Otherwise a client can set any of these headers themselves.
/// See the trusted-peer wrapper further down.
pub fn get_client_ip(session: &Session) -> Option<IpAddr> {
    let headers = &session.req_header().headers;

    let parse_ip =
        |v: &str| -> Option<IpAddr> { v.trim().trim_matches('"').parse::<IpAddr>().ok() };

    // 1. Akamai
    if let Some(ip) = headers
        .get("True-Client-IP")
        .and_then(|v| v.to_str().ok())
        .and_then(parse_ip)
    {
        return Some(ip);
    }

    // 2. Fastly
    if let Some(ip) = headers
        .get("Fastly-Client-IP")
        .and_then(|v| v.to_str().ok())
        .and_then(parse_ip)
    {
        return Some(ip);
    }

    // 3. Cloudflare
    if let Some(ip) = headers
        .get("CF-Connecting-IP")
        .and_then(|v| v.to_str().ok())
        .and_then(parse_ip)
    {
        return Some(ip);
    }

    // 4. AWS CloudFront (newer header, may include a port: "1.2.3.4:54321" or "[::1]:54321")
    if let Some(raw) = headers
        .get("CloudFront-Viewer-Address")
        .and_then(|v| v.to_str().ok())
    {
        let cleaned = raw.trim().trim_matches('"');
        let ip_str = if let Some(rest) = cleaned.strip_prefix('[') {
            rest.split_once(']').map(|(ip, _)| ip).unwrap_or(rest)
        } else {
            cleaned
                .rsplit_once(':')
                .map(|(ip, _)| ip)
                .unwrap_or(cleaned)
        };
        // guard against IPv6 addresses that contain multiple colons but no brackets
        if let Ok(ip) = ip_str.parse::<IpAddr>() {
            return Some(ip);
        } else if let Ok(ip) = cleaned.parse::<IpAddr>() {
            return Some(ip);
        }
    }

    // 5. Azure Front Door
    if let Some(ip) = headers
        .get("X-Azure-ClientIP")
        .and_then(|v| v.to_str().ok())
        .and_then(parse_ip)
    {
        return Some(ip);
    }

    // 6. X-Real-IP (nginx and other common proxies)
    if let Some(ip) = headers
        .get("X-Real-IP")
        .and_then(|v| v.to_str().ok())
        .and_then(parse_ip)
    {
        return Some(ip);
    }

    // 7. X-Forwarded-For (generic; first entry = original client, in theory)
    if let Some(xff) = headers.get("X-Forwarded-For").and_then(|v| v.to_str().ok()) {
        if let Some(first) = xff.split(',').next() {
            if let Some(ip) = parse_ip(first) {
                return Some(ip);
            }
        }
    }

    // 8. Standard "Forwarded" header (RFC 7239): Forwarded: for=1.2.3.4;proto=https
    if let Some(fwd) = headers.get("Forwarded").and_then(|v| v.to_str().ok()) {
        for part in fwd.split(';') {
            let part = part.trim();
            if let Some(rest) = part.strip_prefix("for=") {
                let cleaned = rest.trim_matches('"');
                let ip_str = cleaned
                    .trim_start_matches('[')
                    .split(&[']', ':'][..])
                    .next()
                    .unwrap_or(cleaned);
                if let Some(ip) = parse_ip(ip_str) {
                    return Some(ip);
                }
            }
        }
    }

    // 9. Fall back to direct TCP peer
    session
        .client_addr()
        .and_then(|addr| addr.as_inet())
        .map(|socket_addr| socket_addr.ip())
}
