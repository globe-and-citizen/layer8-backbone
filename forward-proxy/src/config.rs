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
    pub tls_config: ProxyConfig,
    #[serde(flatten)] // This flattens the HandlerConfig fields into this struct
    pub handler_config: HandlerConfig,
    #[serde(flatten)]
    pub influxdb_config: InfluxDBConfig,
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
pub struct ProxyConfig {
    #[serde(deserialize_with = "deserializer::string_to_bool")]
    pub enable_tls: bool,
    #[serde(default)]
    pub ca_cert: String,
    #[serde(default)]
    pub cert: String,
    #[serde(default)]
    pub key: String,
    #[serde(default)]
    pub path_to_ca_cert: String,
    #[serde(default)]
    pub path_to_cert: String,
    #[serde(default)]
    pub path_to_key: String,
    #[serde(deserialize_with = "utils::deserializer::string_to_bool")]
    pub cors_allow_credentials: bool,
    #[serde(deserialize_with = "deserializer::string_to_vec")]
    pub cors_allow_origins: Vec<String>,
}

impl ProxyConfig {
    pub fn load(&mut self) -> Result<(), String> {
        if self.ca_cert.is_empty() {
            self.ca_cert = std::fs::read_to_string(&self.path_to_ca_cert)
                .map_err(|e| format!("Failed to read CA certificate: {}", e))?;
        }

        if self.cert.is_empty() {
            self.cert = std::fs::read_to_string(&self.path_to_cert)
                .map_err(|e| format!("Failed to read certificate: {}", e))?;
        }

        if self.key.is_empty() {
            self.key = std::fs::read_to_string(&self.path_to_key)
                .map_err(|e| format!("Failed to read key: {}", e))?;
        }

        Ok(())
    }
}

#[derive(Debug, Deserialize)]
pub struct InfluxDBConfig {
    pub influxdb_url: String,
    pub influxdb_org: String,
    pub influxdb_bucket: String,
    pub influxdb_auth_token: String,
}