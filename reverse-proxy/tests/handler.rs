#[cfg(test)]
mod test_handler {
    mod test_get_ntor_shared_secret {
        use pingora::http::StatusCode;
        use reverse_proxy::config::{HandlerConfig, ServerConfig};
        use reverse_proxy::handler::{InMemorySecretsStorage, ReverseHandler};

        fn create_test_handler() -> ReverseHandler {
            let handler_config = HandlerConfig {
                ntor_server_id: "test_server_id".to_string(),
                ntor_static_secret: [0u8; 32],
                jwt_virtual_connection_secret: vec![],
                jwt_exp_in_hours: 24,
                backend_url: "http://localhost:8080".to_string(),
            };

            let rp_config = reverse_proxy::config::RPConfig {
                proxy: reverse_proxy::config::ProxyConfig {
                    tls: utils::cert::TLSConfig {
                        enable_tls: false,
                        ca_path: "".to_string(),
                        cert_path: "path/to/cert.pem".to_string(),
                        key_path: "path/to/key.pem".to_string(),
                    },
                    cors_allow_credentials: false,
                    cors_allow_origins: vec![],
                },
                handler: handler_config,
                log: reverse_proxy::config::LogConfig {
                    log_level: "debug".to_string(),
                    log_format: "json".to_string(),
                    log_path: "./logs".to_string(),
                    log_filename: "reverse_proxy.log".to_string(),
                },
                server: ServerConfig { listen_address: "".to_string(), listen_port: 0 },
            };

            ReverseHandler::new(rp_config)
        }

        #[test]
        fn test_get_ntor_shared_secret_invalid_session() {
            let handler = create_test_handler();
            let result = handler.get_ntor_shared_secret("invalid_session_id".to_string());

            assert!(result.is_err());
            let err_response = result.unwrap_err();
            assert_eq!(err_response.status, StatusCode::UNAUTHORIZED);
            assert_eq!(err_response.cookies, None);
            assert!(err_response.body.is_some());
            let body = err_response.body.unwrap();
            assert_eq!(body, "Invalid or expired nTor session ID".as_bytes().to_vec());
        }

        #[test]
        fn test_get_ntor_shared_secret_valid_session() {
            let handler = create_test_handler();
            let session_id = "test_session_123".to_string();
            let secret = vec![1, 2, 3, 4, 5];

            InMemorySecretsStorage::insert(session_id.clone(), secret.clone());
            let result = handler.get_ntor_shared_secret(session_id);

            assert!(result.is_ok());
            assert_eq!(result.unwrap(), secret);
        }
    }

    mod test_handle_init_tunnel {
        use pingora::http::StatusCode;
        use pingora_router::ctx::{Layer8Context, Layer8ContextTrait};
        use pingora_router::handler::{RequestBodyTrait, ResponseBodyTrait};
        use reverse_proxy::config::{HandlerConfig, LogConfig, ProxyConfig, RPConfig, ServerConfig};
        use reverse_proxy::handler::init_tunnel::{InitEncryptedTunnelRequest, InitEncryptedTunnelResponse};
        use reverse_proxy::handler::{InMemorySecretsStorage, ReverseHandler};
        use serde_json::json;
        use utils::cert::TLSConfig;

        fn create_test_handler() -> (ReverseHandler, RPConfig) {
            let config = RPConfig {
                log: LogConfig {
                    log_level: "".to_string(),
                    log_format: "".to_string(),
                    log_path: "".to_string(),
                    log_filename: "".to_string(),
                },
                server: ServerConfig { listen_address: "".to_string(), listen_port: 0 },
                proxy: ProxyConfig {
                    tls: TLSConfig {
                        enable_tls: false,
                        ca_path: "".to_string(),
                        cert_path: "".to_string(),
                        key_path: "".to_string(),
                    },
                    cors_allow_credentials: false,
                    cors_allow_origins: vec![],
                },
                handler: HandlerConfig {
                    ntor_server_id: "test_server_id".to_string(),
                    ntor_static_secret: <[u8; 32]>::try_from("this is 32-byte nTorStaticSecret".as_bytes()).unwrap(),
                    jwt_virtual_connection_secret: b"test_secret".to_vec(),
                    jwt_exp_in_hours: 24,
                    backend_url: "http://localhost:8080".to_string(),
                },
            };
            (ReverseHandler::new(config.clone()), config)
        }

