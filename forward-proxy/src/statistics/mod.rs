pub mod influxdb_client;

use std::error::Error;
use futures::TryFutureExt;
use once_cell::sync::Lazy;
use tokio::sync::Mutex;
use tracing::error;
use crate::handler::consts::LogTypes;

#[async_trait::async_trait]
pub trait StatisticsWriter: Send + Sync {
    /// Updates statistics for a request.
    ///
    /// Increments the total request counter, and depending on the response status
    /// and request path, updates additional metrics (bytes, success count, tunnel initiations).
    ///
    /// # Arguments
    /// * `client_id` - unique identifier of the client
    /// * `request_path` - request path (PROXY or INIT\_TUNNEL)
    /// * `total_byte_transferred` - number of bytes transferred
    /// * `response_status` - HTTP response status code
    ///
    /// # Returns
    /// * `Ok(())` if the update was successful
    /// * `Err` if an error occurred while writing
    async fn update_statistics(
        &self,
        client_id: String,
        request_path: String,
        total_byte_transferred: i64,
        response_status: u16,
    ) -> Result<(), Box<dyn Error + Sync + Send>>;
}

static INFLUXDB_CLIENT: Lazy<Mutex<Option<Box<dyn StatisticsWriter>>>> = Lazy::new(|| Mutex::new(None));

pub struct Statistics;

impl Statistics {
    pub async fn init_statistics_writer(writer: Box<dyn StatisticsWriter>) {
        let mut influxdb_client = INFLUXDB_CLIENT.lock().await;
        *influxdb_client = Some(writer);
    }

    pub async fn update(
        client_id: String,
        correlation_id: String,
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
                        %correlation_id,
                        log_type = LogTypes::INFLUXDB,
                        "Failed to update statistics: {:?}", e
                    );
                })
                .await
                .ok();
        }
    }
}
