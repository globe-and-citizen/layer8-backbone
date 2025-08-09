use serde::Deserialize;
use crate::tls_conf::TlsConfig;

#[derive(Debug, Deserialize, Clone)]
pub struct RPConfig {
    #[serde(flatten)]
    pub log: LogConfig,
    #[serde(flatten)]
    pub server: ServerConfig,
    #[serde(flatten)]
    pub tls: TlsConfig,
    #[serde(flatten)]
    pub handler: HandlerConfig
}

#[derive(Debug, Deserialize, Clone)]
pub(super) struct LogConfig {
    pub log_path: String,
    pub log_level: String,
}

impl LogConfig {
    pub fn to_level_filter(&self) -> log::LevelFilter {
        match self.log_level.to_uppercase().as_str() {
            "INFO" => log::LevelFilter::Info,
            "DEBUG" => log::LevelFilter::Debug,
            "WARNING" => log::LevelFilter::Warn,
            "ERROR" => log::LevelFilter::Error,
            "TRACE" => log::LevelFilter::Trace,
            "OFF" => log::LevelFilter::Off,
            _ => log::max_level()
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub(super) struct ServerConfig {
    pub listen_address: String,
    #[serde(deserialize_with = "utils::deserializer::string_to_number")]
    pub listen_port: u16
}

#[derive(Debug, Deserialize, Clone)]
pub(super) struct HandlerConfig {
    pub ntor_server_id: String,
    #[serde(deserialize_with = "utils::deserializer::string_to_u8_32")]
    pub ntor_static_secret: [u8; 32],
    #[serde(deserialize_with = "utils::deserializer::string_to_vec_u8")]
    pub jwt_virtual_connection_secret: Vec<u8>,
    #[serde(deserialize_with = "utils::deserializer::string_to_number")]
    pub jwt_exp_in_hours: i64,
    pub forward_proxy_url: Option<String>,
    pub backend_url: String,
}
