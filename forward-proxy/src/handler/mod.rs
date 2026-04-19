use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use pingora::http::StatusCode;
use reqwest::Client;
use pingora_router::{
    ctx::{Layer8Context, Layer8ContextTrait},
    handler::{APIHandlerResponse, DefaultHandlerTrait, RequestBodyTrait, ResponseBodyTrait},
};
use serde::Deserialize;
use tracing::{debug, error, info};

use crate::handler::types::{
    response::{ErrorResponse, FpHealthcheckError, FpHealthcheckSuccess, InitTunnelResponseFromRP, InitTunnelResponseToINT},
    request::InitTunnelRequest,
};
use utils::{self, jwt::JWTClaims};
use crate::config::HandlerConfig;
use crate::handler::consts::LogTypes;

pub mod types;
pub mod consts;

/// Handles forward proxy request processing and NTor tunnel initialization.
///
/// The `ForwardHandler` is responsible for:
/// - Managing JWT session storage for int_fp_jwt tokens
/// - Fetching and validating NTor server certificates from authentication server
/// - Processing init-tunnel requests (NTor handshake setup)
/// - Handling tunnel initialization responses from reverse proxy
/// - Health check processing
///
/// # Session Management
/// Sessions are stored in-memory using a HashMap with int_fp_jwt as the key.
/// Each session contains client authentication info and credentials for reverse proxy communication.
///
/// # NTor Tunnel Flow
/// 1. Client initiates tunnel with `handle_init_tunnel_request()`
/// 2. Handler fetches NTor server certificate from authentication server
/// 3. Handler receives RP response with tunnel setup data
/// 4. Handler creates session and returns int_fp_jwt in `handle_init_tunnel_response()`
/// 5. Subsequent requests use int_fp_jwt to retrieve session info
///
/// # Thread Safety
/// - Uses Arc<Mutex<>> for thread-safe session storage
/// - Safe for concurrent request handling in async environment
pub struct ForwardHandler {
    pub config: HandlerConfig,
    /// Internal JWT → Session mapping for int_fp_jwt tokens
    /// Used to store and retrieve IntFPSession after tunnel initialization
    jwts_storage: Arc<Mutex<HashMap<String, IntFPSession>>>,
}

impl DefaultHandlerTrait for ForwardHandler {}

/// Represents an NTor server certificate with cryptographic credentials.
///
/// This structure holds the public key and server identifier retrieved from the
/// authentication server during tunnel initialization. The public key is used by
/// the client in the NTor handshake process to establish encrypted communication
/// with the reverse proxy.
///
/// # Fields
/// * `server_id` - Unique identifier for the NTor server (currently backend URL)
/// * `public_key` - The server's NTor public key in bytes (extracted from X.509 certificate)
#[derive(Debug)]
struct NTorServerCertificate {
    server_id: String,
    public_key: Vec<u8>,
}

/// Session data stored after successful tunnel initialization.
///
/// This structure represents an authenticated session between a client and the forward proxy.
/// It is created during tunnel initialization and stored using the int_fp_jwt as the key.
/// The session contains all information needed for subsequent proxy requests.
///
/// # Fields
/// * `client_id` - Unique identifier of the authenticated client (from auth server)
/// * `rp_base_url` - The reverse proxy base URL for this client's requests
/// * `fp_rp_jwt` - JWT token for forward proxy → reverse proxy communication
///
/// # Lifetime
/// Sessions are stored in the ForwardHandler's jwts_storage HashMap and are
/// retrieved using the int_fp_jwt when processing proxy requests.
///
/// # Security
/// - The `fp_rp_jwt` is specific to each session and client
/// - Sessions should have expiration enforcement (see `jwt_exp_in_hours` config)
/// - Each session is tied to the client_id and cannot be reused by others
#[derive(Clone, Debug, Default)]
pub struct IntFPSession {
    pub client_id: String,
    pub rp_base_url: String,
    pub fp_rp_jwt: String,
}