        #[tokio::test]
        async fn valid_request_body() {
            let (handler, config) = create_test_handler();
            let mut ctx = Layer8Context::default();
            let body = InitEncryptedTunnelRequest {
                public_key: vec![0u8; 32],
            };
            ctx.set_request_body(body.to_bytes());

            let response = handler.handle_init_tunnel(&mut ctx).await;
            // Verify response status and body
            assert_eq!(response.status, StatusCode::OK);
            assert!(response.body.is_some());

            // Verify response body can be parsed into InitEncryptedTunnelResponse
            let response_body =
                InitEncryptedTunnelResponse::from_bytes(response.body.unwrap());
            assert!(response_body.is_ok());

            // Verify response body fields
            let response_data = response_body.unwrap();
            assert_eq!(response_data.public_key.len(), 32);
            assert_eq!(response_data.t_b_hash.len(), 32);
            assert!(!response_data.int_rp_jwt.is_empty());
            assert!(!response_data.fp_rp_jwt.is_empty());

            // Verify JWT tokens can be decoded and contain expected claims
            let int_rp_jwt = utils::jwt::verify_jwt_token(
                &response_data.int_rp_jwt,
                &config.handler.jwt_virtual_connection_secret,
            ).expect("INT_RP_JWT token verification failed");
            let session_id = int_rp_jwt.claims.ntor_session_id;
            assert!(session_id.is_some());

            // Verify shared secret is stored in InMemorySecretsStorage with correct session ID
            let shared_secret = InMemorySecretsStorage::get(session_id.unwrap()).expect("Shared secret not found in storage for valid session ID");
            assert_eq!(shared_secret.len(), 16);
        }

        #[tokio::test]
        async fn invalid_request_body_cases() {
            let (handler, _) = create_test_handler();
            let mut ctx = Layer8Context::default();

            let cases = vec![
                (
                    "Invalid public key length, 31 bytes",
                    json!({
                        "public_key": b"a".repeat(31)
                    }).to_string().into_bytes(),
                ),
                (
                    "Invalid public key length: 33 bytes",
                    json!({
                        "public_key": b"a".repeat(33)
                    }).to_string().into_bytes(),
                ),
                (
                    "Invalid public key length: 0 bytes",
                    json!({
                        "public_key": b"a".repeat(0)
                    }).to_string().into_bytes(),
                ),
                (
                    "Invalid public key length: very large byte array",
                    json!({
                        "public_key": b"a".repeat(100000000)
                    }).to_string().into_bytes(),
                ),
                (
                    "public_key is not a byte array",
                    json!({
                        "public_key": "not a byte array"
                    }).to_string().into_bytes(),
                ),
                (
                    "public_key is a number",
                    json!({
                        "public_key": 12345
                    }).to_string().into_bytes(),
                ),
                (
                    "public_key is null",
                    json!({
                        "public_key": null
                    }).to_string().into_bytes(),
                ),
                (
                    "public_key is an object",
                    json!({
                        "public_key": {}
                    }).to_string().into_bytes(),
                ),
                (
                    "public_key is an array",
                    json!({
                        "public_key": []
                    }).to_string().into_bytes(),
                ),
                (
                    "public_key field is missing",
                    json!({
                        "not_public_key": "a".repeat(32)
                    }).to_string().into_bytes(),
                ),
                (
                    "body is not valid JSON",
                    b"invalid json".to_vec(),
                )
            ];

            for (test_case, body) in cases {
                println!("Running test case: {}", test_case);
                ctx.set_request_body(body);

                let response = handler.handle_init_tunnel(&mut ctx).await;

                assert_eq!(response.status, StatusCode::BAD_REQUEST);
                assert!(response.body.is_some());
            }
        }
    }

