use log::{error, info};
use reqwest::Client;
use crate::handler::common::consts::INIT_TUNNEL_TO_BACKEND_PATH;
use crate::handler::init_tunnel::InitTunnelRequestToBackend;

pub struct InitTunnelHandler {}

impl InitTunnelHandler {
    pub(crate) async fn init_tunnel_result_to_be(result: bool) {
        let body = InitTunnelRequestToBackend {
            success: result,
        };
        let log_meta = format!("[FORWARD {}]", INIT_TUNNEL_TO_BACKEND_PATH.as_str());
        info!("{log_meta} request to BE body: {:?}", body);

        let client = Client::new();
        match client.post(INIT_TUNNEL_TO_BACKEND_PATH.as_str())
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
        {
            Ok(res) => {
                info!("{log_meta} Response sending init-tunnel result to BE: {:?}", res)
            }
            Err(err) => {
                error!("{log_meta} Error sending init-tunnel result to BE: {:?}", err)
            }
        }
    }
}
