use pingora::http::StatusCode;
use pingora_router::ctx::{Layer8Context, Layer8ContextTrait};
use pingora_router::handler::{APIHandlerResponse, DefaultHandlerTrait, ResponseBodyTrait};
use reqwest::Client;
use tracing::{error, info};
use crate::handler::common::types::ErrorResponse;
use crate::handler::init_tunnel::{InitEncryptedTunnelRequest, InitTunnelRequestToBackend};

/// Struct containing only associated methods (no instance methods or fields)
pub(crate) struct InitTunnelHandler {}

impl DefaultHandlerTrait for InitTunnelHandler {}

impl InitTunnelHandler {
    pub(crate) async fn validate_request_body(
        ctx: &mut Layer8Context,
        _backend_url: String,
    ) -> Result<InitEncryptedTunnelRequest, APIHandlerResponse>
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

                // InitTunnelHandler::send_result_to_be(backend_url, false).await;

                Err(APIHandlerResponse {
                    status: StatusCode::BAD_REQUEST,
                    body,
                })
            }
        };
    }

    /// Deprecated: Sends the result of the init-tunnel operation to the backend service.
    #[allow(dead_code)]
    pub(crate) async fn send_result_to_be(backend_url: String, result: bool) {
        let body = InitTunnelRequestToBackend {
            success: result,
        };

        let request_url = format!("{backend_url}/init-tunnel");

        let log_meta = format!("[FORWARD {}]", request_url);
        info!("Request to BE body: {:?}", body);

        let client = Client::new();
        match client.post(request_url)
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