    mod test_handle_proxy_request {
        use std::net::SocketAddr;
        use axum::http::header::SET_COOKIE;
        use axum::http::{HeaderMap, HeaderValue};
        use axum::{Json, Router};
        use axum::response::IntoResponse;
        use axum::routing::{get};
        use ntor::common::EncryptedMessage;
        use pingora::http::StatusCode;
        use pingora_router::ctx::{Layer8Context, Layer8ContextTrait};
        use pingora_router::handler::ResponseBodyTrait;
        use reverse_proxy::config::RPConfig;
        use reverse_proxy::handler::{InMemorySecretsStorage, ReverseHandler};
        use reverse_proxy::handler::common::types::ErrorResponse;
        use utils::jwt::JWTClaims;

        const MOCK_BACKEND_URL: &str = "http://localhost:3004";
        const MOCK_BACKEND_PORT: u16 = 3004;
        pub const VALID_JWT_SECRET: &[u8] = b"test_valid_jwt_secret";
        pub const INVALID_JWT_SECRET: &[u8] = b"test_invalid_jwt_secret";
        const MOCK_SESSION_ID_1: &str = "d92db61d-e8d8-4f91-9ab4-b9fa9c53e65c";
        const MOCK_SESSION_ID_2: &str = "a3f5c8e7-1b2c-4d6e-9f8a-0b1c2d3e4f5g";
        const MOCK_SHARED_SECRET_1: [u8; 16] = [245, 239, 74, 167, 84, 191, 140, 194, 16, 59, 154, 244, 108, 221, 148, 85];
        const MOCK_SHARED_SECRET_2: [u8; 16] = [226, 186, 202, 159, 51, 31, 151, 34, 174, 233, 128, 164, 202, 226, 146, 91];
        // api GET /me
        const MOCK_PROXY_REQUEST_BODY_1: [u8; 80] = [159, 207, 157, 116, 32, 92, 248, 78, 122, 253, 236, 125, 67, 223, 157, 30, 30, 45, 49, 165, 234, 211, 72, 242, 252, 31, 128, 60, 245, 158, 182, 126, 117, 152, 232, 172, 52, 155, 246, 122, 86, 89, 78, 162, 110, 171, 73, 84, 127, 41, 195, 46, 85, 31, 71, 121, 234, 63, 27, 236, 43, 190, 186, 124, 94, 212, 238, 13, 254, 32, 147, 59, 239, 30, 176, 138, 54, 167, 161, 132];
        // api GET /profile/test
        const MOCK_PROXY_REQUEST_BODY_2: [u8; 90] = [210, 33, 207, 169, 246, 8, 233, 118, 37, 197, 180, 162, 77, 165, 168, 70, 128, 112, 244, 122, 187, 27, 188, 245, 126, 167, 152, 194, 233, 6, 37, 95, 101, 101, 247, 243, 37, 221, 51, 101, 23, 95, 2, 28, 123, 161, 251, 79, 193, 18, 75, 19, 204, 130, 106, 149, 30, 170, 91, 8, 218, 17, 212, 12, 130, 29, 187, 109, 187, 30, 42, 25, 187, 152, 171, 75, 78, 99, 74, 47, 61, 196, 224, 108, 107, 217, 220, 32, 187, 70];
        const INT_RP_JWT_HEADER: &str = "int_rp_jwt";
        const FP_RP_JWT_HEADER: &str = "fp_rp_jwt";

        fn create_int_rp_jwt_1(secret: &[u8], expiry_hrs: i64) -> String {
            let mut claims = JWTClaims::new(Some(expiry_hrs));
            claims.ntor_session_id = Some(MOCK_SESSION_ID_1.to_string());
            utils::jwt::create_jwt_token(claims, secret)
        }

        fn create_int_rp_jwt_2(secret: &[u8], expiry_hrs: i64) -> String {
            let mut claims = JWTClaims::new(Some(expiry_hrs));
            claims.ntor_session_id = Some(MOCK_SESSION_ID_2.to_string());
            utils::jwt::create_jwt_token(claims, secret)
        }

        fn create_fp_rp_jwt(secret: &[u8], expiry_hrs: i64) -> String {
            let claims = JWTClaims::new(Some(expiry_hrs));
            utils::jwt::create_jwt_token(claims, secret)
        }

