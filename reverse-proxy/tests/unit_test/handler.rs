#[path = "../mock/mod.rs"]
mod mock;

#[cfg(test)]
mod test_handler {
    mod test_get_ntor_shared_secret {

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
                server: ServerConfig {
                    listen_address: "".to_string(),
                    listen_port: 0,
                },
            };

            ReverseHandler::new(rp_config)
        }

        #[test]
        fn test_get_ntor_shared_secret_invalid_session() {
            let handler = create_test_handler();
            let result = handler.get_ntor_shared_secret("invalid_session_id");

            assert!(result.is_err());
            let err_response = result.unwrap_err();
            assert_eq!(err_response, "Session ID not found".to_string());
        }

        #[test]
        fn test_get_ntor_shared_secret_valid_session() {
            let handler = create_test_handler();
            let session_id = "test_session_123".to_string();
            let secret = vec![1, 2, 3, 4, 5];

            InMemorySecretsStorage::insert(session_id.clone(), secret.clone());
            let result = handler.get_ntor_shared_secret(&session_id);

            assert!(result.is_ok());
            assert_eq!(result.unwrap(), secret);
        }
    }

    mod test_handle_init_tunnel {
        use pingora::http::StatusCode;
        use pingora_router::ctx::{Layer8Context, Layer8ContextTrait};
        use pingora_router::handler::{RequestBodyTrait, ResponseBodyTrait};
        use reverse_proxy::config::{
            HandlerConfig, LogConfig, ProxyConfig, RPConfig, ServerConfig,
        };
        use reverse_proxy::handler::init_tunnel::{
            InitEncryptedTunnelRequest, InitEncryptedTunnelResponse,
        };
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
                server: ServerConfig {
                    listen_address: "".to_string(),
                    listen_port: 0,
                },
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
                    ntor_static_secret: <[u8; 32]>::try_from(
                        "this is 32-byte nTorStaticSecret".as_bytes(),
                    )
                    .unwrap(),
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
            let response_body = InitEncryptedTunnelResponse::from_bytes(response.body.unwrap());
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
            )
            .expect("INT_RP_JWT token verification failed");
            let session_id = int_rp_jwt.claims.ntor_session_id;
            assert!(session_id.is_some());

            // Verify shared secret is stored in InMemorySecretsStorage with correct session ID
            let shared_secret = InMemorySecretsStorage::get(&session_id.unwrap())
                .expect("Shared secret not found in storage for valid session ID");
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
                    })
                    .to_string()
                    .into_bytes(),
                ),
                (
                    "Invalid public key length: 33 bytes",
                    json!({
                        "public_key": b"a".repeat(33)
                    })
                    .to_string()
                    .into_bytes(),
                ),
                (
                    "Invalid public key length: 0 bytes",
                    json!({
                        "public_key": b"a".repeat(0)
                    })
                    .to_string()
                    .into_bytes(),
                ),
                (
                    "Invalid public key length: very large byte array",
                    json!({
                        "public_key": b"a".repeat(100000)
                    })
                    .to_string()
                    .into_bytes(),
                ),
                (
                    "public_key is not a byte array",
                    json!({
                        "public_key": "not a byte array"
                    })
                    .to_string()
                    .into_bytes(),
                ),
                (
                    "public_key is a number",
                    json!({
                        "public_key": 12345
                    })
                    .to_string()
                    .into_bytes(),
                ),
                (
                    "public_key is null",
                    json!({
                        "public_key": null
                    })
                    .to_string()
                    .into_bytes(),
                ),
                (
                    "public_key is an object",
                    json!({
                        "public_key": {}
                    })
                    .to_string()
                    .into_bytes(),
                ),
                (
                    "public_key is an array",
                    json!({
                        "public_key": []
                    })
                    .to_string()
                    .into_bytes(),
                ),
                (
                    "public_key field is missing",
                    json!({
                        "not_public_key": "a".repeat(32)
                    })
                    .to_string()
                    .into_bytes(),
                ),
                ("body is not valid JSON", b"invalid json".to_vec()),
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
        use crate::mock;
        use crate::mock::data::{
            FP_RP_JWT_HEADER, INT_RP_JWT_HEADER, INVALID_JWT_SECRET, MOCK_BACKEND_URL,
            MOCK_PROXY_REQUEST_BODY_1, MOCK_PROXY_REQUEST_BODY_2, MOCK_SESSION_ID_1,
            MOCK_SESSION_ID_2, MOCK_SHARED_SECRET_1, MOCK_SHARED_SECRET_2, VALID_JWT_SECRET,
            create_fp_rp_jwt, create_int_rp_jwt_1, create_int_rp_jwt_2,
        };
        use ntor::common::EncryptedMessage;
        use pingora::http::StatusCode;
        use pingora_router::ctx::{Layer8Context, Layer8ContextTrait};
        use pingora_router::handler::ResponseBodyTrait;
        use reverse_proxy::config::RPConfig;
        use reverse_proxy::handler::common::types::ErrorResponse;
        use reverse_proxy::handler::{InMemorySecretsStorage, ReverseHandler};
        use utils::jwt::JWTClaims;

        pub fn create_test_handler() -> ReverseHandler {
            let mut config = RPConfig::default();
            config.handler.jwt_virtual_connection_secret = VALID_JWT_SECRET.to_vec();
            config.handler.backend_url = MOCK_BACKEND_URL.to_string();
            ReverseHandler::new(config)
        }

        #[tokio::test]
        async fn test_success() {
            mock::backend::run_mock_be();
            let handler = create_test_handler();
            InMemorySecretsStorage::insert(
                MOCK_SESSION_ID_1.to_string(),
                MOCK_SHARED_SECRET_1.to_vec(),
            );
            InMemorySecretsStorage::insert(
                MOCK_SESSION_ID_2.to_string(),
                MOCK_SHARED_SECRET_2.to_vec(),
            );

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
                ),
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
                    assert!(
                        response.cookies.is_none(),
                        "Cookies should NOT be set in the response"
                    );
                } else {
                    assert!(
                        response.cookies.is_some(),
                        "Cookies should be set in the response"
                    );
                    assert_eq!(
                        response.cookies, expected_cookie,
                        "Response should contain the expected Set-Cookie header"
                    );

                    let response_body = response.body.expect("Response body should be present");
                    let res_body: EncryptedMessage = utils::bincode_to_type(&response_body)
                        .expect("Response body should be a valid EncryptedMessage");
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
            mock::backend::run_mock_be();
            let handler = create_test_handler();
            InMemorySecretsStorage::insert(
                MOCK_SESSION_ID_1.to_string(),
                MOCK_SHARED_SECRET_1.to_vec(),
            );

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
                    ("Empty header", vec![]),
                    (
                        "Only valid int_rp_jwt",
                        vec![(INT_RP_JWT_HEADER, valid_int_rp_jwt.clone())],
                    ),
                    (
                        "Only valid fp_rp_jwt",
                        vec![(FP_RP_JWT_HEADER, valid_fp_rp_jwt.clone())],
                    ),
                    (
                        "Invalid int_rp_jwt format, valid fp_rp_jwt",
                        vec![
                            (INT_RP_JWT_HEADER, "invalid_format_token".to_string()),
                            (FP_RP_JWT_HEADER, valid_fp_rp_jwt.clone()),
                        ],
                    ),
                    (
                        "Invalid fp_rp_jwt format, valid int_rp_jwt",
                        vec![
                            (INT_RP_JWT_HEADER, valid_int_rp_jwt.clone()),
                            (FP_RP_JWT_HEADER, "invalid_format_token".to_string()),
                        ],
                    ),
                    (
                        "Valid JWTs but missing session ID in int_rp_jwt, valid fp_rp_jwt",
                        vec![
                            (INT_RP_JWT_HEADER, int_rp_jwt_without_session_id),
                            (FP_RP_JWT_HEADER, valid_fp_rp_jwt.clone()),
                        ],
                    ),
                    (
                        "Valid JWTs but invalid session ID in int_rp_jwt, valid fp_rp_jwt",
                        vec![
                            (INT_RP_JWT_HEADER, int_rp_jwt_with_invalid_session_id),
                            (FP_RP_JWT_HEADER, valid_fp_rp_jwt.clone()),
                        ],
                    ),
                    (
                        "Expired int_rp_jwt, valid fp_rp_jwt",
                        vec![
                            (INT_RP_JWT_HEADER, expired_int_rp_jwt.clone()),
                            (FP_RP_JWT_HEADER, valid_fp_rp_jwt.clone()),
                        ],
                    ),
                    (
                        "Expired fp_rp_jwt",
                        vec![
                            (INT_RP_JWT_HEADER, valid_int_rp_jwt.clone()),
                            (FP_RP_JWT_HEADER, expired_fp_rp_jwt.clone()),
                        ],
                    ),
                    (
                        "Invalid signature in int_rp_jwt",
                        vec![
                            (INT_RP_JWT_HEADER, invalid_signature_int_rp_jwt.clone()),
                            (FP_RP_JWT_HEADER, valid_fp_rp_jwt),
                        ],
                    ),
                    (
                        "Invalid signature in fp_rp_jwt",
                        vec![
                            (INT_RP_JWT_HEADER, valid_int_rp_jwt),
                            (FP_RP_JWT_HEADER, invalid_signature_fp_rp_jwt.clone()),
                        ],
                    ),
                    (
                        "Both JWTs invalid signature",
                        vec![
                            (INT_RP_JWT_HEADER, invalid_signature_int_rp_jwt),
                            (FP_RP_JWT_HEADER, invalid_signature_fp_rp_jwt),
                        ],
                    ),
                    (
                        "Both JWTs expired",
                        vec![
                            (INT_RP_JWT_HEADER, expired_int_rp_jwt),
                            (FP_RP_JWT_HEADER, expired_fp_rp_jwt),
                        ],
                    ),
                    (
                        "Both JWTs invalid format",
                        vec![
                            (INT_RP_JWT_HEADER, "invalid_format_token".to_string()),
                            (FP_RP_JWT_HEADER, "invalid_format_token".to_string()),
                        ],
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
            mock::backend::run_mock_be();
            let handler = create_test_handler();
            InMemorySecretsStorage::insert(
                MOCK_SESSION_ID_1.to_string(),
                MOCK_SHARED_SECRET_1.to_vec(),
            );

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
                data: String,  // should be Vec<u8>
            }

            const EXPECTED_PARSING_ERR_STR: &str = "Failed to parse request body";
            const EXPECTED_DECRYPTION_ERR_STR: &str = "Failed to decrypt request body";

            let cases = vec![
                ("Empty body", vec![], EXPECTED_PARSING_ERR_STR),
                (
                    "Body that is not valid bincode",
                    b"not a valid bincode".to_vec(),
                    EXPECTED_PARSING_ERR_STR,
                ),
                (
                    "Body that is valid bincode but not a valid EncryptedMessage",
                    utils::type_to_bincode(&"just a string, not an EncryptedMessage"),
                    EXPECTED_PARSING_ERR_STR,
                ),
                (
                    "Body is valid bincode but not a valid EncryptedMessage because nonce is wrong length",
                    utils::type_to_bincode(&InvalidEncryptedMessage {
                        nonce: vec![0u8; 10], // should be 12 bytes
                        data: vec![1, 2, 3],
                    }),
                    EXPECTED_DECRYPTION_ERR_STR,
                ),
                (
                    "Body that is valid bincode but nonce is missing",
                    utils::type_to_bincode(&MissingNonceEncryptedMessage {
                        iv: vec![0u8; 12], // should be nonce field
                        data: vec![1, 2, 3],
                    }),
                    EXPECTED_DECRYPTION_ERR_STR,
                ),
                (
                    "Body that is valid bincode and data is missing",
                    utils::type_to_bincode(&MissingDataEncryptedMessage {
                        nonce: vec![0u8; 12],
                        ciphertext: vec![1, 2, 3], // should be data field
                    }),
                    EXPECTED_DECRYPTION_ERR_STR,
                ),
                (
                    "Body that is valid bincode and valid EncryptedMessage but nonce is not a byte array",
                    utils::type_to_bincode(&WrongTypeNonceEncryptedMessage {
                        nonce: "this should be a byte array".to_string(),
                        data: vec![1, 2, 3],
                    }),
                    EXPECTED_PARSING_ERR_STR,
                ),
                (
                    "Body that is valid bincode and valid EncryptedMessage but data is not a byte array",
                    utils::type_to_bincode(&WrongTypeDataEncryptedMessage {
                        nonce: vec![0u8; 12],
                        data: "not a byte array".to_string(), // parsing is still valid because of bincode deserialization, but decryption should fail because data is not an encrypted byte array
                    }),
                    EXPECTED_DECRYPTION_ERR_STR,
                ),
                (
                    "Body that is valid bincode but has wrong field types",
                    utils::type_to_bincode(&WrongTypesEncryptedMessage {
                        nonce: "this should be a byte array".to_string(),
                        data: "this should be a byte array".to_string(),
                    }),
                    EXPECTED_DECRYPTION_ERR_STR,
                ),
                (
                    "Body that is valid bincode but has extra fields",
                    utils::type_to_bincode(&ExtraFieldEncryptedMessage {
                        nonce: vec![],
                        data: vec![],
                        extra_field: "this should be ignored".to_string(),
                    }),
                    EXPECTED_PARSING_ERR_STR,
                ),
                (
                    "Body that is valid bincode and valid EncryptedMessage but decryption fails due to ciphertext was signed with different secret",
                    MOCK_PROXY_REQUEST_BODY_2.to_vec(), // we're using shared secret 1 in the test, but this ciphertext was generated with shared secret 2, so decryption should fail
                    EXPECTED_DECRYPTION_ERR_STR,
                ),
                // add internal server error cases ?
            ];

            for (test_case, body, expected_err_str) in cases {
                println!("Running test case: {}", test_case);
                ctx.set_request_body(body);

                let response = handler.handle_proxy_request(&mut ctx).await;

                assert_eq!(response.status, StatusCode::BAD_REQUEST);

                let response_body = response.body.expect("Response body should be present");
                let err_response = ErrorResponse::from_bytes(response_body)
                    .expect("Response body should be a valid ErrorResponse");
                assert!(
                    err_response.error.contains(expected_err_str),
                    "Error message should contain the expected substring. Actual error message: {}",
                    err_response.error
                );
            }
        }

        // unable to connect to backend
    }
}
