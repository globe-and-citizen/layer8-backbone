#[path = "../mock/mod.rs"]
mod mock;

#[cfg(test)]
mod test_init_tunnel_request {
    use reverse_proxy::handler::init_tunnel::{InitEncryptedTunnelRequest, InitEncryptedTunnelResponse};
    use crate::mock;
    use crate::mock::start_mock_services;

    #[tokio::test]
    async fn test() {
        start_mock_services();
        let client = reqwest::Client::new();
        let body = InitEncryptedTunnelRequest {
            public_key: Vec::from(mock::data::MOCK_NTOR_CLIENT_PUBLIC_KEY),
        };

        let request_url = format!(
            "http://localhost:{}/init-tunnel?backend_url={}",
            mock::data::REVERSE_PROXY_PORT,
            mock::data::MOCK_BACKEND_URL
        );

        let response = client.post(request_url).json(&body).send().await;

        println!("Response: {:?}", response);
        assert!(response.is_ok(), "Expected the request to succeed");
        let res = response.unwrap();
        assert_eq!(res.status(), reqwest::StatusCode::OK, "Expected status code 200 OK");

        let response_body = res.bytes().await;
        assert!(response_body.is_ok(), "Expected to successfully read response body");
        let body_bytes = response_body.unwrap();
        assert!(!body_bytes.is_empty(), "Expected response body to be non-empty");

        let body = serde_json::from_slice::<InitEncryptedTunnelResponse>(&body_bytes)
            .expect("Failed to deserialize response body to InitTunnelResponse");

        assert_eq!(body.public_key.len(), 32, "Expected public key length to be 32 bytes");
        assert_eq!(body.t_b_hash.len(), 32, "Expected t_b_hash length to be 32 bytes");
        assert!(body.int_rp_jwt.len() > 0, "Expected int_rp_jwt to be non-empty");
        assert!(body.fp_rp_jwt.len() > 0, "Expected fp_rp_jwt to be non-empty");

        // todo check jwts?
    }
}