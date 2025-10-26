use serde::Deserialize;
use utils::deserializer;

#[derive(Debug, Deserialize)]
pub struct FPConfig {
    pub listen_address: String,
    #[serde(deserialize_with = "deserializer::string_to_number")]
    pub listen_port: u16,
    #[serde(flatten)]
    pub log_config: LogConfig,
    #[serde(flatten)]
    pub tls_config: TlsConfig,
    #[serde(flatten)] // This flattens the HandlerConfig fields into this struct
    pub handler_config: HandlerConfig
}

#[derive(Debug, Deserialize)]
pub struct LogConfig {
    pub log_level: String,
    /// default to "json" if not "plain"
    pub log_format: String,
    /// "console" or folder path
    pub log_path: String,
    /// required if log_path is not "console"
    pub log_filename: String,
}

#[derive(Debug, Deserialize)]
pub struct HandlerConfig {
    #[serde(deserialize_with = "deserializer::string_to_vec_u8")]
    pub jwt_virtual_connection_key: Vec<u8>,
    #[serde(deserialize_with = "deserializer::string_to_number")]
    pub jwt_exp_in_hours: i64,
    pub auth_access_token: String,
    pub auth_get_certificate_url: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct TlsConfig {
    #[serde(deserialize_with = "deserializer::string_to_bool")]
    pub enable_tls: bool,
    pub ca_cert: String,
    pub cert: String,
    pub key: String,
}