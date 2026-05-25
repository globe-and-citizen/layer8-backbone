use crate::mock;
use crate::mock::data::{
    BACKEND_SERVER_PORT, EncryptedMessage, INIT_TUNNEL_API_PATH, PROXY_API_PATH,
};
use axum::http::{HeaderMap, StatusCode};
use axum::routing::post;
use axum::{Json, Router};
use forward_proxy::handler::types::response::InitTunnelResponseFromRP;
use serde::Deserialize;
use std::net::SocketAddr;

pub(crate) async fn start_mock_backend_server() {
    let mut app = Router::new();
    app = app.route(INIT_TUNNEL_API_PATH, post(init_tunnel_handler));
    app = app.route(PROXY_API_PATH, post(proxy_handler));

    let addr = SocketAddr::from(([127, 0, 0, 1], BACKEND_SERVER_PORT));
    println!("Mock upstream server listening on {}", addr);

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

#[derive(Deserialize)]
struct InitTunnelRequest {
    pub public_key: Vec<u8>,
}

async fn init_tunnel_handler(
    Json(payload): Json<InitTunnelRequest>,
) -> (StatusCode, Json<InitTunnelResponseFromRP>) {
    // if the payload wasn't changed by forward-proxy
    if payload.public_key != mock::data::MOCK_NTOR_CLIENT_PUBLIC_KEY {
        println!("Received invalid public key: {:?}", payload.public_key);
        return (
            StatusCode::BAD_REQUEST,
            Json(InitTunnelResponseFromRP {
                public_key: vec![],
                t_b_hash: vec![],
                int_rp_jwt: String::new(),
                fp_rp_jwt: String::new(),
            }),
        );
    }

    let response = InitTunnelResponseFromRP {
        public_key: Vec::from(mock::data::MOCK_NTOR_SERVER_EPHEMERAL_PUBLIC_KEY),
        t_b_hash: Vec::from(mock::data::MOCK_NTOR_SERVER_T_B_HASH),
        int_rp_jwt: mock::data::MOCK_INT_RP_JWT.to_string(),
        fp_rp_jwt: mock::data::MOCK_FP_RP_JWT.to_string(),
    };

    println!(
        "Received valid init tunnel request. Responding with: {:?}",
        response
    );
    (StatusCode::OK, Json(response))
}

async fn proxy_handler(
    headers: HeaderMap,
    payload: axum::body::Bytes,
) -> (StatusCode, axum::body::Bytes) {
    // Validate int_rp_jwt header
    let int_rp_jwt = headers
        .get("int_rp_jwt")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| {
            return (
                StatusCode::UNAUTHORIZED,
                utils::type_to_bincode(&EncryptedMessage {
                    nonce: [0; 12],
                    data: Vec::from(b"Missing int_rp_jwt header"),
                }),
            );
        })
        .unwrap();

    if int_rp_jwt != mock::data::MOCK_INT_RP_JWT {
        return (
            StatusCode::UNAUTHORIZED,
            axum::body::Bytes::from(utils::type_to_bincode(&EncryptedMessage {
                nonce: [0; 12],
                data: b"Invalid int_rp_jwt token".to_vec(),
            })),
        );
    }

    // validate fp_rp_jwt header
    let fp_rp_jwt = headers
        .get("fp_rp_jwt")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| {
            return (
                StatusCode::UNAUTHORIZED,
                axum::body::Bytes::from(utils::type_to_bincode(&EncryptedMessage {
                    nonce: [0; 12],
                    data: b"Missing fp_rp_jwt header".to_vec(),
                })),
            );
        })
        .unwrap();

    if fp_rp_jwt != mock::data::MOCK_FP_RP_JWT {
        return (
            StatusCode::UNAUTHORIZED,
            axum::body::Bytes::from(utils::type_to_bincode(&EncryptedMessage {
                nonce: [0; 12],
                data: b"Invalid fp_rp_jwt token".to_vec(),
            })),
        );
    }

    // int_fp_jwt should not be included
    if headers.contains_key("int_fp_jwt") {
        return (
            StatusCode::BAD_REQUEST,
            axum::body::Bytes::from(utils::type_to_bincode(&EncryptedMessage {
                nonce: [0; 12],
                data: b"int_fp_jwt header should not be included".to_vec(),
            })),
        );
    }

    // Validate the payload data
    let payload_json: EncryptedMessage = utils::bincode_to_type(&payload)
        .map_err(|e| {
            println!("Failed to decode payload: {}", e);
            return (
                StatusCode::BAD_REQUEST,
                axum::body::Bytes::from(utils::type_to_bincode(&EncryptedMessage {
                    nonce: [0; 12],
                    data: Vec::from(b"Invalid payload format"),
                })),
            );
        })
        .unwrap();

    println!("Received proxy request with payload: {:?}", payload_json);

    if payload_json.nonce != mock::data::MOCK_PROXY_REQUEST_NONCE {
        return (
            StatusCode::BAD_REQUEST,
            axum::body::Bytes::from(utils::type_to_bincode(&EncryptedMessage {
                nonce: [0; 12],
                data: b"Proxy request nonce does not match expected value".to_vec(),
            })),
        );
    }

    if payload_json.data != mock::data::MOCK_PROXY_REQUEST_DATA {
        return (
            StatusCode::BAD_REQUEST,
            axum::body::Bytes::from(utils::type_to_bincode(&EncryptedMessage {
                nonce: [0; 12],
                data: b"Proxy request data does not match expected value".to_vec(),
            })),
        );
    }

    let response = axum::body::Bytes::from(utils::type_to_bincode(&EncryptedMessage {
        nonce: mock::data::MOCK_PROXY_RESPONSE_NONCE,
        data: mock::data::MOCK_PROXY_RESPONSE_DATA.to_vec(),
    }));

    println!(
        "Received valid proxy request. Responding with: {:?}",
        response
    );

    (StatusCode::OK, response)
}
