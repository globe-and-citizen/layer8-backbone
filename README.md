# layer8-backbone
This repo is for where the layer8 backbone will be mocked: frontend, FP, RP, backend, &amp; TIO server

## Project structure

```md
layer8-backbone
├── `certs`
│   ├── `mtls` (deprecated): Contains example mTLS certificates and a manual generation guide.
│   ├── `ntor`: Contains example reverse-proxy's certificate for NTor-protocol and a python script for NTor certificate generation.
│   └── `scripts`: entrypoint scripts for docker build and run
├── `docker` (deprecated): outdated, please use `layer8-docker`
├── **`forward-proxy`: Intercept and manipulate incoming request/outgoing response headers.**
│   ├── `src`: The root directory.
│   │   ├── `handler`: Contains the logic for processing incoming network requests.
│   │   │   ├── `types`: Data structures.
│   │   │   │   ├── `request.rs`: Definitions for incoming request schemas and deserialization logic.
│   │   │   │   ├── `response.rs`: Definitions for outgoing response structures and serialization logic.
│   │   │   │   └── `mod.rs`: The module declaration file that exports the request and response types.
│   │   │   ├── `consts.rs`: Local constants specific to request handling (e.g., timeout limits, default headers).
│   │   │   └── `mod.rs`: The main module file for the handler, defining how requests are routed and processed.
│   │   ├── `statistics`: Update client usage statistics to InfluxDB.
│   │   ├── `config.rs`: Handles application settings, environment variables, and runtime configuration loading.
│   │   ├── `proxy.rs`: The core engine responsible for forwarding traffic, implement and follow pingora-proxy's request lifecycle.
│   │   └── `main.rs`: The entry point of the application; initializes the configuration, starts the services, and manages the runtime loop.
│   ├── Dockerfile
│   ├── .env.dev
│   ├── .env.docker
│   ├── build.rs
│   └── Cargo.toml
├── **`reverse-proxy`: Intercept, wrap and decrypt/encrypt the entire request/response.**
│   ├── `src`: The root directory.
│   │   ├── `handler`: Contains the logic for processing incoming network requests.
│   │   │   ├── `common`: Centralized repository for shared constants, error types, and common data structures used across handlers.
│   │   │   ├── `healthcheck`: Implements the `/healthcheck` endpoint to monitor service availability and internal status.
│   │   │   ├── `init_tunnel`: Manages the `/init-tunnel` handshake logic, establishing secure communication paths for clients and backends.
│   │   │   ├── `proxy`: Handles the logic for the `/proxy` endpoint, responsible for decrypting incoming payloads, 
                    and re-encrypting subsequent responses.
│   │   │   └── `mod.rs`: The module declaration file that exports and organizes the handler sub-modules.
│   │   ├── `config.rs`: Manages application settings, environment variables, and the loading of runtime configurations.
│   │   ├── `tls_conf.rs`: Implements the `TLS_Accept` interface to enforce mTLS (Mutual TLS) verification and handle authentication callbacks.
│   │   ├── `proxy.rs`: The core engine of the service; acts as the primary traffic interceptor and 
                dispatches requests to the appropriate handlers based on the endpoint.
│   │   └── `main.rs`: The entry point of the application; initializes the asynchronous runtime, loads TLS certificates,
                and starts the main event loop to listen for incoming connections.
│   ├── Dockerfile
│   ├── .env.dev
│   ├── .env.docker
│   ├── build.rs
│   └── Cargo.toml
├── `pingora-router`: A specialized wrapper and abstraction layer for the Pingora framework; 
        simplifies API route definitions, middleware integration, and request dispatching logic.
├── `utils`: Contains utility functions.
└── `spa`: Demo single page application.
```

