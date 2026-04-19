# 📘 Data Dictionary

This document defines the data structures used for initializing secure tunnels, managing sessions, performing health
checks, and handling Layer 8 (L8) HTTP requests and responses within the system. These models facilitate communication
between the **Interceptor (INT)**, **Forward Proxy (FP)**, and **Reverse Proxy (RP)**.

---

## 1. `InitTunnelRequest`

**Description:**  
Represents a request to initiate a secure tunnel by providing the client’s public key for key exchange.

| Field        | Type      | Description                                                        |
|--------------|-----------|--------------------------------------------------------------------|
| `public_key` | `Vec<u8>` | Client’s ephemeral public key used for cryptographic key exchange. |

---

## 2. `ErrorResponse`

**Description:**  
Standard error response returned when an operation fails.

| Field   | Type     | Visibility   | Description                                          |
|---------|----------|--------------|------------------------------------------------------|
| `error` | `String` | `pub(crate)` | Human-readable error message describing the failure. |

---

## 3. `InitTunnelResponseFromRP`

**Description:**  
Response from the **Reverse Proxy (RP)** during tunnel initialization. This structure must match the RP’s API response.

| Field        | Type      | JSON Name    | Description                                                         |
|--------------|-----------|--------------|---------------------------------------------------------------------|
| `public_key` | `Vec<u8>` | `public_key` | RP’s ephemeral public key used for key exchange.                    |
| `t_b_hash`   | `Vec<u8>` | `t_b_hash`   | Tunnel binding hash for integrity verification.                     |
| `int_rp_jwt` | `String`  | `jwt1`       | JWT for authentication between the Interceptor and Reverse Proxy.   |
| `fp_rp_jwt`  | `String`  | `jwt2`       | JWT for authentication between the Forward Proxy and Reverse Proxy. |

---

## 4. `InitTunnelResponseToINT`

**Description:**  
Response sent to the **Interceptor (INT)** after processing the RP’s response. It includes additional NTor-related
information required by the INT.

| Field                    | Type      | JSON Name              | Description                                     |
|--------------------------|-----------|------------------------|-------------------------------------------------|
| `ephemeral_public_key`   | `Vec<u8>` | `ephemeral_public_key` | Ephemeral public key for secure communication.  |
| `t_b_hash`               | `Vec<u8>` | `t_b_hash`             | Tunnel binding hash for integrity verification. |
| `int_rp_jwt`             | `String`  | `jwt1`                 | JWT for authentication between INT and RP.      |
| `int_fp_jwt`             | `String`  | `jwt2`                 | JWT for authentication between INT and FP.      |
| `ntor_static_public_key` | `Vec<u8>` | `public_key`           | Static public key of the NTor server.           |
| `ntor_server_id`         | `String`  | `server_id`            | Unique identifier of the NTor server.           |

---

## 5. `FpHealthcheckSuccess`

**Description:**  
Indicates a successful health check of the **Forward Proxy (FP)**.

| Field                    | Type     | Visibility   | Description                                        |
|--------------------------|----------|--------------|----------------------------------------------------|
| `fp_healthcheck_success` | `String` | `pub(crate)` | Confirmation message indicating FP is operational. |

---

## 6. `FpHealthcheckError`

**Description:**  
Represents a failure during the **Forward Proxy (FP)** health check.

| Field                  | Type     | Visibility   | Description                                        |
|------------------------|----------|--------------|----------------------------------------------------|
| `fp_healthcheck_error` | `String` | `pub(crate)` | Error message describing the health check failure. |

---

## 7. `NTorServerCertificate`

**Description:**  
Represents the identity of an NTor server used in cryptographic handshakes. This structure is internal to the crate.

| Field        | Type      | Visibility | Description                           |
|--------------|-----------|------------|---------------------------------------|
| `server_id`  | `String`  | Private    | Unique identifier of the NTor server. |
| `public_key` | `Vec<u8>` | Private    | Static public key of the NTor server. |

---

## 8. `IntFPSession`

**Description:**  
Maintains session information between the **Interceptor (INT)** and the **Forward Proxy (FP)**.

