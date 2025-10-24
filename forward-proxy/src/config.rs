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
    pub handler_config: HandlerConfig,
    #[serde(flatten)]
    pub influxdb_config: InfluxDBConfig,
}

#[derive(Debug, Deserialize)]
pub struct LogConfig {
    #[allow(dead_code)]
    pub log_path: String,
    pub log_level: String,
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

#[derive(Debug, Deserialize)]
pub struct InfluxDBConfig {
    pub influxdb_url: String,
    pub influxdb_org: String,
    pub influxdb_bucket: String,
    pub influxdb_auth_token: String,
}