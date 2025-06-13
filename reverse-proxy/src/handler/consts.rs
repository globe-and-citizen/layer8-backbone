use clap::__macro_refs::once_cell::sync::Lazy;

// can be replaced by constants, will see
pub enum HeaderKeys {
    RpHeaderRequestKey,
    RpHeaderResponseKey,
    SpaHeaderRequestKey,
    BeHeaderResponseKey
}

impl HeaderKeys {
    pub fn as_str(&self) -> &'static str {
        match self {
            HeaderKeys::RpHeaderRequestKey => "rp_request_header",
            HeaderKeys::RpHeaderResponseKey => "rp_response_header",
            HeaderKeys::SpaHeaderRequestKey => "spa_request_header",
            HeaderKeys::BeHeaderResponseKey => "be_response_header"
        }
    }

    pub fn placeholder_value(&self) -> &'static str {
        match self {
            HeaderKeys::RpHeaderRequestKey => "placeholder value",
            HeaderKeys::RpHeaderResponseKey => "placeholder value",
            _ => ""
        }
    }
}

const BACKEND_URL: &str = "http://localhost:3000";
pub static PROXY_TO_BACKEND_PATH: Lazy<String> = Lazy::new(|| format!("{}/proxy", BACKEND_URL));