        pub fn create_test_handler() -> ReverseHandler {
            let mut config = RPConfig::default();
            config.handler.jwt_virtual_connection_secret = VALID_JWT_SECRET.to_vec();
            config.handler.backend_url = MOCK_BACKEND_URL.to_string();
            ReverseHandler::new(config)
        }

        #[derive(Debug, serde::Deserialize)]
        struct RequestBody;

        #[derive(Debug, serde::Serialize)]
        struct ResponseBody {
            success: bool,
        }

        async fn test_api_get_me(_headers: HeaderMap) -> impl IntoResponse {
            let mut response_headers = HeaderMap::new();

            response_headers.insert(
                SET_COOKIE,
                HeaderValue::from_static(
                    "session_id=abc123; HttpOnly; Path=/"
                ),
            );

            (
                axum::http::StatusCode::OK,
                response_headers,
                Json(ResponseBody {
                    success: true,
                }),
            )
        }

        async fn test_api_get_profile(_headers: HeaderMap) -> impl IntoResponse {
            (
                axum::http::StatusCode::OK,
                Json(ResponseBody {
                    success: true,
                }),
            )
        }

        fn run_mock_be() {
            let mut app = Router::new();
            app = app.route("/me", get(test_api_get_me));
            app = app.route("/profile/test", get(test_api_get_profile));

            let addr = SocketAddr::from(([127, 0, 0, 1], MOCK_BACKEND_PORT));
            println!("Mock upstream server listening on {:?}", addr.clone());

            // Run server in background
            tokio::spawn(async move {
                axum::Server::bind(&addr)
                    .serve(app.into_make_service())
                    .await
                    .unwrap();
            });
        }

        #[tokio::test]
        async fn test_success() {
            run_mock_be();
            let handler = create_test_handler();
            InMemorySecretsStorage::insert(MOCK_SESSION_ID_1.to_string(), MOCK_SHARED_SECRET_1.to_vec());
            InMemorySecretsStorage::insert(MOCK_SESSION_ID_2.to_string(), MOCK_SHARED_SECRET_2.to_vec());

            let valid_int_rp_jwt_1 = create_int_rp_jwt_1(VALID_JWT_SECRET, 24);
            let valid_int_rp_jwt_2 = create_int_rp_jwt_2(VALID_JWT_SECRET, 24);
            let valid_fp_rp_jwt = create_fp_rp_jwt(VALID_JWT_SECRET, 24);

            // we don't need to test setting cookie in request header because the backend will
            // check it and whether it's valid or not, the result is encrypted
            let cases = vec![
                (
                    "Valid JWTs with session ID 1 and cookie set in response",
                    (valid_int_rp_jwt_1.clone(), valid_fp_rp_jwt.clone()),
                    MOCK_PROXY_REQUEST_BODY_1.to_vec(),
                    Some("session_id=abc123; HttpOnly; Path=/".to_string()),
                ),
                (
                    "Valid JWTs with session ID 2 and NO cookie set in response",
                    (valid_int_rp_jwt_2, valid_fp_rp_jwt.clone()),
                    MOCK_PROXY_REQUEST_BODY_2.to_vec(),
                    None,
                )
            ];

            for (test_case, (int_rp_jwt, fp_rp_jwt), request_body, expected_cookie) in cases {
                println!("Running test case: {}", test_case);
                let mut ctx = Layer8Context::default();
                ctx.set_request_body(request_body);

                ctx.insert_request_header(INT_RP_JWT_HEADER, &int_rp_jwt);
                ctx.insert_request_header(FP_RP_JWT_HEADER, &fp_rp_jwt);

                let response = handler.handle_proxy_request(&mut ctx).await;
                assert_eq!(response.status, StatusCode::OK);

                if expected_cookie.is_none() {
                    assert!(response.cookies.is_none(), "Cookies should NOT be set in the response");
                } else {
                    assert!(response.cookies.is_some(), "Cookies should be set in the response");
                    assert_eq!(response.cookies, expected_cookie, "Response should contain the expected Set-Cookie header");

                    let response_body = response.body.expect("Response body should be present");
                    let res_body: EncryptedMessage = utils::bincode_to_type(&response_body).expect("Response body should be a valid EncryptedMessage");
                    assert!(!res_body.nonce.is_empty(), "Nonce should not be empty");
                    assert!(!res_body.data.is_empty(), "Ciphertext should not be empty");
                }
            }
        }

