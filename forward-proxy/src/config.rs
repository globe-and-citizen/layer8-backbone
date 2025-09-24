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

impl TlsConfig {
    pub fn load(&mut self) -> Result<(), String> {
        // self.ca_cert = std::fs::read(&self.path_to_ca_cert)
        //     .map_err(|e| format!("Failed to read CA certificate: {}", e))?;
        // self.cert = std::fs::read(&self.path_to_cert)
        //     .map_err(|e| format!("Failed to read certificate: {}", e))?;
        // self.key = std::fs::read(&self.path_to_key)
        //     .map_err(|e| format!("Failed to read key: {}", e))?;
        Ok(())
    }
}