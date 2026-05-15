use tokio::time::sleep;
use crate::mock::data::EncryptedMessage;

mod mock;

#[tokio::test]
async fn test_proxy_data() {
    mock::init().await;

    sleep(std::time::Duration::from_secs(5)).await; // Wait for the mock server to start

    let client = reqwest::Client::new();
    let body = EncryptedMessage {
        nonce: mock::data::MOCK_PROXY_REQUEST_NONCE,
        data: mock::data::MOCK_PROXY_REQUEST_DATA.to_vec(),
    };

    let body_bytes = utils::type_to_bincode(&body);

    let response = client
        .post(
            format!(
                "http://localhost:{}{}",
                mock::data::FORWARD_PROXY_PORT,
                mock::data::PROXY_API_PATH,
            )
        )
        .header("int_fp_jwt", mock::data::MOCK_INT_FP_JWT)
        .header("int_rp_jwt", mock::data::MOCK_INT_RP_JWT)
        .body(body_bytes.clone())
        .send()
        .await;

    println!("Response: {:?}", response);

    match response {
        Ok(res) => {
            let status = res.status();
            let res_body_bytes: axum::body::Bytes = res.bytes().await.expect("Failed to parse response body");

            let res_body: EncryptedMessage = utils::bincode_to_type(&res_body_bytes).expect("Failed to decode response body");
            println!("Response body: {:?}", res_body);

            assert_eq!(res_body.nonce, mock::data::MOCK_PROXY_RESPONSE_NONCE, "Response nonce does not match expected value");
            assert_eq!(res_body.data, mock::data::MOCK_PROXY_RESPONSE_DATA.to_vec(), "Response data does not match expected value");

            assert_eq!(status, reqwest::StatusCode::OK, "Expected status 200 OK");
        }
        Err(err) => {
            assert!(false, "Request failed: {}", err);
        }
    }
}