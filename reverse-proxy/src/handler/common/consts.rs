use clap::__macro_refs::once_cell::sync::Lazy;

// can be replaced by constants, will see
pub enum HeaderKeys {
    RpHeaderRequestKey,
    RpHeaderResponseKey,
    SpaHeaderRequestKey,
    IntHeaderRequestKey,
    FpHeaderRequestKey,
    BeHeaderResponseKey,
    NTorSessionIDKey
}

impl HeaderKeys {
    pub fn as_str(&self) -> &'static str {
        match self {
            HeaderKeys::RpHeaderRequestKey => "rp_request_header",
            HeaderKeys::RpHeaderResponseKey => "rp_response_header",
            HeaderKeys::SpaHeaderRequestKey => "spa_request_header",
            HeaderKeys::BeHeaderResponseKey => "be_response_header",
            HeaderKeys::FpHeaderRequestKey => "fp_request_header",
            HeaderKeys::IntHeaderRequestKey => "int_request_header",
            HeaderKeys::NTorSessionIDKey => "ntor-session-id"
        }
    }

    pub fn placeholder_value(&self) -> &'static str {
        match self {
            HeaderKeys::RpHeaderRequestKey => "added in ReverseProxy",
            HeaderKeys::RpHeaderResponseKey => "added in ReverseProxy",
            _ => ""
        }
    }
}

// fixme BE path should be taken` from configuration
pub const BACKEND_HOST: &str = "http://localhost:3000";
pub static INIT_TUNNEL_TO_BACKEND_PATH: Lazy<String> = Lazy::new(|| format!("{}/init-tunnel", BACKEND_HOST));