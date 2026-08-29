use serde::Deserialize;
use utils::cert::TLSConfig;
use utils::log::LogConfig;
use utils::telemetry::TelemetryConfig;

#[derive(Debug, Deserialize, Clone, Default)]
pub struct RPConfig {
    #[serde(flatten)]
    pub log: LogConfig,
    #[serde(flatten)]
    pub server: ServerConfig,
    #[serde(flatten)]
    pub proxy: ProxyConfig,
    #[serde(flatten)]
    pub handler: HandlerConfig,
    #[serde(flatten)]
    pub telemetry: TelemetryConfig,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct ServerConfig {
    pub listen_address: String,
    #[serde(deserialize_with = "utils::deserializer::string_to_number")]
    pub listen_port: u16,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct HandlerConfig {
    pub ntor_server_id: String,
    #[serde(deserialize_with = "utils::deserializer::string_to_u8_32")]
    pub ntor_static_secret: [u8; 32],
    #[serde(deserialize_with = "utils::deserializer::string_to_vec_u8")]
    pub jwt_virtual_connection_secret: Vec<u8>,
    #[serde(deserialize_with = "utils::deserializer::string_to_number")]
    pub jwt_exp_in_hours: i64,
    pub backend_url: String,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct ProxyConfig {
    #[serde(flatten)]
    pub tls: TLSConfig,
    #[serde(default, deserialize_with = "utils::deserializer::string_to_bool")]
    pub cors_allow_credentials: bool,
    #[serde(default, deserialize_with = "utils::deserializer::string_to_vec")]
    pub cors_allow_origins: Vec<String>,
    #[serde(default, deserialize_with = "utils::deserializer::string_to_bool")]
    pub use_correlation_id: bool,
}
