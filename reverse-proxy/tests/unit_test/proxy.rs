#[path = "../mock/mod.rs"]
mod mock;

#[cfg(test)]
mod test_proxy_handler {
    use utils::jwt::JWTClaims;

    pub const INT_RP_JWT_HEADER: &str = "int_rp_jwt";
    pub const FP_RP_JWT_HEADER: &str = "fp_rp_jwt";
    pub const VALID_JWT_SECRET: &[u8] = b"test_valid_jwt_secret";
    // pub const INVALID_JWT_SECRET: &[u8] = b"test_invalid_jwt_secret";
    // const MOCK_SESSION_ID: &str = "d92db61d-e8d8-4f91-9ab4-b9fa9c53e65c";
    const MOCK_SHARED_SECRET: [u8; 16] = [245, 239, 74, 167, 84, 191, 140, 194, 16, 59, 154, 244, 108, 221, 148, 85];
    const MOCK_PROXY_REQUEST_BODY: [u8; 80] = [159, 207, 157, 116, 32, 92, 248, 78, 122, 253, 236, 125, 67, 223, 157, 30, 30, 45, 49, 165, 234, 211, 72, 242, 252, 31, 128, 60, 245, 158, 182, 126, 117, 152, 232, 172, 52, 155, 246, 122, 86, 89, 78, 162, 110, 171, 73, 84, 127, 41, 195, 46, 85, 31, 71, 121, 234, 63, 27, 236, 43, 190, 186, 124, 94, 212, 238, 13, 254, 32, 147, 59, 239, 30, 176, 138, 54, 167, 161, 132];
    const MOCK_NTOR_SERVER_ID: &str = "http://localhost:6193";
    const MOCK_ENCRYPTED_MESSAGE_NONCE: [u8; 12] = [159, 207, 157, 116, 32, 92, 248, 78, 122, 253, 236, 125];
    const MOCK_ENCRYPTED_MESSAGE_DATA: [u8; 67] = [223, 157, 30, 30, 45, 49, 165, 234, 211, 72, 242, 252, 31, 128, 60, 245, 158, 182, 126, 117, 152, 232, 172, 52, 155, 246, 122, 86, 89, 78, 162, 110, 171, 73, 84, 127, 41, 195, 46, 85, 31, 71, 121, 234, 63, 27, 236, 43, 190, 186, 124, 94, 212, 238, 13, 254, 32, 147, 59, 239, 30, 176, 138, 54, 167, 161, 132];

    fn create_int_rp_jwt(secret: &[u8], expiry_hrs: i64) -> (String, String) {
        let ntor_session_id = utils::new_uuid();

        let int_rp_jwt = {
            let mut claims = JWTClaims::new(Some(expiry_hrs));
            claims.ntor_session_id = Some(ntor_session_id.clone());
            utils::jwt::create_jwt_token(claims, secret)
        };

        (ntor_session_id, int_rp_jwt)
    }

    fn create_fp_rp_jwt(secret: &[u8], expiry_hrs: i64) -> String {
        let claims = JWTClaims::new(Some(expiry_hrs));
        utils::jwt::create_jwt_token(claims, secret)
    }

    mod test_validate_jwt_token {
        use pingora_router::ctx::{Layer8Context, Layer8ContextTrait};
        use reverse_proxy::handler::proxy::ProxyHandler;
        use crate::test_proxy_handler::{create_int_rp_jwt, VALID_JWT_SECRET};

