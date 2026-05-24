use crate::mock::backend::run_mock_be;
use crate::mock::reverse_proxy::TEST_REVERSE_PROXY;

pub mod backend;
pub mod data;
pub mod reverse_proxy;

#[allow(dead_code)]
pub fn start_mock_services() {
    // start backend
    run_mock_be();

    // start reverse-proxy
    let _ = *TEST_REVERSE_PROXY;
}
