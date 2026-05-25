use crate::mock::data::MOCK_BACKEND_PORT;
use axum::http::header::SET_COOKIE;
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use std::net::SocketAddr;

#[allow(dead_code)]
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
        HeaderValue::from_static("session_id=abc123; HttpOnly; Path=/"),
    );

    (
        axum::http::StatusCode::OK,
        response_headers,
        Json(ResponseBody { success: true }),
    )
}

async fn test_api_get_profile(_headers: HeaderMap) -> impl IntoResponse {
    (
        axum::http::StatusCode::OK,
        Json(ResponseBody { success: true }),
    )
}

#[derive(Debug, serde::Deserialize)]
struct TestRequestBody {
    key: String,
}
async fn test_api(
    Json(payload): Json<TestRequestBody>,
) -> (StatusCode, HeaderMap, Json<ResponseBody>) {
    println!("Received request with payload: {:?}", payload);

    let response = ResponseBody {
        success: payload.key == "value",
    };

    let mut headers = HeaderMap::new();
    headers.insert(
        SET_COOKIE,
        HeaderValue::from_static("session_id=abc123; HttpOnly; Path=/; Max-Age=3600"),
    );

    (StatusCode::OK, headers, Json(response))
}

pub fn run_mock_be() {
    let mut app = Router::new();
    app = app.route("/me", get(test_api_get_me));
    app = app.route("/profile/test", get(test_api_get_profile));
    app = app.route("/test/api", post(test_api));

    let addr = SocketAddr::from(([127, 0, 0, 1], MOCK_BACKEND_PORT));
    println!("Mock upstream server listening on {:?}", addr.clone());

    // Run server in background
    tokio::spawn(async move {
        let listener = tokio::net::TcpListener::bind(addr)
            .await
            .unwrap();

        axum::serve(listener, app)
            .await
            .unwrap();
    });
}
