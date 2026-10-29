use crate::mock::backend::LAST_PROXY_REQUEST_HEADERS;
use crate::mock::data::EncryptedMessage;
use tokio::time::sleep;

mod mock;

const CLIENT_COOKIE: &str = "session=client-session-id";
const CLIENT_TRACEPARENT: &str = "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01";
const CLIENT_USER_AGENT: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0.0.0 Safari/537.36";
const CLIENT_SEC_CH_UA: &str = "\"Chromium\";v=\"130\", \"Google Chrome\";v=\"130\", \"Not?A_Brand\";v=\"99\"";
const CLIENT_X_FORWARDED_FOR: &str = "203.0.113.7";

#[tokio::test]
async fn test_proxy_anonymizes_identifying_headers() {
    mock::init().await;

    sleep(std::time::Duration::from_secs(5)).await; // Wait for the mock server to start

    let client = reqwest::Client::new();
    let body = EncryptedMessage {
        nonce: mock::data::MOCK_PROXY_REQUEST_NONCE,
        data: mock::data::MOCK_PROXY_REQUEST_DATA.to_vec(),
    };

    let response = client
        .post(format!(
            "http://localhost:{}{}",
            mock::data::FORWARD_PROXY_PORT,
            mock::data::PROXY_API_PATH,
        ))
        .header("int_fp_jwt", mock::data::MOCK_INT_FP_JWT)
        .header("int_rp_jwt", mock::data::MOCK_INT_RP_JWT)
        .header("cookie", CLIENT_COOKIE)
        .header("traceparent", CLIENT_TRACEPARENT)
        .header("user-agent", CLIENT_USER_AGENT)
        .header("sec-ch-ua", CLIENT_SEC_CH_UA)
        .header("x-forwarded-for", CLIENT_X_FORWARDED_FOR)
        .body(utils::type_to_bincode(&body))
        .send()
        .await
        .expect("Request failed");

    assert_eq!(
        response.status(),
        reqwest::StatusCode::OK,
        "Expected status 200 OK"
    );

    let headers = LAST_PROXY_REQUEST_HEADERS
        .lock()
        .unwrap()
        .clone()
        .expect("Mock backend did not receive the proxy request");
    println!("Headers received by mock backend: {:#?}", headers);

    let header = |name: &str| headers.get(name).and_then(|v| v.to_str().ok());

    // Browser fingerprinting and client IP headers must be dropped entirely
    assert!(
        !headers.contains_key("sec-ch-ua"),
        "sec-ch-ua should not be forwarded"
    );
    assert!(
        !headers.contains_key("x-forwarded-for"),
        "x-forwarded-for should not be forwarded"
    );

    // The FP may add its own trace context, but never the client's
    assert_ne!(
        header("traceparent"),
        Some(CLIENT_TRACEPARENT),
        "client traceparent should not be forwarded"
    );

    // User-Agent is replaced with a generic value
    assert_eq!(
        header("user-agent"),
        Some("Mozilla/5.0"),
        "user-agent should be replaced with the generic value"
    );

    // Cookie is forwarded on purpose (needed for login)
    assert_eq!(
        header("cookie"),
        Some(CLIENT_COOKIE),
        "cookie should be forwarded unchanged"
    );
}
