use std::net::SocketAddr;

use crate::mock;
use crate::mock::data::{
    AUTH_ACCESS_TOKEN, AUTH_NTOR_CERT_API_PATH, AUTH_SERVER_PORT, BACKEND_URL,
};
use axum::{
    Router,
    extract::Query,
    http::{HeaderMap, StatusCode, header::AUTHORIZATION},
    response::Json,
    routing::get,
};
use serde::{Deserialize, Serialize};

pub(crate) async fn start_mock_auth_server() {
    let app = Router::new().route(AUTH_NTOR_CERT_API_PATH, get(get_certificate));

    let addr = SocketAddr::from(([127, 0, 0, 1], AUTH_SERVER_PORT));

    println!("Mock auth server listening on {}", addr);

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

#[derive(Serialize, Debug)]
struct AuthServerResponse {
    cert: String,
    client_id: String,
}

#[derive(Deserialize, Debug)]
struct CertificateQuery {
    backend_url: String,
}

async fn get_certificate(
    headers: HeaderMap,
    Query(params): Query<CertificateQuery>,
) -> Result<Json<AuthServerResponse>, (StatusCode, String)> {
    let auth_header = headers
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| {
            (
                StatusCode::UNAUTHORIZED,
                "Missing Authorization header".to_string(),
            )
        })?;

    if auth_header != AUTH_ACCESS_TOKEN {
        return Err((
            StatusCode::UNAUTHORIZED,
            "Invalid authorization token".to_string(),
        ));
    }

    if params.backend_url != BACKEND_URL {
        return Err((StatusCode::BAD_REQUEST, "Invalid backend_url".to_string()));
    }

    Ok(Json(AuthServerResponse {
        cert: mock::data::MOCK_NTOR_CERTIFICATE.to_string(),
        client_id: mock::data::MOCK_AUTH_CLIENT_ID.to_string(),
    }))
}
