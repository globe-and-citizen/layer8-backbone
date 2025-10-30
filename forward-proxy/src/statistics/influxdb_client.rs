use crate::config::InfluxDBConfig;
use crate::handler::consts::RequestPaths;
use crate::statistics::InfluxDBMeasurements;
use futures::stream;
use influxdb2::Client;
use influxdb2::models::DataPoint;
use pingora::http::StatusCode;
use std::error::Error;

pub struct InfluxDBClient {
    client: Client,
    bucket: String,
}

impl InfluxDBClient {
    pub fn new(config: &InfluxDBConfig) -> Self {
        let influxdb_client = Client::new(
            &config.influxdb_url,
            &config.influxdb_org,
            &config.influxdb_auth_token,
        );
        InfluxDBClient {
            client: influxdb_client,
            bucket: config.influxdb_bucket.clone(),
        }
    }

    pub async fn update_statistics(
        &self,
        client_id: String,
        request_path: String,
        total_byte_transferred: i64,
        response_status: u16,
    ) -> Result<(), Box<dyn Error + Sync + Send>> {
        self.increase_total_request(&client_id).await?;

        if response_status == StatusCode::OK {
            return match request_path.as_str() {
                RequestPaths::PROXY => {
                    self.add_total_byte_transferred(&client_id, total_byte_transferred)
                        .await?;

                    self.increase_total_success(&client_id).await
                }
                RequestPaths::INIT_TUNNEL => self.increase_total_tunnel_initiated(&client_id).await,
                _ => Ok(()),
            };
        }

        Ok(())
    }

    async fn update_counter(
        &self,
        measurement: &str,
        client_id: &str,
        value: i64,
    ) -> Result<(), Box<dyn Error + Sync + Send>> {
        // Create a data point
        let point = DataPoint::builder(measurement)
            .tag("client_id", client_id)
            .field("counter", value)
            .build()
            .map_err(|e| {
                Box::<dyn Error + Sync + Send>::from(format!(
                    "Failed to increase counter for {}: {:?}",
                    measurement, e
                ))
            })?;

        // Write to bucket
        self.client
            .write(self.bucket.as_str(), stream::iter(vec![point]))
            .await
            .map_err(|e| {
                Box::<dyn Error + Sync + Send>::from(format!(
                    "Failed to write counter for {}: {:?}",
                    measurement, e
                ))
            })?;
        Ok(())
    }

    async fn add_total_byte_transferred(
        &self,
        client_id: &str,
        bytes_size: i64,
    ) -> Result<(), Box<dyn Error + Sync + Send>> {
        self.update_counter(
            InfluxDBMeasurements::TOTAL_BYTE_TRANSFERRED,
            client_id,
            bytes_size,
        )
        .await
    }

    async fn increase_total_tunnel_initiated(
        &self,
        client_id: &str,
    ) -> Result<(), Box<dyn Error + Sync + Send>> {
        self.update_counter(InfluxDBMeasurements::TOTAL_TUNNEL_INITIATED, client_id, 1)
            .await
    }

    async fn increase_total_request(
        &self,
        client_id: &str,
    ) -> Result<(), Box<dyn Error + Sync + Send>> {
        self.update_counter(InfluxDBMeasurements::TOTAL_REQUEST, client_id, 1)
            .await
    }

    async fn increase_total_success(
        &self,
        client_id: &str,
    ) -> Result<(), Box<dyn Error + Sync + Send>> {
        self.update_counter(InfluxDBMeasurements::TOTAL_SUCCESS, client_id, 1)
            .await
    }
}
