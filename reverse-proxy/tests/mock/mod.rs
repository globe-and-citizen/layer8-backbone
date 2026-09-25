use crate::mock::backend::run_mock_be;
use crate::mock::reverse_proxy::TEST_REVERSE_PROXY;
use std::time::Duration;
use tokio::time::sleep;

pub mod backend;
pub mod data;
pub mod reverse_proxy;

#[allow(dead_code)]
pub async fn start_mock_services() {
    // start backend
    run_mock_be().await;

    // start reverse-proxy
    let _ = *TEST_REVERSE_PROXY;
    sleep(Duration::from_secs(10)).await;
}
