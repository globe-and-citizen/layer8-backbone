pub struct HeaderKeys;

impl HeaderKeys {
    #[allow(dead_code)]
    pub const INT_RP_JWT: &'static str = "int_rp_jwt";
    pub const INT_FP_JWT: &'static str = "int_fp_jwt";
    pub const FP_RP_JWT: &'static str = "fp_rp_jwt";
}

pub struct CtxKeys;

impl CtxKeys {
    pub const NTOR_SERVER_ID: &'static str = "ntor_server_id";
    pub const NTOR_STATIC_PUBLIC_KEY: &'static str = "ntor_static_public_key";
    pub const UPSTREAM_ADDRESS: &'static str = "upstream_address";
    pub const UPSTREAM_SNI: &'static str = "upstream_sni";
    #[allow(dead_code)]
    pub const INT_RP_JWT: &'static str = "int_rp_jwt";
    #[allow(dead_code)]
    pub const INT_FP_JWT: &'static str = "int_fp_jwt";
    pub const FP_RP_JWT: &'static str = "fp_rp_jwt";
    pub const BACKEND_AUTH_CLIENT_ID: &'static str = "backend_auth_client_id";
}

pub struct LogTypes;

impl LogTypes {
    pub const ACCESS_LOG: &'static str = "ACCESS_LOG";
    pub const ACCESS_LOG_RESULT: &'static str = "ACCESS_LOG_RESULT";
    pub const UPSTREAM_CONNECT: &'static str = "UPSTREAM_CONNECT";
    pub const HANDLE_CLIENT_REQUEST: &'static str = "HANDLE_CLIENT_REQUEST";
    pub const HANDLE_UPSTREAM_RESPONSE: &'static str = "HANDLE_UPSTREAM_RESPONSE";
    pub const HEALTHCHECK: &'static str = "HEALTHCHECK";
    pub const INFLUXDB: &'static str = "INFLUXDB";
    pub const AUTHENTICATION_SERVER: &'static str = "AUTHENTICATION_SERVER";
}

pub struct RequestPaths;

impl RequestPaths {
    pub const PROXY: &'static str = "/proxy";
    pub const INIT_TUNNEL: &'static str = "/init-tunnel";
    pub const HEALTHCHECK: &'static str = "/healthcheck";
}

