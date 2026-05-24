pub const AUTH_SERVER_PORT: u16 = 3000;
pub const FORWARD_PROXY_PORT: u16 = 6391;
pub const BACKEND_SERVER_PORT: u16 = 6393;
pub const BACKEND_URL: &str = "http://localhost:6393";
pub const AUTH_ACCESS_TOKEN: &str = "Bearer valid-token";
pub const AUTH_NTOR_CERT_API_PATH: &str = "/ntor/certificate"; // ?backend_url=
pub const INIT_TUNNEL_API_PATH: &str = "/init-tunnel"; // ?backend_url=
pub const PROXY_API_PATH: &str = "/proxy";

pub const MOCK_NTOR_CLIENT_PUBLIC_KEY: [u8; 32] = [
    157, 192, 124, 16, 87, 35, 182, 121, 125, 166, 205, 87, 89, 15, 158, 42, 84, 193, 173, 211,
    155, 177, 32, 217, 51, 204, 79, 44, 189, 176, 79, 21,
];
pub const MOCK_NTOR_SERVER_EPHEMERAL_PUBLIC_KEY: [u8; 32] = [
    237, 12, 46, 100, 252, 26, 102, 19, 73, 40, 204, 185, 16, 72, 199, 57, 60, 89, 105, 239, 164,
    255, 209, 100, 250, 41, 246, 157, 7, 121, 221, 111,
];
pub const MOCK_NTOR_SERVER_T_B_HASH: [u8; 32] = [
    175, 53, 72, 226, 127, 62, 62, 121, 255, 198, 133, 168, 224, 43, 81, 172, 32, 138, 194, 200,
    165, 61, 48, 18, 73, 199, 122, 117, 93, 173, 143, 59,
];
#[allow(dead_code)]
pub const MOCK_NTOR_SERVER_STATIC_PUBLIC_KEY: [u8; 32] = [
    131, 210, 36, 101, 39, 191, 61, 165, 29, 112, 94, 149, 120, 202, 189, 170, 151, 62, 247, 71,
    208, 255, 144, 173, 52, 223, 239, 221, 153, 225, 40, 10,
];
pub const MOCK_NTOR_CERTIFICATE: &str = "-----BEGIN CERTIFICATE-----\nMIH4MIGroAMCAQICFCAy9VJULMTDz4YxgT3Yj3gny6HTMAUGAytlcDAcMRowGAYD\nVQQDDBFyZXZlcnNlX3Byb3h5LmNvbTAeFw0yNTA3MTYxMDMzMzdaFw0yNjA3MTYx\nMDMzMzdaMB0xGzAZBgNVBAMMElJldmVyc2VQcm94eVNlcnZlcjAqMAUGAytlbgMh\nAIPSJGUnvz2lHXBelXjKvaqXPvdH0P+QrTTf792Z4SgKMAUGAytlcANBANMvwCl1\nB8oRatOTicKGmPlO6wUj3bmhd5ldOcd3xLB1h47HTRJs8mdTWD3pqayPGGnuYRsX\nNjCXOCyH/VbUlQM=\n-----END CERTIFICATE-----";
pub const MOCK_AUTH_CLIENT_ID: &str = "whatever"; // "c3e4a0ed-e7e4-4251-8339-78e6611c7293";
pub const MOCK_INT_RP_JWT: &str = "whatever but not the same as INT_FP_JWT and FP_RP_JWT"; // "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJleHAiOjE3NzgwNjM0NjAsImlhdCI6MTc3Nzk3NzA2MCwic2lkIjoiNzA3NTI1NzItMWE2YS00MzdhLWJjZGItMzQ0Mzg4OThlZjFkIiwidXVpZCI6bnVsbH0._uYz2HfOld5xpQQgOzX8eiw99-35Ys8OmuaGBtMCS8Y"
pub const MOCK_INT_FP_JWT: &str = "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJleHAiOjIwOTM0NDcyMTgsImlhdCI6MTc3ODA4NzIxOCwidXVpZCI6ImNkMmMwYmJhLTA0ZTAtNGYyNS04NmQ3LWJkMmM5ZDFkMzczNiJ9.NU6zmU9RatYW8kxHJnE0Q2H7wzzExanuZLvcRnEcDdw";
pub const MOCK_FP_RP_JWT: &str = "whatever but not the same as INT_FP_JWT and INT_RP_JWT";
pub const MOCK_JWT_SECRET: &str = "secret";

pub const MOCK_PROXY_REQUEST_NONCE: [u8; 12] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12];
pub const MOCK_PROXY_REQUEST_DATA: &[u8] =
    b"Hello, this is a test message from the client to the backend through the forward proxy.";
pub const MOCK_PROXY_RESPONSE_NONCE: [u8; 12] = [12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1];
pub const MOCK_PROXY_RESPONSE_DATA: &[u8] =
    b"Hello, this is a test message from the backend to the client through the forward proxy.";

#[derive(bincode::Encode, bincode::Decode, Debug)]
pub struct EncryptedMessage {
    // this struct is defined in ntor crate, but we redefine it here to avoid adding ntor as a dependency in forward-proxy
    pub nonce: [u8; 12],
    pub data: Vec<u8>,
}
