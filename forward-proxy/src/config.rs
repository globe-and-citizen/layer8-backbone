use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct FPConfig {
    pub listen_address: String,
    pub listen_port: u16,
    #[serde(flatten)]
    pub log_config: LogConfig,
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
    #[serde(deserialize_with = "string_to_vec_u8")]
    pub jwt_virtual_connection_key: Vec<u8>,
    #[serde(deserialize_with = "string_to_i64")]
    pub jwt_exp_in_hours: i64,
    pub auth_access_token: String,
    pub auth_get_certificate_url: String,
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

fn string_to_i64<'de, D>(deserializer: D) -> Result<i64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer)?;
    s.parse::<i64>().map_err(serde::de::Error::custom)
}

fn string_to_vec_u8<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer)?;
    Ok(s.into_bytes())
}