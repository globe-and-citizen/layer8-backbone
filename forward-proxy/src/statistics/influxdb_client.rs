use futures::stream;
use influxdb2::Client;
use influxdb2::models::DataPoint;
use pingora::http::StatusCode;
use std::error::Error;
use crate::config::InfluxDBConfig;
use crate::handler::consts::RequestPaths;
use crate::statistics::StatisticsWriter;

struct InfluxDBMeasurements;

impl InfluxDBMeasurements {
    const TOTAL_BYTE_TRANSFERRED: &'static str = "total_byte_transferred";
    const TOTAL_TUNNEL_INITIATED: &'static str = "total_tunnel_initiated";
    const TOTAL_SUCCESS: &'static str = "total_success";
    const TOTAL_REQUEST: &'static str = "total_request";
}

/// A client for writing statistics to InfluxDB.
///
/// Manages the connection to InfluxDB and provides methods for updating
/// various metrics related to proxy requests (total requests, successful responses,
/// bytes transferred, initiated tunnels).
pub struct InfluxDBClient {
    client: Client,
    bucket: String,
}

impl InfluxDBClient {
    /// Creates a new instance of InfluxDBClient.
    ///
    /// # Arguments
    /// * `config` - InfluxDB configuration containing URL, organization, token, and bucket
    ///
    /// # Example
    /// ```ignore
    /// let client = InfluxDBClient::new(&config);
    /// ```
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

    /// Updates a counter metric in InfluxDB.
    ///
    /// Creates a data point with the specified metric, tags it with client\_id,
    /// and writes it to the InfluxDB bucket.
    ///
    /// # Arguments
    /// * `measurement` - name of the metric in InfluxDB
    /// * `client_id` - client identifier (used as a tag)
    /// * `value` - counter value to record
    ///
    /// # Returns
    /// * `Ok(())` if the write was successful
    /// * `Err` if an error occurred while building or writing the data point
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

    /// Adds transferred bytes to the total bytes counter.
    ///
    /// # Arguments
    /// * `client_id` - client identifier
    /// * `bytes_size` - number of bytes transferred
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

    /// Increments the tunnel initiated counter by 1.
    ///
    /// # Arguments
    /// * `client_id` - client identifier
    async fn increase_total_tunnel_initiated(
        &self,
        client_id: &str,
    ) -> Result<(), Box<dyn Error + Sync + Send>> {
        self.update_counter(InfluxDBMeasurements::TOTAL_TUNNEL_INITIATED, client_id, 1)
            .await
    }

    /// Increments the total request counter by 1.
    ///
    /// # Arguments
    /// * `client_id` - client identifier
    async fn increase_total_request(
        &self,
        client_id: &str,
    ) -> Result<(), Box<dyn Error + Sync + Send>> {
        self.update_counter(InfluxDBMeasurements::TOTAL_REQUEST, client_id, 1)
            .await
    }

    /// Increments the successful response counter by 1.
    ///
    /// # Arguments
    /// * `client_id` - client identifier
    async fn increase_total_success(
        &self,
        client_id: &str,
    ) -> Result<(), Box<dyn Error + Sync + Send>> {
        self.update_counter(InfluxDBMeasurements::TOTAL_SUCCESS, client_id, 1)
            .await
    }
}

#[async_trait::async_trait]
impl StatisticsWriter for InfluxDBClient {
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
    /// * `Err` if an error occurred while writing to InfluxDB
    async fn update_statistics(
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
}
