#[cfg(test)]
mod test_init_tunnel_handler {
    mod test_validate_request_body {
        use pingora::http::StatusCode;
        use serde_json::json;
        use pingora_router::ctx::{Layer8Context, Layer8ContextTrait};
        use reverse_proxy::handler::init_tunnel::{InitEncryptedTunnelRequest, InitTunnelHandler};

        #[tokio::test]
        async fn test_validate_request_body_valid_public_key() {
            let body = json!({
                "public_key": b"a".repeat(32)
            }).to_string().into_bytes();

            let mut ctx = Layer8Context::default();
            ctx.set_request_body(body);

            match InitTunnelHandler::validate_request_body(&mut ctx).await {
                Ok(res) => {
                    assert_eq!(res, InitEncryptedTunnelRequest {
                        public_key: b"a".repeat(32)
                    }, "expected valid public key to be parsed successfully");
                }
                Err(_) => panic!("Expected success with valid public key"),
            }
        }

        #[tokio::test]
        async fn test_validate_request_body_invalid_public_key() {
            let mut ctx = Layer8Context::default();

            let cases = vec![
                (
                    "Public key oversize (33 bytes)",
                    json!({
                        "public_key": b"a".repeat(33)
                    }).to_string().into_bytes(),
                    Some("Invalid public key length".as_bytes().to_vec())
                ),
                (
                    "Public key undersize (31 bytes)",
                    json!({
                        "public_key": b"a".repeat(31)
                    }).to_string().into_bytes(),
                    Some("Invalid public key length".as_bytes().to_vec())
                ),
                (
                    "Public key empty (0 bytes)",
                    json!({
                        "public_key": b"a".repeat(0)
                    }).to_string().into_bytes(),
                    Some("Invalid public key length".as_bytes().to_vec())
                ),
                (
                    "Public key way oversize (100 million bytes)",
                    json!({
                        "public_key": b"a".repeat(100000000)
                    }).to_string().into_bytes(),
                    Some("Invalid public key length".as_bytes().to_vec())
                ),
            ];

            for (case_name, body, expected_body) in cases {
                println!("Running case: {case_name}");
                ctx.set_request_body(body);

                match InitTunnelHandler::validate_request_body(&mut ctx).await {
                    Err(response) => {
                        assert_eq!(response.status, StatusCode::BAD_REQUEST);
                        if expected_body != None {
                            assert_eq!(response.body, expected_body);
                        }
                    }
                    Ok(_) => {
                        panic!("Expected error for invalid public key length");
                    }
                }
            }
        }

        #[tokio::test]
        async fn test_validate_request_body_parse_error() {
            let mut ctx = Layer8Context::default();

            let cases = vec![
                (
                    "Public key is a string instead of byte array",
                    json!({
                        "public_key": "not a byte array"
                    }).to_string().into_bytes(),
                ),
                (
                    "Public key is a number instead of byte array",
                    json!({
                        "public_key": 12345
                    }).to_string().into_bytes(),
                ),
                (
                    "Public key is null",
                    json!({
                        "public_key": null
                    }).to_string().into_bytes(),
                ),
                (
                    "Public key is an object instead of byte array",
                    json!({
                        "public_key": {}
                    }).to_string().into_bytes(),
                ),
                (
                    "Public key is an array instead of byte array",
                    json!({
                        "public_key": []
                    }).to_string().into_bytes(),
                ),
                (
                    "Public key is missing from the JSON",
                    json!({
                        "not_public_key": b"a".repeat(32)
                    }).to_string().into_bytes(),
                ),
                (
                    "Invalid JSON format (not even a JSON object)",
                    b"invalid json".to_vec(),
                )
            ];

            for (case_name, body) in cases {
                println!("Running case: {case_name}");
                ctx.set_request_body(body);
                let result = InitTunnelHandler::validate_request_body(&mut ctx).await;

                match result {
                    Err(response) => {
                        assert_eq!(response.status, StatusCode::BAD_REQUEST);
                        assert!(response.body.is_some())
                    }
                    Ok(_) => panic!("Expected error for invalid JSON"),
                }
            }
        }
    }
}