        #[test]
        fn test_validate_jwt_token() {
            let mut ctx = Layer8Context::default();
            let invalid_jwt_secret = utils::new_uuid().into_bytes();

            let header_key = "int_rp_jwt"; // or any header key since the function takes it as a parameter
            let err_from_dependency = Some("dependent on jsonwebtoken::decode");
            let missing_header_err_body = Some("Missing int_rp_jwt header");
            let empty_header_err_body = Some("Empty int_rp_jwt header");

            let valid_token = create_int_rp_jwt(VALID_JWT_SECRET, 24).1;
            let invalid_secret_token = create_int_rp_jwt(&invalid_jwt_secret, 24).1;
            let expired_token = create_int_rp_jwt(VALID_JWT_SECRET, -1).1;

            let cases = vec![
                ("Missing header", header_key, None, missing_header_err_body),
                ("Empty header", header_key, Some(""), empty_header_err_body),
                ("Valid token", header_key, Some(&valid_token), None),
                ("Expired token", header_key, Some(&expired_token), err_from_dependency.clone()),
                ("Invalid signature token", header_key, Some(&invalid_secret_token), err_from_dependency.clone()),
            ];

            for (case_name, header_key, header_value, err_body) in cases {
                println!("Running case: {}", case_name);
                if let Some(value) = header_value {
                    ctx.insert_request_header(header_key, value);
                }

                let result = ProxyHandler::validate_jwt_token(&mut ctx, header_key, &VALID_JWT_SECRET.to_vec());

                if err_body == None && result.is_err() {
                    panic!("Expected success but got error: {:?}", result.unwrap_err());
                } else if err_body == None {
                    continue; // Success case, move to next case
                }

                assert!(result.is_err());
                match result.unwrap_err() {
                    err_response => {
                        if err_body != err_from_dependency.clone() {
                            assert_eq!(err_response, err_body.unwrap());
                        }
                    }
                }
            }
        }
    }

    mod test_validate_request_headers {
        use pingora_router::ctx::{Layer8Context, Layer8ContextTrait};
        use reverse_proxy::handler::proxy::ProxyHandler;
        use crate::test_proxy_handler::{create_fp_rp_jwt, create_int_rp_jwt, FP_RP_JWT_HEADER, INT_RP_JWT_HEADER, VALID_JWT_SECRET};

        #[test]
        fn test_validate_request_headers() {
            let (valid_ntor_session_id, valid_int_rp_jwt) = create_int_rp_jwt(VALID_JWT_SECRET, 24);
            let invalid_secret_int_rp_jwt = create_int_rp_jwt("invalid_jwt_secret".as_bytes(), 24).1;
            let expired_int_rp_jwt = create_int_rp_jwt(VALID_JWT_SECRET, -1).1;
            let missing_session_id_int_rp_jwt = create_fp_rp_jwt(VALID_JWT_SECRET, 24);

            let valid_fp_rp_jwt = create_fp_rp_jwt(VALID_JWT_SECRET, 24);
            let invalid_secret_fp_rp_jwt = create_fp_rp_jwt("invalid_jwt_secret".as_bytes(), 24);
            let expired_fp_rp_jwt = create_fp_rp_jwt(VALID_JWT_SECRET, -1);

            let err_cases = vec![
                ("Empty header", vec![]),
                ("Only valid `int_rp_jwt`", vec![(INT_RP_JWT_HEADER, valid_int_rp_jwt.as_str())]),
                ("Only valid `fp_rp_jwt`", vec![(FP_RP_JWT_HEADER, valid_fp_rp_jwt.as_str())]),
                ("Invalid `int_rp_jwt` format", vec![(INT_RP_JWT_HEADER, "this is not a jwt")]),
                ("Invalid `fp_rp_jwt` format", vec![(FP_RP_JWT_HEADER, "this is not a jwt")]),
                ("Valid JWTs but missing session ID in `int_rp_jwt`", vec![(INT_RP_JWT_HEADER, missing_session_id_int_rp_jwt.as_str())]),
                ("Invalid signature `int_rp_jwt`", vec![(INT_RP_JWT_HEADER, invalid_secret_int_rp_jwt.as_str())]),
                ("Invalid signature `fp_rp_jwt`", vec![(FP_RP_JWT_HEADER, invalid_secret_fp_rp_jwt.as_str())]),
                ("Expired int_rp_jwt", vec![(INT_RP_JWT_HEADER, expired_int_rp_jwt.as_str())]),
                ("Expired fp_rp_jwt", vec![(FP_RP_JWT_HEADER, expired_fp_rp_jwt.as_str())]),
            ];

            for (case_name, headers) in err_cases {
                println!("Running case: {}", case_name);
                let mut ctx = Layer8Context::default(); // Reset context for each case

                for (header_key, header_value) in headers {
                    ctx.insert_request_header(header_key, header_value);
                }

                let result = ProxyHandler::validate_request_headers(&mut ctx, &VALID_JWT_SECRET.to_vec());
                assert!(result.is_err());
            }

            let valid_cases = vec![
                (
                    "Valid `int_rp_jwt` and `fp_rp_jwt`",
                    vec![
                        (INT_RP_JWT_HEADER, valid_int_rp_jwt),
                        (FP_RP_JWT_HEADER, valid_fp_rp_jwt)
                    ],
                    valid_ntor_session_id.clone()
                ),
            ];

            for (case_name, headers, session_id) in valid_cases {
                println!("Running case: {}", case_name);
                let mut ctx = Layer8Context::default(); // Reset context for each case

                for (header_key, header_value) in headers {
                    ctx.insert_request_header(header_key, &header_value);
                }

                let result = ProxyHandler::validate_request_headers(&mut ctx, &VALID_JWT_SECRET.to_vec());
                assert!(result.is_ok());
                assert_eq!(session_id, result.unwrap())
            }
        }
    }

