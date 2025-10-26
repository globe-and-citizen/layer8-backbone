mod influxdb_client;

use crate::config::InfluxDBConfig;
use crate::handler::consts::LogTypes;
use crate::statistics::influxdb_client::InfluxDBClient;
use futures::TryFutureExt;
use once_cell::sync::Lazy;
use tokio::sync::Mutex;
use tracing::error;

struct InfluxDBMeasurements;

impl InfluxDBMeasurements {
    const TOTAL_BYTE_TRANSFERRED: &'static str = "total_byte_transferred";
    const TOTAL_TUNNEL_INITIATED: &'static str = "total_tunnel_initiated";
    const TOTAL_SUCCESS: &'static str = "total_success";
    const TOTAL_REQUEST: &'static str = "total_request";
}

static INFLUXDB_CLIENT: Lazy<Mutex<Option<InfluxDBClient>>> = Lazy::new(|| Mutex::new(None));

pub struct Statistics;

impl Statistics {
    pub async fn init_influxdb_client(config: &InfluxDBConfig) {
        let mut influxdb_client = INFLUXDB_CLIENT.lock().await;
        *influxdb_client = Some(InfluxDBClient::new(&config));
    }

    pub async fn update(
        client_id: String,
        request_path: String,
        total_byte_transferred: i64,
        response_status: u16,
    ) {
        let client = INFLUXDB_CLIENT.lock().await;
        if let Some(ref influxdb_client) = *client {
            influxdb_client
                .update_statistics(
                    client_id,
                    request_path,
                    total_byte_transferred,
                    response_status,
                )
                .map_err(|e| {
                    error!(
                        log_type = LogTypes::INFLUXDB,
                        "Failed to update statistics: {:?}", e
                    );
                })
                .await
                .ok();
        }
    }
}