impl ForwardHandler {
    /// Creates a new ForwardHandler instance with the given configuration.
    ///
    /// Initializes the JWT session storage HashMap wrapped in Arc<Mutex<>> for thread-safe
    /// concurrent access in the async environment.
    ///
    /// # Arguments
    /// * `config` - Handler configuration containing auth server URLs, JWT keys, and timeouts
    ///
    /// # Returns
    /// A new ForwardHandler instance ready to process requests
    ///
    /// # Thread Safety
    /// The created handler is safe to use across multiple async tasks as the session
    /// storage is protected by Mutex
    pub fn new(config: HandlerConfig) -> Self {
        ForwardHandler {
            config,
            jwts_storage: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Retrieves a session from storage using the JWT token as the key.
    ///
    /// This method is called when processing proxy requests to obtain the cached
    /// `IntFPSession` that was created during tunnel initialization.
    ///
    /// # Arguments
    /// * `int_fp_jwt` - The JWT token string to look up in the session storage
    ///
    /// # Returns
    /// * `Ok(IntFPSession)` - The session if found and cloned from storage
    /// * `Err(String)` - Error message "token not found!" if the token is not in storage
    ///
    /// # Error Cases
    /// * Token has expired or been removed from storage
    /// * Token never existed (invalid token)
    /// * Token was cleared during handler shutdown
    ///
    /// # Note
    /// This method performs a clone of the session, so it's suitable for single-threaded
    /// retrieval. For concurrent access, the Mutex lock is properly handled.
    pub fn get_session(&self, int_fp_jwt: &str) -> Result<IntFPSession, String> {
        match {
            let jwts = self.jwts_storage.lock().unwrap();
            jwts.get(int_fp_jwt).cloned()
        } {
            None => Err("token not found!".to_string()),
            Some(session) => Ok(session)
        }
    }

    /// Fetches the NTor server certificate from the authentication server.
    ///
    /// Sends a GET request to the configured auth server with the provided `backend_url`
    /// to retrieve the server's public key and client ID. Stores the client ID in the
    /// request context for later use.
    ///
    /// # Authentication Server Flow
    /// 1. Build request URL: auth_get_certificate_url + backend_url
    /// 2. Send GET request with Authorization header (Bearer token)
    /// 3. Parse response containing X.509 certificate and client_id
    /// 4. Extract public key from X.509 certificate
    /// 5. Store client_id in context for session creation
    /// 6. Return NTorServerCertificate with server_id and public_key
    ///
    /// # Arguments
    /// * `backend_url` - The backend URL to request the certificate for
    /// * `ctx` - Mutable reference to the Layer8 context for storing response data
    ///
    /// # Returns
    /// * `Ok(NTorServerCertificate)` - Contains the server ID and extracted public key
    /// * `Err(APIHandlerResponse)` - HTTP error response if the request fails or parsing fails
    ///
    /// # Error Cases
    /// * **Connection Failure** (500 Internal Server Error)
    ///   - Cannot connect to authentication server
    ///   - Network timeout or DNS resolution failure
    ///
    /// * **HTTP Error Response** (400 Bad Request)
    ///   - Auth server returns non-2xx status code
    ///   - Logs failed request details
    ///
    /// * **Response Parsing Failure** (500 Internal Server Error)
    ///   - Response body is not valid JSON
    ///   - Missing required fields (cert, client_id)
    ///
    /// * **Certificate Parsing Failure** (500 Internal Server Error)
    ///   - X.509 certificate is malformed
    ///   - Public key extraction fails
    ///
    /// # Security Considerations
    /// - Authorization header uses Bearer token from config (must be kept secure)
    /// - X.509 certificate is validated during public key extraction
    /// - Client ID is securely stored in context for session creation
    /// - Errors include detailed logging with correlation ID for debugging
    async fn get_public_key(
        &self,
        backend_url: String,
        ctx: &mut Layer8Context,
    ) -> Result<NTorServerCertificate, APIHandlerResponse> {
        let correlation_id = ctx.get_correlation_id();
        let client = Client::new();

        let request_path = format!(
            "{}{}",
            self.config.auth_get_certificate_url,
            backend_url
        );
        let res = client.get(&request_path)
            .header("Authorization", self.config.auth_access_token.clone())
            .send()
            .await
            // unable to connect
            .map_err(|e| {
                let response_body = ErrorResponse {
                    error: format!("Failed to connect to layer8: {}", e)
                };

                APIHandlerResponse {
                    status: StatusCode::INTERNAL_SERVER_ERROR,
                    cookies: None,
                    body: Some(response_body.to_bytes()),
                }
            })?;

        // connected but request failed
        if !res.status().is_success() {
            let response_body = ErrorResponse {
                error: format!("Failed to get public key from layer8, status code: {}", res.status().as_u16()),
            };
            error!(
                %correlation_id,
                log_type=LogTypes::AUTHENTICATION_SERVER,
                "Failed to get ntor certificate for {request_path}: {response_body:?}"
            );

            ctx.insert_response_header("Connection", "close"); // Ensure connection closes???

            Err(APIHandlerResponse {
                status: StatusCode::BAD_REQUEST,
                cookies: None,
                body: Some(response_body.to_bytes()),
            })
        } else {
            #[derive(Deserialize, Debug)]
            struct AuthServerResponse {
                pub cert: String,
                pub client_id: String,
            }

            let auth_res: AuthServerResponse = res.json().await.map_err(|err| {
                error!(
                    %correlation_id,
                    log_type=LogTypes::AUTHENTICATION_SERVER,
                    "Failed to parse authentication server response: {:?}",
                    err
                );
                APIHandlerResponse {
                    status: StatusCode::INTERNAL_SERVER_ERROR,
                    cookies: None,
                    body: None,
                }
            })?;

            // save `client_id` to ctx for later use
            ctx.set(consts::CtxKeys::BACKEND_AUTH_CLIENT_ID.to_string(), auth_res.client_id.clone());

            let pub_key = utils::cert::extract_x509_pem(auth_res.cert.clone())
                .map_err(|e| {
                    error!(
                        %correlation_id,
                        log_type=LogTypes::AUTHENTICATION_SERVER,
                        "Failed to parse x509 certificate: {:?}",
                        e
                    );
                    APIHandlerResponse {
                        status: StatusCode::INTERNAL_SERVER_ERROR,
                        cookies: None,
                        body: None,
                    }
                })?;

            debug!(%correlation_id, "AuthenticationServer response: {:?}", auth_res);
            info!(
                %correlation_id,
                log_type=LogTypes::AUTHENTICATION_SERVER,
                "Obtained ntor credentials for backend_url: {}",
                backend_url
            );

            Ok(NTorServerCertificate {
                server_id: backend_url, // todo I still prefer taking the server_id value from certificate's subject
                public_key: pub_key,
            })
        }
    }

    /// Verify `int_fp_jwt` token and retrieve the associated session.
    ///
    /// Validates the JWT token using the configured virtual connection key and retrieves
    /// the corresponding `IntFPSession` from storage if the token is valid.
    ///
    /// # JWT Verification Process
    /// 1. Verify JWT signature using jwt_virtual_connection_key from config
    /// 2. Check token expiration (optional, based on JWT claims)
    /// 3. Retrieve session from storage using token as key
    /// 4. Return session if all validations pass
    ///
    /// # Arguments
    /// * `token` - The JWT token string to verify
    ///
    /// # Returns
    /// * `Ok(IntFPSession)` - The session associated with the verified token
    /// * `Err(String)` - Error message if token verification fails or session not found
    ///
    /// # Error Cases
    /// * **Invalid Signature** - Token was signed with different key
    /// * **Token Expired** - Token expiration time has passed (if claims are checked)
    /// * **Malformed Token** - Token does not follow JWT format (header.payload.signature)
    /// * **Token Not Found** - Token was valid but session was removed from storage
    ///   - This can happen if session expired or was cleared
    ///
    /// # Security Considerations
    /// - JWT signature validation ensures token authenticity
    /// - Token is only accepted if the session still exists in storage
    /// - Sessions can expire based on jwt_exp_in_hours config
    /// - Failed verification is logged for security audit trail
    ///
    /// # Note
    /// TODO: Currently claims are not fully validated - consider checking all claim fields
    pub fn verify_int_fp_jwt(
        &self,
        token: &str,
    ) -> Result<IntFPSession, String> {
        match utils::jwt::verify_jwt_token(token, &self.config.jwt_virtual_connection_key) {
            Ok(_claims) => {
                // todo check claims if needed
                self.get_session(token)
            }
            Err(err) => Err(err.to_string())
        }
    }

    /// Validates the request body and retrieves the NTor server certificate for tunnel initialization.
    ///
    /// This is the first step of tunnel initialization. Parses the incoming `InitTunnelRequest`
    /// from the request body, fetches the public key from the authentication server using the
    /// provided backend URL, and stores the NTor server credentials in the context for later use.
    ///
    /// # Tunnel Initialization Flow (Step 1 of 3)
    /// 1. Validate request body format (must be valid InitTunnelRequest JSON)
    /// 2. Extract backend_url from query parameters
    /// 3. Fetch NTor server certificate from authentication server
    /// 4. Extract and store NTor credentials in context:
    ///    - NTOR_SERVER_ID: Unique server identifier
    ///    - NTOR_STATIC_PUBLIC_KEY: Server's public key for NTor handshake
    /// 5. Return validated request body for next step in tunnel initialization
    ///
    /// # Arguments
    /// * `ctx` - Mutable reference to the Layer8 context containing:
    ///   - request body: The InitTunnelRequest JSON
    ///   - query param "backend_url": The reverse proxy backend URL
    ///
    /// # Returns
    /// * `APIHandlerResponse` with status 200 (OK) and the validated request body if successful
    /// * `APIHandlerResponse` with status 400 (BAD_REQUEST) if the request body is invalid
    /// * `APIHandlerResponse` with status 500 (INTERNAL_SERVER_ERROR) if certificate retrieval fails
    ///
    /// # Request Body Format
    /// The request must contain a valid `InitTunnelRequest` JSON structure with client's ephemeral public key
    /// and other NTor handshake initialization data.
    ///
    /// # Context Updates
    /// After successful execution:
    /// - NTOR_SERVER_ID - Set to the backend URL (TODO: should be certificate subject???)
    /// - NTOR_STATIC_PUBLIC_KEY - Set to hex-encoded server public key
    /// - BACKEND_AUTH_CLIENT_ID - Set by get_public_key() from auth server response
    ///
    /// # Error Cases
    /// * **Invalid Request Body** (400 Bad Request)
    ///   - Request body is not valid JSON
    ///   - Request body doesn't match InitTunnelRequest schema
    ///   - Missing required fields
    ///
    /// * **Missing Query Parameter** (would be caught in proxy.rs request_filter)
    ///   - backend_url is not provided
    ///
    /// * **Auth Server Failure** (500 Internal Server Error)
    ///   - Cannot connect to authentication server
    ///   - Auth server returns error response
    ///   - Auth server returns malformed certificate
    ///
    /// # Security Considerations
    /// - Request body is validated before processing
    /// - NTor public key is fetched from trusted authentication server
    /// - All NTor credentials are stored securely in context
    /// - Errors include detailed logging for debugging
    pub async fn handle_init_tunnel_request(&self, ctx: &mut Layer8Context) -> APIHandlerResponse {
        // validate request body
        let received_body = match ForwardHandler::parse_request_body::<
            InitTunnelRequest,
            ErrorResponse
        >(&ctx.get_request_body())
        {
            Ok(res) => res.to_bytes(),
            Err(Some(e)) => {
                return APIHandlerResponse {
                    status: StatusCode::BAD_REQUEST,
                    cookies: None,
                    body: Some(e.to_bytes()),
                };
            }
            Err(None) => {
                return APIHandlerResponse {
                    status: StatusCode::BAD_REQUEST,
                    cookies: None,
                    body: None,
                };
            }
        };

        // get public key to initialize encrypted tunnel
        {
            // it's safe to use unwrap here because this param was already checked in `request_filter`
            let backend_url = ctx.param("backend_url").unwrap_or(&"".to_string()).to_string();

            let server_certificate = match self.get_public_key(backend_url.to_string(), ctx).await {
                Ok(cert) => cert,
                Err(err) => return err
            };
            debug!("Server certificate: {:?}", server_certificate);

            ctx.set(
                consts::CtxKeys::NTOR_SERVER_ID.to_string(),
                server_certificate.server_id,
            );
            ctx.set(
                consts::CtxKeys::NTOR_STATIC_PUBLIC_KEY.to_string(),
                hex::encode(server_certificate.public_key),
            );
        }

        APIHandlerResponse {
            status: StatusCode::OK,
            cookies: None,
            body: Some(received_body),
        }
    }

    /// Handles the RP response during tunnel initialization.
    ///
    /// This is the final step of tunnel initialization. Extracts NTor server data from context,
    /// parses the RP response body into an `InitTunnelResponseFromRP` structure, creates a new
    /// JWT token for the INT-FP session, stores the session with client information and RP credentials,
    /// and constructs a response containing ephemeral public key, hash, JWT tokens, and NTor server data.
    ///
    /// # Tunnel Initialization Flow (Step 3 of 3 - Final Response)
    /// 1. Extract NTor credentials from context:
    ///    - NTOR_SERVER_ID: Backend URL identifier
    ///    - NTOR_STATIC_PUBLIC_KEY: Server's NTor public key (hex-encoded)
    /// 2. Parse RP response containing:
    ///    - Ephemeral public key for NTor handshake
    ///    - t_b hash for key verification
    ///    - int_rp_jwt: Token for INT → RP communication
    ///    - fp_rp_jwt: Token for FP → RP communication
    /// 3. Create new int_fp_jwt:
    ///    - Sign with jwt_virtual_connection_key
    ///    - Include expiration based on jwt_exp_in_hours config
    ///    - Generate unique UUID for session tracking
    /// 4. Create and store IntFPSession:
    ///    - Key: int_fp_jwt (newly created)
    ///    - Value: IntFPSession with client_id, rp_base_url, fp_rp_jwt
    ///    - Storage: jwts_storage HashMap (in-memory)
    /// 5. Construct response with all NTor data and tokens for INT
    ///
    /// # Arguments
    /// * `ctx` - Mutable reference to the Layer8 context containing:
    ///   - NTor context values set by handle_init_tunnel_request()
    ///   - response body: Parsed InitTunnelResponseFromRP from reverse proxy
    ///   - BACKEND_AUTH_CLIENT_ID: Client ID from authentication server
    ///
    /// # Returns
    /// * `APIHandlerResponse` with:
    ///   - status 200 (OK): Response serialized as InitTunnelResponseToINT
    ///   - status 500: If RP response parsing fails
    ///
    /// # Session Storage
    /// After successful execution:
    /// - Session stored in jwts_storage with key = int_fp_jwt
    /// - Session contains client_id, rp_base_url, fp_rp_jwt
    /// - Session lifetime: Managed by jwt_exp_in_hours config
    /// - Future requests use int_fp_jwt to retrieve this session
    ///
    /// # Response Structure
    /// Returns `InitTunnelResponseToINT` containing:
    /// - `ephemeral_public_key`: Derived from RP response for client-side NTor computation
    /// - `t_b_hash`: Hash value for verification (from RP)
    /// - `int_rp_jwt`: Token for INT → RP communication (from RP)
    /// - `int_fp_jwt`: NEW token for INT → FP proxy communication (created here)
    /// - `ntor_static_public_key`: Server's public key for NTor (from context)
    /// - `ntor_server_id`: Server identifier (from context)
    ///
    /// # Error Cases
    /// * **RP Response Parsing Failure** (500 Internal Server Error)
    ///   - Response body is not valid JSON
    ///   - Response is missing required fields
    ///   - Response contains invalid data
    ///
    /// # Security Considerations
    /// - int_fp_jwt is cryptographically signed and unique per session
    /// - Sessions are tied to client_id from authentication server
    /// - fp_rp_jwt is securely stored and used for RP communication
    /// - Session storage is protected by Mutex for thread safety
    /// - All tokens have expiration enforcement
    /// - NTor credentials ensure encrypted tunnel communication
    pub fn handle_init_tunnel_response(&self, ctx: &mut Layer8Context) -> APIHandlerResponse {
        let ntor_server_id = ctx.get(&consts::CtxKeys::NTOR_SERVER_ID.to_string()).unwrap_or(&"".to_string()).clone();
        let ntor_static_public_key = hex::decode(
            ctx.get(&consts::CtxKeys::NTOR_STATIC_PUBLIC_KEY.to_string()).clone().unwrap_or(&"".to_string())
        ).unwrap_or_default();

        let response_body = ctx.get_response_body();

        match utils::bytes_to_json::<InitTunnelResponseFromRP>(response_body) {
            Err(e) => {
                error!(
                    correlation_id=ctx.get_correlation_id(),
                    log_type=LogTypes::HANDLE_UPSTREAM_RESPONSE,
                    "Error parsing RP response: {:?}",
                    e
                );
                APIHandlerResponse {
                    status: StatusCode::INTERNAL_SERVER_ERROR,
                    cookies: None,
                    body: None,
                }
            }
            Ok(res_from_rp) => {
                let int_fp_jwt = {
                    let mut claims = JWTClaims::new(Some(self.config.jwt_exp_in_hours));
                    claims.uuid = Some(utils::new_uuid());
                    utils::jwt::create_jwt_token(claims, &self.config.jwt_virtual_connection_key)
                };

                let int_fp_session = IntFPSession {
                    client_id: ctx.get(&consts::CtxKeys::BACKEND_AUTH_CLIENT_ID.to_string()).unwrap_or(&"".to_string()).to_string(),
                    rp_base_url: ctx.param("backend_url").unwrap_or(&"".to_string()).to_string(),
                    fp_rp_jwt: res_from_rp.fp_rp_jwt,
                };

                let mut jwts = self.jwts_storage.lock().unwrap();
                jwts.insert(int_fp_jwt.clone(), int_fp_session);

                let res_to_int = InitTunnelResponseToINT {
                    ephemeral_public_key: res_from_rp.public_key,
                    t_b_hash: res_from_rp.t_b_hash,
                    int_rp_jwt: res_from_rp.int_rp_jwt,
                    int_fp_jwt,
                    ntor_static_public_key,
                    ntor_server_id,
                };

                APIHandlerResponse {
                    status: StatusCode::OK,
                    cookies: None,
                    body: Some(res_to_int.to_bytes()),
                }
            }
        }
    }

    /// Handles `/healthcheck` endpoint for the forward proxy.
    ///
    /// Returns 418 IM_A_TEAPOT if `error=true` query parameter is present,
    /// otherwise returns 200 OK with success response.
    pub fn handle_healthcheck(&self, ctx: &mut Layer8Context) -> APIHandlerResponse {
        if let Some(error) = ctx.param("error") {
            if error == "true" {
                let response_bytes = FpHealthcheckError {
                    fp_healthcheck_error: "this is placeholder for a custom error".to_string()
                }.to_bytes();

                ctx.insert_response_header("x-fp-healthcheck-error", "response-header-error");
                return APIHandlerResponse {
                    status: StatusCode::IM_A_TEAPOT,
                    cookies: None,
                    body: Some(response_bytes),
                };
            }
        }

        let response_bytes = FpHealthcheckSuccess {
            fp_healthcheck_success: "this is placeholder for a custom body".to_string(),
        }.to_bytes();

        ctx.insert_response_header("x-fp-healthcheck-success", "response-header-success");

        APIHandlerResponse {
            status: StatusCode::OK,
            cookies: None,
            body: Some(response_bytes),
        }
    }
}
