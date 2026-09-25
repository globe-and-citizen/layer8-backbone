use utils::jwt::JWTClaims;

/// Maps mock data set:
/// 1. tokens(MOCK_SESSION_ID_1, VALID_JWT_SECRET)
///     -> memory session(MOCK_SESSION_ID_1, MOCK_SHARED_SECRET_1)
///     -> request(int_fp_jwt1, MOCK_PROXY_REQUEST_BODY_1)
///     -> decrypt (MOCK_SHARED_SECRET_1,  MOCK_PROXY_REQUEST_BODY_1)
///     -> be(MOCK_API_PATH_1, handler)
/// 2. similar to 1 without api path.

// mock server config
#[allow(dead_code)]
pub const REVERSE_PROXY_PORT: u16 = 6393;
#[allow(dead_code)]
pub const MOCK_BACKEND_PORT: u16 = 3003;
#[allow(dead_code)]
pub const MOCK_BACKEND_URL: &str = "http://localhost:3003";
#[allow(dead_code)]
pub const VALID_JWT_SECRET: &[u8] = b"test_valid_jwt_secret";
#[allow(dead_code)]
pub const INVALID_JWT_SECRET: &[u8] = b"test_invalid_jwt_secret";

// constants
#[allow(dead_code)]
pub const INT_RP_JWT_HEADER: &str = "int_rp_jwt";
#[allow(dead_code)]
pub const FP_RP_JWT_HEADER: &str = "fp_rp_jwt";

// mock session data
#[allow(dead_code)]
pub const MOCK_NTOR_SERVER_ID: &str = "mock_server_id";
#[allow(dead_code)]
pub const MOCK_NTOR_CLIENT_PUBLIC_KEY: [u8; 32] = [1; 32];
#[allow(dead_code)]
pub const MOCK_NTOR_SERVER_EPHEMERAL_PUBLIC_KEY: [u8; 32] = [2; 32];
#[allow(dead_code)]
pub const MOCK_NTOR_SERVER_STATIC_PUBLIC_KEY: [u8; 32] = [3; 32];
#[allow(dead_code)]
pub const MOCK_NTOR_SERVER_T_B_HASH: [u8; 32] = [4; 32];
#[allow(dead_code)]
pub const MOCK_SESSION_ID_1: &str = "d92db61d-e8d8-4f91-9ab4-b9fa9c53e65c";
#[allow(dead_code)]
pub const MOCK_SESSION_ID_2: &str = "a3f5c8e7-1b2c-4d6e-9f8a-0b1c2d3e4f5g";
#[allow(dead_code)]
pub const MOCK_SHARED_SECRET_1: [u8; 16] = [
    233, 27, 171, 53, 222, 167, 4, 49, 178, 16, 154, 109, 97, 127, 121, 172,
];
#[allow(dead_code)]
pub const MOCK_SHARED_SECRET_2: [u8; 16] = [
    182, 79, 4, 127, 72, 121, 95, 240, 223, 109, 20, 116, 226, 221, 8, 79,
];
#[allow(dead_code)]
pub const MOCK_API_PATH_1: &str = "/v1/posts";
/// api GET /v1/posts?size=10&page=1
#[allow(dead_code)]
pub const MOCK_PROXY_REQUEST_BODY_1: [u8; 90] = [
    21, 194, 226, 232, 59, 31, 250, 27, 220, 125, 64, 37, 77, 99, 189, 33, 212, 139, 185, 242, 119,
    3, 121, 99, 43, 129, 224, 118, 36, 146, 122, 189, 233, 62, 104, 223, 200, 85, 176, 220, 112,
    35, 5, 35, 127, 84, 166, 149, 175, 54, 186, 90, 134, 49, 47, 145, 76, 19, 158, 107, 54, 180,
    112, 170, 184, 151, 72, 154, 251, 86, 90, 131, 215, 182, 66, 32, 6, 238, 106, 41, 196, 239, 17,
    252, 116, 39, 87, 227, 171, 98,
];
/// api GET /v1/users/test
#[allow(dead_code)]
pub const MOCK_PROXY_REQUEST_BODY_2: [u8; 241] = [
    104, 245, 113, 76, 208, 47, 121, 69, 61, 108, 168, 233, 228, 144, 102, 188, 187, 111, 107, 164,
    101, 13, 75, 128, 55, 142, 148, 18, 165, 106, 224, 225, 45, 83, 38, 129, 176, 124, 162, 179,
    63, 217, 219, 69, 26, 114, 118, 122, 85, 126, 35, 150, 60, 227, 95, 51, 245, 122, 246, 154, 92,
    138, 1, 166, 98, 225, 235, 65, 138, 48, 30, 57, 209, 28, 205, 4, 172, 32, 39, 209, 39, 224,
    195, 28, 76, 199, 73, 210, 195, 169, 25, 67, 20, 207, 222, 153, 253, 51, 59, 91, 65, 218, 251,
    229, 242, 126, 48, 245, 15, 76, 254, 192, 173, 12, 155, 125, 89, 81, 126, 124, 198, 43, 210, 4,
    1, 188, 197, 86, 95, 231, 165, 167, 113, 104, 236, 67, 163, 3, 110, 178, 83, 148, 9, 167, 191,
    7, 140, 116, 238, 110, 168, 52, 3, 194, 67, 3, 75, 39, 214, 196, 42, 156, 36, 128, 234, 228,
    133, 223, 196, 28, 176, 49, 206, 172, 53, 77, 135, 245, 130, 66, 58, 203, 75, 132, 205, 27, 30,
    117, 224, 127, 73, 219, 116, 236, 174, 0, 148, 97, 175, 236, 191, 63, 195, 245, 157, 94, 227,
    64, 160, 153, 61, 137, 198, 69, 60, 126, 154, 175, 122, 198, 91, 157, 20, 240, 196, 19, 51, 33,
    255, 96, 212, 253, 164, 81, 48, 169, 107, 138, 240, 154, 150,
];

#[allow(dead_code)]
pub fn create_int_rp_jwt_1(secret: &[u8], expiry_hrs: i64) -> String {
    let mut claims = JWTClaims::new(Some(expiry_hrs));
    claims.ntor_session_id = Some(MOCK_SESSION_ID_1.to_string());
    utils::jwt::create_jwt_token(claims, secret)
}

#[allow(dead_code)]
pub fn create_int_rp_jwt_2(secret: &[u8], expiry_hrs: i64) -> String {
    let mut claims = JWTClaims::new(Some(expiry_hrs));
    claims.ntor_session_id = Some(MOCK_SESSION_ID_2.to_string());
    utils::jwt::create_jwt_token(claims, secret)
}

#[allow(dead_code)]
pub fn create_fp_rp_jwt(secret: &[u8], expiry_hrs: i64) -> String {
    let claims = JWTClaims::new(Some(expiry_hrs));
    utils::jwt::create_jwt_token(claims, secret)
}
