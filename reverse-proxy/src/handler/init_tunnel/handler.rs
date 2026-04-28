use pingora::http::StatusCode;
use pingora_router::ctx::{Layer8Context, Layer8ContextTrait};
use pingora_router::handler::{APIHandlerResponse, DefaultHandlerTrait, ResponseBodyTrait};
use crate::handler::common::types::ErrorResponse;
use crate::handler::init_tunnel::{InitEncryptedTunnelRequest};

/// Struct containing only associated methods (no instance methods or fields)
pub(crate) struct InitTunnelHandler {}

impl DefaultHandlerTrait for InitTunnelHandler {}

impl InitTunnelHandler {
    /// Validates the request body for initializing an encrypted tunnel.
    ///
    /// # Arguments
    ///
    /// * `ctx` - The layer 8 context containing the request data
    /// * `_backend_url` - The backend URL (currently unused)
    ///
    /// # Returns
    ///
    /// Returns `Ok(InitEncryptedTunnelRequest)` if the body is valid,
    /// or `Err(APIHandlerResponse)` with a BAD_REQUEST status if parsing fails or the public key length is invalid.
    pub(crate) async fn validate_request_body(
        ctx: &mut Layer8Context,
        _backend_url: String,
    ) -> Result<InitEncryptedTunnelRequest, APIHandlerResponse>
    {
        match InitTunnelHandler::parse_request_body::<
            InitEncryptedTunnelRequest,
            ErrorResponse
        >(&ctx.get_request_body())
        {
            Ok(res) => {
                if res.public_key.len() != 32 {
                    return Err(APIHandlerResponse {
                        status: StatusCode::BAD_REQUEST,
                        body: Some("Invalid public key length".as_bytes().to_vec()),
                        cookies: None,
                    });
                }
                Ok(res)
            }
            Err(err) => {
                let body = match err {
                    None => None,
                    Some(err_response) => Some(err_response.to_bytes())
                };

                Err(APIHandlerResponse {
                    status: StatusCode::BAD_REQUEST,
                    cookies: None,
                    body,
                })
            }
        }
    }
}
