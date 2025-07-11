use once_cell::sync::Lazy;

// can be replaced by constants, will see
pub enum HeaderKeys {
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

const LAYER8_URL: &str = "http://127.0.0.1:5001";
const RP_URL: &str = "http://127.0.0.1:6194";

pub static RP_INIT_ENCRYPTED_TUNNEL_PATH: Lazy<String> = Lazy::new(|| format!("{}/init-tunnel", RP_URL));
pub static RP_PROXY_PATH: Lazy<String> = Lazy::new(|| format!("{}/proxy", RP_URL));
pub static LAYER8_GET_CERTIFICATE_PATH: Lazy<String> = Lazy::new(|| format!("{}/sp-pub-key?backend_url=", LAYER8_URL));

pub const NTOR_SERVER_ID: &str = "ntor_server_id";
pub const NTOR_SERVER_ID_TMP_VALUE: &str = "ReverseProxyServer";
pub const NTOR_STATIC_PUBLIC_KEY: &str = "ntor_static_public_key";
pub const NTOR_STATIC_PUBLIC_KEY_TMP_VALUE: [u8; 32] = [
    131, 210, 36, 101, 39, 191, 61, 165, 29, 112, 94, 149, 120, 202, 189, 170,
    151, 62, 247, 71, 208, 255, 144, 173, 52, 223, 239, 221, 153, 225, 40, 10
];

pub const INIT_TUNNEL_ENDPOINT: &str = "/init-tunnel";
pub const PROXY_ENDPOINT: &str = "/proxy";
pub const HEALTHCHECK_ENDPOINT: &str = "/healthcheck";