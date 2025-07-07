use std::fs;
use serde::Deserialize;
use toml;

#[derive(Debug, Deserialize)]
pub struct RPConfig {
    pub upstream: UpstreamConfig,
    pub log: LogConfig,
    pub server: ServerConfig,
    pub handler: HandlerConfig
}

impl RPConfig {
    /// panic if unable to validate.
    /// assuming after this validation, all configs are valid
    pub fn validate(&self) {
        // todo
    }
}

impl RPConfig {
    pub fn from_file(path: &str) -> Self {
        let content = fs::read_to_string(path).expect("Failed to read configuration file");
        toml::from_str(&content).expect("Failed to parse configuration file")
    }
}

#[derive(Debug, Deserialize)]
pub(super) struct UpstreamConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Deserialize)]
pub(super) struct LogConfig {
    pub path: String,
    pub level: String,
}

impl LogConfig {
    pub fn to_level_filter(&self) -> log::LevelFilter {
        match self.level.to_uppercase().as_str() {
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

#[derive(Debug, Deserialize)]
pub(super) struct ServerConfig {
    pub host: String,
    pub port: u16
}

#[derive(Debug, Deserialize)]
pub(super) struct HandlerConfig {
    pub ntor_server_id: String,
    pub ntor_static_secret: String,
    pub jwt_virtual_connection_secret: String,
    pub jwt_exp: i64,
    pub forward_proxy_url: Option<String>,
    pub backend_url: String,
}


