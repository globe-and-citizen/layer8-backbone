// can be replaced by constants, will see
pub enum HeaderKeys {
    FpRpJwtKey,
    IntRpJwtKey,
}

impl HeaderKeys {
    pub fn as_str(&self) -> &'static str {
        match self {
            HeaderKeys::FpRpJwtKey => "fp_rp_jwt",
            HeaderKeys::IntRpJwtKey => "int_rp_jwt"
        }
    }
}

pub const INIT_TUNNEL_TO_BACKEND_PATH: &str = "/init-tunnel";