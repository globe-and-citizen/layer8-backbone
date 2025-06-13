use once_cell::sync::Lazy;

// can be replaced by constants, will see
pub enum ForwardHeaderKeys {
    FpHeaderRequestKey,
    FpHeaderResponseKey,
}

impl ForwardHeaderKeys {
    pub fn as_str(&self) -> &'static str {
        match self {
            ForwardHeaderKeys::FpHeaderRequestKey => "fp_request_header",
            ForwardHeaderKeys::FpHeaderResponseKey => "fp_response_header"
        }
    }

    pub fn placeholder_value(&self) -> &'static str {
        match self {
            ForwardHeaderKeys::FpHeaderRequestKey => "placeholder value",
            ForwardHeaderKeys::FpHeaderResponseKey => "placeholder value"
        }
    }
}

const LAYER8_URL: &str = "http://127.0.0.1:5001";
const RP_URL: &str = "http://127.0.0.1:6193";
pub static RP_INIT_ENCRYPTED_TUNNEL_PATH: Lazy<String> = Lazy::new(|| format!("{}/init_tunnel", RP_URL));
pub static RP_PROXY_PATH: Lazy<String> = Lazy::new(|| format!("{}/proxy", RP_URL));
pub static LAYER8_GET_CERTIFICATE_PATH: Lazy<String> = Lazy::new(|| format!("{}/sp-pub-key?backend_url=", LAYER8_URL));
