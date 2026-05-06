use std::error::Error;
use forward_proxy::statistics::StatisticsWriter;

pub struct MockStatisticsWriter;

#[async_trait::async_trait]
impl StatisticsWriter for MockStatisticsWriter {
    async fn update_statistics(
        &self,
        _client_id: String,
        _request_path: String,
        _total_byte_transferred: i64,
        _response_status: u16,
    ) -> Result<(), Box<dyn Error + Sync + Send>> {
        Ok(())
    }
}