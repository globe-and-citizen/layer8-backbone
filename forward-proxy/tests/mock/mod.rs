use ::forward_proxy::statistics::Statistics;
use crate::mock::forward_proxy::TEST_FORWARD_PROXY;

pub(crate) mod stats;
mod auth_server;
mod backend;
pub mod data;
mod forward_proxy;

pub async fn init() {
    // Start the mock authentication server
    auth_server::start_mock_auth_server().await;

    // Start the mock backend server
    backend::start_mock_backend_server().await;

    // Start the mock statistics server
    let mock_stats_writer = stats::MockStatisticsWriter{};
    Statistics::init_statistics_writer(Box::new(mock_stats_writer)).await;

    // Start Forward Proxy
    let _ = *TEST_FORWARD_PROXY;
}