    mod test_parse_request_body {
        use pingora_router::ctx::{Layer8Context, Layer8ContextTrait};
        use reverse_proxy::handler::proxy::ProxyHandler;
        use crate::test_proxy_handler::MOCK_PROXY_REQUEST_BODY;

        #[test]
        fn test_valid() {
            let mut ctx = Layer8Context::default();

            let cases = vec![
                ("Valid encrypted message", MOCK_PROXY_REQUEST_BODY.to_vec()),
            ];

            for (case_name, body) in cases {
                println!("Running case: {}", case_name);
                ctx.set_request_body(body);

                let result = ProxyHandler::parse_request_body(&mut ctx);

                assert!(result.is_ok(), "Expected success but got error");
            }
        }

        #[test]
        fn test_invalid() {
            let mut ctx = Layer8Context::default();

            let cases = vec![
                ("Empty body", vec![]),
                ("Invalid bincode format", vec![0xFF, 0xFF, 0xFF]),
            ];


            for (case_name, body) in cases {
                println!("Running test case: {}", case_name);
                ctx.set_request_body(body.clone());

                let result = ProxyHandler::parse_request_body(&mut ctx);

                assert!(result.is_err(), "Expected error but got success");
            }
        }
    }

    mod test_decrypt_request_body {
        use ntor::common::EncryptedMessage;
        use reverse_proxy::handler::proxy::ProxyHandler;
        use crate::test_proxy_handler::{MOCK_ENCRYPTED_MESSAGE_DATA, MOCK_ENCRYPTED_MESSAGE_NONCE,
                                        MOCK_NTOR_SERVER_ID, MOCK_SHARED_SECRET};

        #[test]
        fn test_valid_input() {
            let encrypted_msg = EncryptedMessage {
                nonce: MOCK_ENCRYPTED_MESSAGE_NONCE,
                data: MOCK_ENCRYPTED_MESSAGE_DATA.to_vec(),
            };

            let cases = vec![
                ("Valid ntor server ID", MOCK_NTOR_SERVER_ID.to_string()),
                // Note: nTor serverID was required to create NTorServer instance, but it does not
                // affect the encryption/decryption process since the shared secret is already derived
                ("Invalid ntor server ID", "wrong ntor server id".to_string()),
            ];

            for (case_name, ntor_server_id) in cases {
                println!("Running case: {}", case_name);

                let result = ProxyHandler::decrypt_request_body(
                    encrypted_msg.clone(),
                    ntor_server_id,
                    &MOCK_SHARED_SECRET,
                );

                assert!(result.is_ok());
            }
        }

        #[test]
        fn test_invalid_input() {
            let empty_data = EncryptedMessage {
                nonce: [0u8; 12],
                data: vec![],
            };

            let invalid_encrypted_data = EncryptedMessage {
                nonce: MOCK_ENCRYPTED_MESSAGE_NONCE.clone(),
                data: b"a".repeat(67).to_vec(),
            };

            let wrong_shared_secret = EncryptedMessage {
                nonce: MOCK_ENCRYPTED_MESSAGE_NONCE.clone(),
                data: MOCK_ENCRYPTED_MESSAGE_DATA.to_vec(),
            };

            let cases = vec![
                ("Empty data", empty_data, MOCK_NTOR_SERVER_ID.to_string(), MOCK_SHARED_SECRET.to_vec()),
                ("Invalid encrypted data", invalid_encrypted_data, MOCK_NTOR_SERVER_ID.to_string(), MOCK_SHARED_SECRET.to_vec()),
                ("Invalid shared secret", wrong_shared_secret, MOCK_NTOR_SERVER_ID.to_string(), b"wrong_shared_secret".to_vec()),
            ];

            for (case_name, encrypted_msg, ntor_server_id, shared_secret) in cases {
                println!("Running case: {}", case_name);

                let result = ProxyHandler::decrypt_request_body(
                    encrypted_msg,
                    ntor_server_id,
                    &shared_secret,
                );

                assert!(result.is_err());
            }
        }
    }

