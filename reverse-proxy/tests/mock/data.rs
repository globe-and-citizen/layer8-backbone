use utils::jwt::JWTClaims;

// mock server config
pub const REVERSE_PROXY_PORT: u16 = 6393;
pub const MOCK_BACKEND_PORT: u16 = 3003;
pub const MOCK_BACKEND_URL: &str = "http://localhost:3003";
pub const VALID_JWT_SECRET: &[u8] = b"test_valid_jwt_secret";
pub const INVALID_JWT_SECRET: &[u8] = b"test_invalid_jwt_secret";

// constants
pub const INT_RP_JWT_HEADER: &str = "int_rp_jwt";
pub const FP_RP_JWT_HEADER: &str = "fp_rp_jwt";

// mock session data
pub const MOCK_NTOR_SERVER_ID: &str = "mock_server_id";
pub const MOCK_NTOR_CLIENT_PUBLIC_KEY: [u8; 32] = [1; 32];
pub const MOCK_NTOR_SERVER_EPHEMERAL_PUBLIC_KEY: [u8; 32] = [2; 32];
pub const MOCK_NTOR_SERVER_STATIC_PUBLIC_KEY: [u8; 32] = [3; 32];
pub const MOCK_NTOR_SERVER_T_B_HASH: [u8; 32] = [4; 32];
pub const MOCK_SESSION_ID_1: &str = "d92db61d-e8d8-4f91-9ab4-b9fa9c53e65c";
pub const MOCK_SESSION_ID_2: &str = "a3f5c8e7-1b2c-4d6e-9f8a-0b1c2d3e4f5g";
pub const MOCK_SHARED_SECRET_1: [u8; 16] = [245, 239, 74, 167, 84, 191, 140, 194, 16, 59, 154, 244, 108, 221, 148, 85];
pub const MOCK_SHARED_SECRET_2: [u8; 16] = [226, 186, 202, 159, 51, 31, 151, 34, 174, 233, 128, 164, 202, 226, 146, 91];
/// api GET /me
pub const MOCK_PROXY_REQUEST_BODY_1: [u8; 80] = [159, 207, 157, 116, 32, 92, 248, 78, 122, 253, 236, 125, 67, 223, 157, 30, 30, 45, 49, 165, 234, 211, 72, 242, 252, 31, 128, 60, 245, 158, 182, 126, 117, 152, 232, 172, 52, 155, 246, 122, 86, 89, 78, 162, 110, 171, 73, 84, 127, 41, 195, 46, 85, 31, 71, 121, 234, 63, 27, 236, 43, 190, 186, 124, 94, 212, 238, 13, 254, 32, 147, 59, 239, 30, 176, 138, 54, 167, 161, 132];
/// api GET /profile/test
pub const MOCK_PROXY_REQUEST_BODY_2: [u8; 90] = [210, 33, 207, 169, 246, 8, 233, 118, 37, 197, 180, 162, 77, 165, 168, 70, 128, 112, 244, 122, 187, 27, 188, 245, 126, 167, 152, 194, 233, 6, 37, 95, 101, 101, 247, 243, 37, 221, 51, 101, 23, 95, 2, 28, 123, 161, 251, 79, 193, 18, 75, 19, 204, 130, 106, 149, 30, 170, 91, 8, 218, 17, 212, 12, 130, 29, 187, 109, 187, 30, 42, 25, 187, 152, 171, 75, 78, 99, 74, 47, 61, 196, 224, 108, 107, 217, 220, 32, 187, 70];

pub fn create_int_rp_jwt_1(secret: &[u8], expiry_hrs: i64) -> String {
    let mut claims = JWTClaims::new(Some(expiry_hrs));
    claims.ntor_session_id = Some(MOCK_SESSION_ID_1.to_string());
    utils::jwt::create_jwt_token(claims, secret)
}

pub fn create_int_rp_jwt_2(secret: &[u8], expiry_hrs: i64) -> String {
    let mut claims = JWTClaims::new(Some(expiry_hrs));
    claims.ntor_session_id = Some(MOCK_SESSION_ID_2.to_string());
    utils::jwt::create_jwt_token(claims, secret)
}

pub fn create_fp_rp_jwt(secret: &[u8], expiry_hrs: i64) -> String {
    let claims = JWTClaims::new(Some(expiry_hrs));
    utils::jwt::create_jwt_token(claims, secret)
}