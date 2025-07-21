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

pub enum CtxKeys {
    NTorServerId,
    NTorStaticPublicKey,
    UpstreamAddress,
    UpstreamSNI,
}

impl CtxKeys {
    pub fn to_string(&self) -> String {
        match self {
            CtxKeys::NTorServerId => "ntor_server_id".to_string(),
            CtxKeys::NTorStaticPublicKey => "ntor_static_public_key".to_string(),
            CtxKeys::UpstreamAddress => "upstream_address".to_string(),
            CtxKeys::UpstreamSNI => "upstream_sni".to_string(),
        }
    }
}