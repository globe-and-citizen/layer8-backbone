# Layer8 Smoke E2E Tests

Self-contained smoke tests for the Forward Proxy (FP) and Reverse Proxy (RP) using Docker Compose and public container images.

## Architecture

```
[smoke-test.js]
     │
     ▼ HTTP
[forward-proxy :6191]  ←── plain HTTP from test runner
     │
     ▼ mTLS (cert from step-ca)
[reverse-proxy :6193]
     │
     ▼ HTTP
[spa-backend   :3000]

[mock-auth     :5001]  ←── FP fetches NTor cert from here
[step-ca       :9000]  ←── issues mTLS certs to FP and RP via ACME
[influxdb2     :8086]  ←── FP writes usage metrics here
```

## Prerequisites

- Docker Engine with the Compose plugin (`docker compose`)
- Node.js 20+

## Quick Start

```sh
cd test
npm run smoke:run
```

This runs `smoke:up` → `smoke:wait` → `smoke` in sequence.

## Step-by-Step

```sh
cd test

# 1. Build images and start all services in the background
npm run smoke:up

# 2. Wait until FP, mock-auth and spa-backend are reachable
#    (FP and RP obtain mTLS certs from step-ca via ACME during this time)
npm run smoke:wait

# 3. Run smoke assertions
npm run smoke

# 4. Tear down containers and volumes
npm run smoke:down
```

## What Is Tested

| # | Check | Description |
|---|-------|-------------|
| 1 | `GET FP /healthcheck` | Forward Proxy is up and accepting connections |
| 2 | `GET mock-auth /healthcheck` | Mock auth server is up |
| 3 | `GET backend /healthcheck` | SPA backend is up |
| 4 | `POST FP /init-tunnel` | FP→RP mTLS chain works end-to-end (NTor handshake) |
| 5 | Interceptor `fetch` via FP `/proxy` | Encrypted proxy request succeeds end-to-end |

## Environment Variables

The smoke test runner (`smoke-test.js`) accepts the following variables:

| Variable | Default | Description |
|----------|---------|-------------|
| `FP_URL` | `http://localhost:6191` | Forward Proxy base URL |
| `BACKEND_URL` | `http://localhost:3000` | SPA backend base URL |
| `MOCK_AUTH_URL` | `http://localhost:5001` | Mock auth server base URL |
| `PROXY_TARGET_URL` | same as `RP_BACKEND_URL` | Reverse Proxy server identity URL used by interceptor `fetch` proxy validation; must match the URL used for init-tunnel |
| `RP_BACKEND_URL` | `https://reverse-proxy:6193` | Reverse Proxy URL as seen from FP within the Docker network (this is the mTLS endpoint sent in init-tunnel); must exactly match RP's `NTOR_SERVER_ID` |
| `INIT_TUNNEL_RETRIES` | `20` | Maximum number of init-tunnel attempts, including the initial attempt |
| `INIT_TUNNEL_RETRY_DELAY_MS` | `3000` | Milliseconds between retries |
| `PROXY_REQUEST_TIMEOUT_MS` | `15000` | Timeout for interceptor proxy healthcheck request |
| `PROXY_REQUEST_RETRIES` | `10` | Max attempts for the interceptor proxy healthcheck after init-tunnel succeeds |
| `PROXY_REQUEST_RETRY_DELAY_MS` | `2000` | Milliseconds between interceptor proxy healthcheck retries |

## Services

| Service | Exposed Port | Image / Build |
|---------|-------------|---------------|
| `step-ca` | — | `smallstep/step-ca:latest` |
| `influxdb2` | — | `influxdb:2` |
| `mock-auth` | `5001` | Built from `test/mock-auth/` |
| `spa-backend` | `3000` | Built from `spa/backend/` |
| `forward-proxy` | `6191` | Built from repo root with `forward-proxy/Dockerfile` |
| `reverse-proxy` | `6193` | Built from repo root with `reverse-proxy/Dockerfile` |

> **Note:** `step-ca` and `influxdb2` are not exposed on the host — they communicate over the internal `layer8-smoke` Docker network.

## Certificate Flow

On first start, FP and RP each:

1. Download the root CA certificate from step-ca.
2. Request a leaf certificate via the ACME HTTP-01 challenge (`step ca certificate …`).
3. Start the proxy binary only after the certificate is available.

Certificates are stored in named Docker volumes (`fp-certs`, `rp-certs`) and reused on subsequent restarts. Run `npm run smoke:down` to remove volumes and start fresh.

## Troubleshooting

```sh
# Tail logs for all services
npm run smoke:logs

# Tail logs for a specific service
docker compose -f docker-compose.smoke.yml logs -f forward-proxy

# Check whether FP obtained its certificate
docker compose -f docker-compose.smoke.yml exec forward-proxy ls -la /certs/
```

**The first run is slow.** The Rust binaries are compiled from source (no pre-built cache). Expect 5 to 15 minutes depending on hardware. Subsequent runs reuse the Docker layer cache and should start in under a minute.

**RP not ready yet.** The init-tunnel check retries according to `INIT_TUNNEL_RETRIES` and `INIT_TUNNEL_RETRY_DELAY_MS` (defaults: 20 attempts, 3000ms delay). The interceptor proxy healthcheck also retries according to `PROXY_REQUEST_RETRIES` and `PROXY_REQUEST_RETRY_DELAY_MS` before the smoke test fails.
