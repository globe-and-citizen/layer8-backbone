use axum::{
    extract::Query,
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use serde_json::json;
use std::net::SocketAddr;

const BACKEND_URL: &str = "http://valid-backend";
const VALID_TOKEN: &str = "Bearer valid-token";
const CERTIFICATE_RESPONSE: &str = "-----BEGIN CERTIFICATE-----\nMIIC...\n-----END CERTIFICATE-----";
const CLIENT_ID: &str = "test-client-123";


async fn start_mock_auth_server() {
    let app = Router::new()
        .route("/ntor/certificate/:backend_url", get(get_certificate));

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("Mock auth server listening on {}", addr);

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn get_certificate(
    headers: axum::http::HeaderMap,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    // Validate Authorization header
    let auth_header = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| (StatusCode::UNAUTHORIZED, "Missing Authorization header".to_string()))?;

    if auth_header.is_empty() {
        return Err((StatusCode::UNAUTHORIZED, "Authorization header cannot be empty".to_string()));
    }

    if auth_header != VALID_TOKEN {
        return Err((StatusCode::UNAUTHORIZED, "Invalid token".to_string()));
    }

    // Validate required query parameters
    let backend_url = params
        .get("backend_url")
        .ok_or_else(|| (StatusCode::BAD_REQUEST, "Missing backend_url parameter".to_string()))?;

    if backend_url.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "backend_url cannot be empty".to_string()));
    }

    if backend_url != BACKEND_URL {
        return Err((StatusCode::BAD_REQUEST, "Invalid backend_url".to_string()));
    }

    Ok((
        StatusCode::OK,
        Json(json!({
            "cert": CERTIFICATE_RESPONSE,
            "client_id": CLIENT_ID,
        })),
    ))
}
