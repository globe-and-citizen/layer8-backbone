pub struct HeaderKeys;

impl HeaderKeys {
    pub const FP_RP_JWT: &'static str = "fp_rp_jwt";
    pub const INT_RP_JWT_KEY: &'static str = "int_rp_jwt";
}

pub struct LogTypes;

impl LogTypes {
    pub const ACCESS_LOG: &'static str = "ACCESS_LOG";
    pub const ACCESS_LOG_RESULT: &'static str = "ACCESS_LOG_RESULT";
    pub const HANDLE_INIT_TUNNEL_REQUEST: &'static str = "HANDLE_INIT_TUNNEL_REQUEST";
    pub const HANDLE_PROXY_REQUEST: &'static str = "HANDLE_PROXY_REQUEST";
    pub const HANDLE_BACKEND_RESPONSE: &'static str = "HANDLE_BACKEND_RESPONSE";
    #[allow(dead_code)]
    pub const HEALTHCHECK: &'static str = "HEALTHCHECK";
    pub const TLS_HANDSHAKE: &'static str = "TLS_HANDSHAKE";
}