| Field         | Type     | Description                                                              |
|---------------|----------|--------------------------------------------------------------------------|
| `client_id`   | `String` | Unique identifier for the client session.                                |
| `rp_base_url` | `String` | Base URL of the Reverse Proxy associated with the session.               |
| `fp_rp_jwt`   | `String` | JWT used for authentication between the Forward Proxy and Reverse Proxy. |

**Traits:** `Clone`, `Debug`, `Default`

---

## 9. `RpHealthcheckSuccess`

**Description:**  
Indicates a successful health check of the **Reverse Proxy (RP)**.

| Field                    | Type     | Visibility   | Description                                        |
|--------------------------|----------|--------------|----------------------------------------------------|
| `rp_healthcheck_success` | `String` | `pub(crate)` | Confirmation message indicating RP is operational. |

---

## 10. `RpHealthcheckError`

**Description:**  
Represents a failure during the **Reverse Proxy (RP)** health check.

| Field                  | Type     | Visibility   | Description                                        |
|------------------------|----------|--------------|----------------------------------------------------|
| `rp_healthcheck_error` | `String` | `pub(crate)` | Error message describing the health check failure. |

---

## 11. `InitEncryptedTunnelRequest`

**Description:**  
Represents a request to initialize an encrypted tunnel, similar to `InitTunnelRequest`, typically used in an additional
security layer.

| Field        | Type      | Description                                                     |
|--------------|-----------|-----------------------------------------------------------------|
| `public_key` | `Vec<u8>` | Client’s public key used for establishing the encrypted tunnel. |

---

## 12. `InitEncryptedTunnelResponse`

**Description:**  
Response returned after successfully establishing an encrypted tunnel.

| Field        | Type      | JSON Name    | Description                                             |
|--------------|-----------|--------------|---------------------------------------------------------|
| `public_key` | `Vec<u8>` | `public_key` | Server’s ephemeral public key for the encrypted tunnel. |
| `t_b_hash`   | `Vec<u8>` | `t_b_hash`   | Tunnel binding hash for integrity verification.         |
| `int_rp_jwt` | `String`  | `jwt1`       | JWT for authentication between INT and RP.              |
| `fp_rp_jwt`  | `String`  | `jwt2`       | JWT for authentication between FP and RP.               |

---

## 13. `L8RequestObject`

**Description:**  
Represents an HTTP request in a JSON-serializable format for transmission through the secure tunnel.

| Field     | Type                                 | Description                                         |
|-----------|--------------------------------------|-----------------------------------------------------|
| `method`  | `String`                             | HTTP method (e.g., `GET`, `POST`, `PUT`, `DELETE`). |
| `uri`     | `String`                             | Target URI of the request.                          |
| `headers` | `HashMap<String, serde_json::Value>` | HTTP headers represented as key-value pairs.        |
| `body`    | `Vec<u8>`                            | Raw request body in bytes.                          |

---

## 14. `L8ResponseObject`

**Description:**  
Represents an HTTP response in a JSON-serializable format returned through the secure tunnel.

| Field         | Type                                 | Description                                                   |
|---------------|--------------------------------------|---------------------------------------------------------------|
| `status`      | `u16`                                | HTTP status code (e.g., `200`, `404`).                        |
| `status_text` | `String`                             | Human-readable description of the status code (e.g., `"OK"`). |
| `headers`     | `HashMap<String, serde_json::Value>` | HTTP headers represented as key-value pairs.                  |
| `body`        | `Vec<u8>`                            | Raw response body in bytes.                                   |
| `ok`          | `bool`                               | Indicates whether the response status is successful (`2xx`).  |
| `url`         | `String`                             | Final URL of the response after redirects.                    |
| `redirected`  | `bool`                               | Indicates whether the request was redirected.                 |

---

## 🔗 Relationships Overview

```text
InitTunnelRequest / InitEncryptedTunnelRequest
                │
                ▼
     Reverse Proxy (RP)
                │
                ▼
InitTunnelResponseFromRP
                │
                ▼
InitTunnelResponseToINT
                │
                ▼
          Interceptor (INT)
                │
                ▼
            Forward Proxy (FP)

Health Checks:
- FpHealthcheckSuccess / FpHealthcheckError
- RpHealthcheckSuccess / RpHealthcheckError

Application Data Flow:
L8RequestObject  ─────────►  Secure Tunnel  ─────────►  L8ResponseObject
```


