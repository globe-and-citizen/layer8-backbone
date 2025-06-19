use log::{error, info};
use pingora::http::StatusCode;
use pingora_router::ctx::{Layer8Context, Layer8ContextTrait};
use pingora_router::handler::{APIHandlerResponse, DefaultHandlerTrait, ResponseBodyTrait};
use reqwest::Client;
use crate::handler::common::consts::INIT_TUNNEL_TO_BACKEND_PATH;
use crate::handler::common::types::ErrorResponse;
use crate::handler::init_tunnel::{InitEncryptedTunnelRequest, InitTunnelRequestToBackend};

/// Struct containing only associated methods (no instance methods or fields)
pub(crate) struct InitTunnelHandler {}

impl DefaultHandlerTrait for InitTunnelHandler {}

impl InitTunnelHandler {
    pub(crate) async fn validate_request_body(ctx: &mut Layer8Context)
        -> Result<InitEncryptedTunnelRequest, APIHandlerResponse>
    {
        return match InitTunnelHandler::parse_request_body::<
            InitEncryptedTunnelRequest,
            ErrorResponse
        >(&ctx.get_request_body())
        {
            Ok(res) => Ok(res),
            Err(err) => {
                let body = match err {
                    None => None,
                    Some(err_response) => Some(err_response.to_bytes())
                };

                InitTunnelHandler::send_result_to_be(false).await;

                Err(APIHandlerResponse {
                    status: StatusCode::BAD_REQUEST,
                    body,
                })
            }
        };
    }

    pub(crate) async fn send_result_to_be(result: bool) {
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