        #[tokio::test]
        async fn test_invalid_tokens() {
            // running the mock backend is not necessary for this test since we are testing JWT
            // validation before any backend call, but to be fair to the test, we want to have the
            // backend running to ensure any failures are due to JWT validation and not backend connectivity issues
            run_mock_be();
            let handler = create_test_handler();
            InMemorySecretsStorage::insert(MOCK_SESSION_ID_1.to_string(), MOCK_SHARED_SECRET_1.to_vec());

            let valid_int_rp_jwt = create_int_rp_jwt_1(VALID_JWT_SECRET, 24);
            let valid_fp_rp_jwt = create_fp_rp_jwt(VALID_JWT_SECRET, 24);
            let invalid_signature_int_rp_jwt = create_int_rp_jwt_1(INVALID_JWT_SECRET, 24);
            let invalid_signature_fp_rp_jwt = create_fp_rp_jwt(INVALID_JWT_SECRET, 24);
            let expired_int_rp_jwt = create_int_rp_jwt_1(VALID_JWT_SECRET, -1);
            let expired_fp_rp_jwt = create_fp_rp_jwt(VALID_JWT_SECRET, -1);
            let int_rp_jwt_without_session_id = create_fp_rp_jwt(VALID_JWT_SECRET, 24); // Reusing FP JWT creation since it doesn't include session ID and has same secret and expiry
            let int_rp_jwt_with_invalid_session_id = {
                let mut claims = JWTClaims::new(Some(24));
                claims.ntor_session_id = Some("invalid_session_id".to_string());
                utils::jwt::create_jwt_token(claims, VALID_JWT_SECRET)
            };

            let cases = {
                vec![
                    (
                        "Empty header",
                        vec![]
                    ),
                    (
                        "Only valid int_rp_jwt",
                        vec![(INT_RP_JWT_HEADER, valid_int_rp_jwt.clone())]
                    ),
                    (
                        "Only valid fp_rp_jwt",
                        vec![(FP_RP_JWT_HEADER, valid_fp_rp_jwt.clone())]
                    ),
                    (
                        "Invalid int_rp_jwt format, valid fp_rp_jwt",
                        vec![
                            (INT_RP_JWT_HEADER, "invalid_format_token".to_string()),
                            (FP_RP_JWT_HEADER, valid_fp_rp_jwt.clone())
                        ]
                    ),
                    (
                        "Invalid fp_rp_jwt format, valid int_rp_jwt",
                        vec![
                            (INT_RP_JWT_HEADER, valid_int_rp_jwt.clone()),
                            (FP_RP_JWT_HEADER, "invalid_format_token".to_string())
                        ]
                    ),
                    (
                        "Valid JWTs but missing session ID in int_rp_jwt, valid fp_rp_jwt",
                        vec![
                            (INT_RP_JWT_HEADER, int_rp_jwt_without_session_id),
                            (FP_RP_JWT_HEADER, valid_fp_rp_jwt.clone())
                        ]
                    ),
                    (
                        "Valid JWTs but invalid session ID in int_rp_jwt, valid fp_rp_jwt",
                        vec![
                            (INT_RP_JWT_HEADER, int_rp_jwt_with_invalid_session_id),
                            (FP_RP_JWT_HEADER, valid_fp_rp_jwt.clone())
                        ]
                    ),
                    (
                        "Expired int_rp_jwt, valid fp_rp_jwt",
                        vec![
                            (INT_RP_JWT_HEADER, expired_int_rp_jwt.clone()),
                            (FP_RP_JWT_HEADER, valid_fp_rp_jwt.clone())
                        ]
                    ),
                    (
                        "Expired fp_rp_jwt",
                        vec![
                            (INT_RP_JWT_HEADER, valid_int_rp_jwt.clone()),
                            (FP_RP_JWT_HEADER, expired_fp_rp_jwt.clone())
                        ]
                    ),
                    (
                        "Invalid signature in int_rp_jwt",
                        vec![
                            (INT_RP_JWT_HEADER, invalid_signature_int_rp_jwt.clone()),
                            (FP_RP_JWT_HEADER, valid_fp_rp_jwt)
                        ]
                    ),
                    (
                        "Invalid signature in fp_rp_jwt",
                        vec![
                            (INT_RP_JWT_HEADER, valid_int_rp_jwt),
                            (FP_RP_JWT_HEADER, invalid_signature_fp_rp_jwt.clone())
                        ]
                    ),
                    (
                        "Both JWTs invalid signature",
                        vec![
                            (INT_RP_JWT_HEADER, invalid_signature_int_rp_jwt),
                            (FP_RP_JWT_HEADER, invalid_signature_fp_rp_jwt)
                        ]
                    ),
                    (
                        "Both JWTs expired",
                        vec![
                            (INT_RP_JWT_HEADER, expired_int_rp_jwt),
                            (FP_RP_JWT_HEADER, expired_fp_rp_jwt)
                        ]
                    ),
                    (
                        "Both JWTs invalid format",
                        vec![
                            (INT_RP_JWT_HEADER, "invalid_format_token".to_string()),
                            (FP_RP_JWT_HEADER, "invalid_format_token".to_string())
                        ]
                    ),
                ]
            };

            for (test_case, headers) in cases {
                println!("Running test case: {}", test_case);
                let mut ctx = Layer8Context::default();
                ctx.set_request_body(MOCK_PROXY_REQUEST_BODY_1.to_vec());

                for (header_name, header_value) in headers {
                    ctx.insert_request_header(header_name, &header_value);
                }

                let response = handler.handle_proxy_request(&mut ctx).await;
                assert_eq!(response.status, StatusCode::UNAUTHORIZED);
            }
        }

