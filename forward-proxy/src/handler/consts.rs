// can be replaced by constants, will see
pub enum HeaderKeys {
    #[allow(dead_code)]
    IntRpJwtKey,
    IntFpJwtKey,
    FpRpJwtKey
}

impl HeaderKeys {
    pub fn as_str(&self) -> &'static str {
        match self {
            HeaderKeys::IntRpJwtKey => "int_rp_jwt",
            HeaderKeys::IntFpJwtKey => "int_fp_jwt",
            HeaderKeys::FpRpJwtKey => "fp_rp_jwt",
        }
    }
}

pub enum CtxKeys {
    NTorServerId,
    NTorStaticPublicKey,
    UpstreamAddress,
    UpstreamSNI,
    #[allow(dead_code)]
    IntRPJwt,
    #[allow(dead_code)]
    IntFPJwt,
    FpRpJwt,
}

impl CtxKeys {
    pub fn to_string(&self) -> String {
        match self {
            CtxKeys::NTorServerId => "ntor_server_id".to_string(),
            CtxKeys::NTorStaticPublicKey => "ntor_static_public_key".to_string(),
            CtxKeys::UpstreamAddress => "upstream_address".to_string(),
            CtxKeys::UpstreamSNI => "upstream_sni".to_string(),
            CtxKeys::IntRPJwt => "int_rp_jwt".to_string(),
            CtxKeys::IntFPJwt => "int_fp_jwt".to_string(),
            CtxKeys::FpRpJwt => "fp_rp_jwt".to_string(),
        }
    }
}

pub struct LogTypes {}

impl LogTypes {
    pub const ACCESS_LOG: &'static str = "ACCESS_LOG";
    pub const ACCESS_LOG_RESULT: &'static str = "ACCESS_LOG_RESULT";
    pub const UPSTREAM_CONNECT: &'static str = "UPSTREAM_CONNECT";
    pub const HANDLE_CLIENT_REQUEST: &'static str = "HANDLE_CLIENT_REQUEST";
    pub const HANDLE_UPSTREAM_RESPONSE: &'static str = "HANDLE_UPSTREAM_RESPONSE";
    pub const HEALTHCHECK: &'static str = "HEALTHCHECK";
}