use std::fmt;
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

use std::net::{IpAddr, SocketAddr};

/// Returns the client IP addresses from the TCP peer address.
pub fn get_peer_ip(session: &Session) -> String {
    let ip = session
        .client_addr()
        .and_then(|addr| addr.as_inet())
        .map(|socket_addr| socket_addr.ip());

    ip.map(|ip| ip.to_string()).unwrap_or_default()
}

#[derive(Clone, PartialEq, Eq)]
pub struct ClientIP {
    pub source: String,
    pub ip: IpAddr,
}

impl fmt::Debug for ClientIP {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.source, self.ip)
    }
}

fn parse_ip(value: &str) -> Option<IpAddr> {
    let value = value.trim().trim_matches('"').trim();

    // Bare IPv4 or IPv6.
    if let Ok(ip) = value.parse::<IpAddr>() {
        return Some(ip);
    }

    // IPv4:port or [IPv6]:port.
    if let Ok(addr) = value.parse::<SocketAddr>() {
        return Some(addr.ip());
    }

    // Bracketed IPv6 without a port.
    if value.starts_with('[') && value.ends_with(']') {
        return value[1..value.len() - 1].parse::<IpAddr>().ok();
    }

    None
}

fn parse_forwarded_header(value: &str) -> Option<IpAddr> {
    for element in value.split(',') {
        for parameter in element.split(';') {
            let Some((key, value)) = parameter.trim().split_once('=') else {
                continue;
            };

            if !key.trim().eq_ignore_ascii_case("for") {
                continue;
            }

            let value = value.trim().trim_matches('"');

            if value.starts_with('_') || value.eq_ignore_ascii_case("unknown") {
                continue;
            }

            if let Some(ip) = parse_ip(value) {
                return Some(ip);
            }
        }
    }

    None
}

pub fn get_client_ips(session: &Session) -> Vec<ClientIP> {
    let headers = &session.req_header().headers;
    let mut ips = Vec::new();

    let single_ip_headers = [
        // Akamai CDN: identifies the original client IP.
        "True-Client-IP",

        // Fastly CDN: identifies the client IP observed by Fastly.
        "Fastly-Client-IP",

        // Cloudflare: identifies the connecting client IP reported by Cloudflare.
        "CF-Connecting-IP",

        // Cloudflare: carries the client's IPv6 address in supported configurations.
        "CF-Connecting-IPv6",

        // AWS CloudFront: viewer IP address, commonly including the source port.
        "CloudFront-Viewer-Address",

        // Microsoft Azure Front Door: reports the client IP.
        "X-Azure-ClientIP",

        // Fly.io: reports the connecting client IP.
        "Fly-Client-IP",

        // Nginx and other reverse proxies: commonly contains the client IP
        // forwarded by the proxy.
        "X-Real-IP",

        // Generic proxy header: may contain the client IP.
        // Its meaning depends on the proxy configuration.
        "X-Client-IP",

        // Generic proxy header: commonly identifies the original client IP
        // forwarded by an intermediary.
        "X-Forwarded-Client-IP",

        // Used by some proxy and cluster configurations to identify a client IP.
        "X-Cluster-Client-IP",

        // Envoy proxy: identifies the external client address as observed
        // or determined by Envoy.
        "X-Envoy-External-Address",

        // Google App Engine: identifies the user IP reported by the platform.
        "X-Appengine-User-IP",

        // Used by some proxies to preserve an earlier X-Forwarded-For value
        // before modifying or replacing the forwarding chain.
        "X-Original-Forwarded-For",
    ];

    for name in single_ip_headers {
        if let Some(ip) = headers
            .get(name)
            .and_then(|v| v.to_str().ok())
            .and_then(parse_ip)
        {
            ips.push(ClientIP {
                source: name.to_string(),
                ip,
            });
        }
    }

    if let Some(ip) = headers
        .get("Forwarded")
        .and_then(|v| v.to_str().ok())
        .and_then(parse_forwarded_header)
    {
        ips.push(ClientIP {
            source: "Forwarded".to_string(),
            ip,
        });
    }

    if let Some(xff) = headers
        .get("X-Forwarded-For")
        .and_then(|v| v.to_str().ok())
    {
        for entry in xff.split(',') {
            if let Some(ip) = parse_ip(entry) {
                ips.push(ClientIP {
                    source: "X-Forwarded-For".to_string(),
                    ip,
                });
            }
        }
    }

    if let Some(ip) = session
        .client_addr()
        .and_then(|addr| addr.as_inet())
        .map(|socket_addr| socket_addr.ip())
    {
        ips.push(ClientIP {
            source: "pingora_tcp_peer".to_string(),
            ip,
        });
    }

    ips
}
