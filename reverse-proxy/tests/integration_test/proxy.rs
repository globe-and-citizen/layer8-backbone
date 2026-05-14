#[path = "../mock/mod.rs"]
mod mock;

#[cfg(test)]
mod test_proxy_request {
    use std::collections::HashMap;
    use ntor::common::EncryptedMessage;
    use reverse_proxy::handler::InMemorySecretsStorage;
    use crate::mock;
    use crate::mock::start_mock_services;
    use crate::mock::data::{
        create_fp_rp_jwt, create_int_rp_jwt_1, create_int_rp_jwt_2,
        MOCK_SESSION_ID_1, MOCK_SESSION_ID_2, MOCK_SHARED_SECRET_1, MOCK_SHARED_SECRET_2, VALID_JWT_SECRET
    };

    #[tokio::test]
    async fn test_success() {
        start_mock_services();

        // add mock session data to in-memory storage
        let sessions = HashMap::from([
            (MOCK_SESSION_ID_1.to_string(), MOCK_SHARED_SECRET_1.to_vec()),
            (MOCK_SESSION_ID_2.to_string(), MOCK_SHARED_SECRET_2.to_vec()),
        ]);
        InMemorySecretsStorage::init(sessions);

        let valid_int_rp_jwt_1 = create_int_rp_jwt_1(VALID_JWT_SECRET, 24);
        let valid_int_rp_jwt_2 = create_int_rp_jwt_2(VALID_JWT_SECRET, 24);
        let valid_fp_rp_jwt = create_fp_rp_jwt(VALID_JWT_SECRET, 24);

        let cases = vec![
            (
                "Case 1",
                mock::data::MOCK_PROXY_REQUEST_BODY_1.to_vec(),
                valid_int_rp_jwt_1,
            ),
            (
                "Case 2",
                mock::data::MOCK_PROXY_REQUEST_BODY_2.to_vec(),
                valid_int_rp_jwt_2,
            )
        ];

        let request_url = format!(
            "http://localhost:{}/proxy",
            mock::data::REVERSE_PROXY_PORT,
        );

        for (case_name, body, int_rp_jwt) in cases {
            println!("Testing {}", case_name);
            let client = reqwest::Client::new();

            let response = client
                .post(request_url.clone())
                .header(mock::data::INT_RP_JWT_HEADER, int_rp_jwt)
                .header(mock::data::FP_RP_JWT_HEADER, valid_fp_rp_jwt.clone())
                .body(body)
                .send()
                .await;

            println!("Response: {:?}", response);
            assert!(response.is_ok(), "Expected the request to succeed");
            let res = response.unwrap();
            assert_eq!(res.status(), reqwest::StatusCode::OK, "Expected status code 200 OK");

            let response_body = res.bytes().await;
            assert!(response_body.is_ok(), "Expected to successfully read response body");
            let body_bytes = response_body.unwrap();
            assert!(!body_bytes.is_empty(), "Expected response body to be non-empty");

            let res_body = EncryptedMessage::from_bytes(&body_bytes).expect("Response body should be a valid EncryptedMessage");
            assert!(!res_body.nonce.is_empty(), "Nonce should not be empty");
            assert!(!res_body.data.is_empty(), "Ciphertext should not be empty");
        }
    }
}

