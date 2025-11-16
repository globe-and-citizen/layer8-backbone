use pingora::http::StatusCode;
use pingora_router::ctx::{Layer8Context, Layer8ContextTrait};
use pingora_router::handler::{APIHandlerResponse, DefaultHandlerTrait, ResponseBodyTrait};

use crate::handler::common::types::ErrorResponse;
use crate::handler::init_tunnel::InitEncryptedTunnelRequest;

/// Struct containing only associated methods (no instance methods or fields)
pub(crate) struct InitTunnelHandler;

impl DefaultHandlerTrait for InitTunnelHandler {}

impl InitTunnelHandler {
    pub(crate) async fn validate_request_body(
        ctx: &mut Layer8Context,
        _backend_url: String,
    ) -> Result<InitEncryptedTunnelRequest, APIHandlerResponse> {
        return match InitTunnelHandler::parse_request_body::<
            InitEncryptedTunnelRequest,
            ErrorResponse,
        >(&ctx.get_request_body())
        {
            Ok(res) => Ok(res),
            Err(err) => {
                let body = match err {
                    None => None,
                    Some(err_response) => Some(err_response.to_bytes()),
                };

                Err(APIHandlerResponse {
                    status: StatusCode::BAD_REQUEST,
                    body,
                })
            }
        };
    }
}