    mod test_encrypt_response_body {
        use ntor::common::NTorParty;
        use ntor::server::NTorServer;
        use reverse_proxy::handler::proxy::{L8ResponseObject, ProxyHandler};
        use utils::bytes_to_json;
        use crate::test_proxy_handler::{MOCK_NTOR_SERVER_ID, MOCK_SHARED_SECRET};

        #[test]
        fn test_encrypt_response_body_success() {
            let mut response_body = L8ResponseObject {
                status: 200,
                status_text: "OK".to_string(),
                headers: Default::default(),
                body: vec![1, 2, 3, 4, 5],
                ok: true,
                url: "http://example.com".to_string(),
                redirected: false,
            };

            let cases = vec![
                ("Empty body", vec![]),
                ("Small body", vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10]),
                ("Large body", vec![1u8; 1000000]),
            ];

            for (case_name, body) in cases {
                println!("Running case: {}", case_name);
                response_body.body = body;


                let result = ProxyHandler::encrypt_response_body(
                    response_body.clone(),
                    MOCK_NTOR_SERVER_ID.to_string(),
                    &MOCK_SHARED_SECRET,
                );

                assert!(result.is_ok());
                let encrypted = result.unwrap();
                assert!(!encrypted.nonce.is_empty());
                assert!(!encrypted.data.is_empty());

                // Verify that the encrypted data can be decrypted back to the original response body
                let mut ntor_server = NTorServer::new(MOCK_NTOR_SERVER_ID.to_string());
                ntor_server.set_shared_secret(MOCK_SHARED_SECRET.to_vec());

                let decrypted = ntor_server.decrypt(encrypted);

                assert!(decrypted.is_ok());
                let decrypted_result = bytes_to_json::<L8ResponseObject>(decrypted.unwrap());
                assert!(decrypted_result.is_ok());
                let decrypted_data = decrypted_result.unwrap();
                assert_eq!(decrypted_data.status, response_body.status, "Decrypted status does not match original status");
                assert_eq!(decrypted_data, response_body, "Decrypted response body does not match original response body");
            }
        }
    }

    mod test_rebuild_user_request {
        use std::collections::HashMap;
        use pingora_router::ctx::{Layer8Context, Layer8ContextRequestSummary, Layer8ContextTrait};
        use reverse_proxy::handler::proxy::{L8RequestObject, ProxyHandler};
        use crate::mock;

        #[tokio::test]
        async fn test_rebuild_user_request() {
            mock::backend::run_mock_be();

            let summary = Layer8ContextRequestSummary {
                method: "POST".parse().unwrap(),
                scheme: "http".to_string(),
                host: format!("localhost:{}", mock::data::MOCK_BACKEND_PORT),
                path: "/test/api".to_string(),
                params: Default::default(),
            };
            let mut ctx = Layer8Context::default();
            ctx.set_request_summary(summary);
            ctx.insert_request_header("Content-Type", "application/json");

            let l8_request = L8RequestObject {
                method: "POST".parse().unwrap(),
                uri: "/test/api".to_string(),
                headers: HashMap::from([("Content-Type".to_string(), "application/json".into())]),
                body: b"{\"key\": \"value\"}".to_vec(),
            };

            let result =
                ProxyHandler::rebuild_user_request(&mut ctx, mock::data::MOCK_BACKEND_URL.to_string(), l8_request).await;
            assert!(result.is_ok());

            let (response, url) = result.unwrap();

            let l8_response = ProxyHandler::wrap_backend_response(&ctx, response, &url).await;

            assert_eq!(l8_response.status, 200);

            assert!(l8_response.headers.contains_key("set-cookie"), "Response headers should contain 'set-cookie'");
            let cookie = l8_response.headers.get("set-cookie").unwrap().to_string();
            assert_eq!("\"session_id=abc123; HttpOnly; Path=/; Max-Age=3600\"".to_string(), cookie, "Set-Cookie header does not match expected value");

            assert_eq!(l8_response.body, b"{\"success\":true}".to_vec(), "Response body does not match expected value");
        }
    }
}