        #[tokio::test]
        async fn test_invalid_body() {
            run_mock_be();
            let handler = create_test_handler();
            InMemorySecretsStorage::insert(MOCK_SESSION_ID_1.to_string(), MOCK_SHARED_SECRET_1.to_vec());

            let valid_int_rp_jwt = create_int_rp_jwt_1(VALID_JWT_SECRET, 24);
            let valid_fp_rp_jwt = create_fp_rp_jwt(VALID_JWT_SECRET, 24);

            let mut ctx = Layer8Context::default();
            ctx.insert_request_header(INT_RP_JWT_HEADER, &valid_int_rp_jwt);
            ctx.insert_request_header(FP_RP_JWT_HEADER, &valid_fp_rp_jwt);

            #[derive(Debug, bincode::Encode, bincode::Decode)]
            struct InvalidEncryptedMessage {
                nonce: Vec<u8>,
                data: Vec<u8>,
            }

            #[derive(Debug, bincode::Encode, bincode::Decode)]
            struct ExtraFieldEncryptedMessage {
                nonce: Vec<u8>,
                data: Vec<u8>,
                extra_field: String,
            }

            #[derive(Debug, bincode::Encode, bincode::Decode)]
            struct MissingNonceEncryptedMessage {
                iv: Vec<u8>,
                data: Vec<u8>,
            }

            #[derive(Debug, bincode::Encode, bincode::Decode)]
            struct MissingDataEncryptedMessage {
                nonce: Vec<u8>,
                ciphertext: Vec<u8>,
            }

            #[derive(Debug, bincode::Encode, bincode::Decode)]
            struct WrongTypeNonceEncryptedMessage {
                nonce: String, // should be Vec<u8>
                data: Vec<u8>,
            }

            #[derive(Debug, bincode::Encode, bincode::Decode)]
            struct WrongTypeDataEncryptedMessage {
                nonce: Vec<u8>,
                data: String, // should be Vec<u8>
            }

            #[derive(Debug, bincode::Encode, bincode::Decode)]
            struct WrongTypesEncryptedMessage {
                nonce: String, // should be Vec<u8>
                data: String, // should be Vec<u8>
            }

            const EXPECTED_PARSING_ERR_STR: &str = "Error parsing request body: ";
            const EXPECTED_DECRYPTION_ERR_STR: &str = "Decryption failed: ";

            let cases = vec![
                ("Empty body", vec![], "Error parsing request body: "),
                (
                    "Body that is not valid bincode",
                    b"not a valid bincode".to_vec(),
                    EXPECTED_PARSING_ERR_STR
                ),
                (
                    "Body that is valid bincode but not a valid EncryptedMessage",
                    utils::type_to_bincode(&"just a string, not an EncryptedMessage"),
                    EXPECTED_PARSING_ERR_STR
                ),
                (
                    "Body is valid bincode but not a valid EncryptedMessage because nonce is wrong length",
                    utils::type_to_bincode(&InvalidEncryptedMessage {
                        nonce: vec![0u8; 10], // should be 12 bytes
                        data: vec![1, 2, 3],
                    }),
                    EXPECTED_DECRYPTION_ERR_STR
                ),
                (
                    "Body that is valid bincode but nonce is missing",
                    utils::type_to_bincode(&MissingNonceEncryptedMessage {
                        iv: vec![0u8; 12], // should be nonce field
                        data: vec![1, 2, 3],
                    }),
                    EXPECTED_DECRYPTION_ERR_STR
                ),
                (
                    "Body that is valid bincode and data is missing",
                    utils::type_to_bincode(&MissingDataEncryptedMessage {
                        nonce: vec![0u8; 12],
                        ciphertext: vec![1, 2, 3], // should be data field
                    }),
                    EXPECTED_DECRYPTION_ERR_STR
                ),
                (
                    "Body that is valid bincode and valid EncryptedMessage but nonce is not a byte array",
                    utils::type_to_bincode(&WrongTypeNonceEncryptedMessage {
                        nonce: "this should be a byte array".to_string(),
                        data: vec![1, 2, 3],
                    }),
                    EXPECTED_PARSING_ERR_STR
                ),
                (
                    "Body that is valid bincode and valid EncryptedMessage but data is not a byte array",
                    utils::type_to_bincode(&WrongTypeDataEncryptedMessage {
                        nonce: vec![0u8; 12],
                        data: "not a byte array".to_string() // parsing is still valid because of bincode deserialization, but decryption should fail because data is not an encrypted byte array
                    }),
                    EXPECTED_DECRYPTION_ERR_STR
                ),
                (
                    "Body that is valid bincode but has wrong field types",
                    utils::type_to_bincode(&WrongTypesEncryptedMessage {
                        nonce: "this should be a byte array".to_string(),
                        data: "this should be a byte array".to_string()
                    }),
                    EXPECTED_DECRYPTION_ERR_STR
                ),
                (
                    "Body that is valid bincode but has extra fields",
                    utils::type_to_bincode(&ExtraFieldEncryptedMessage {
                        nonce: vec![],
                        data: vec![],
                        extra_field: "this should be ignored".to_string(),
                    }),
                    EXPECTED_PARSING_ERR_STR
                ),
                (
                    "Body that is valid bincode and valid EncryptedMessage but decryption fails due to ciphertext was signed with different secret",
                    MOCK_PROXY_REQUEST_BODY_2.to_vec(), // we're using shared secret 1 in the test, but this ciphertext was generated with shared secret 2, so decryption should fail
                    EXPECTED_DECRYPTION_ERR_STR
                ),
                // add internal server error cases ?
            ];

            for (test_case, body, expected_err_str) in cases {
                println!("Running test case: {}", test_case);
                ctx.set_request_body(body);

                let response = handler.handle_proxy_request(&mut ctx).await;

                assert_eq!(response.status, StatusCode::BAD_REQUEST);

                let response_body = response.body.expect("Response body should be present");
                let err_response = ErrorResponse::from_bytes(response_body).expect("Response body should be a valid ErrorResponse");
                assert!(err_response.error.contains(expected_err_str));
            }
        }
    }
}