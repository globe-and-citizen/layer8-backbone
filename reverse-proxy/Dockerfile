# ------------------------------
# Stage 1: Build stage
# ------------------------------
#FROM rust:1.87 as builder
FROM rustlang/rust:nightly-bullseye AS builder

# Install required build dependencies for boring-sys, etc.
RUN apt-get update && apt-get install -y \
  curl \
  cmake \
  perl \
  git \
  pkg-config \
  clang \
  build-essential \
  ninja-build \
  && rm -rf /var/lib/apt/lists/*

# Set working directory
WORKDIR /usr/src/app

# Copy source code
COPY ./pingora-router ./pingora-router
COPY ./utils ./utils
COPY ./reverse-proxy ./reverse-proxy
COPY ./certs/scripts ./scripts

# Set working directory to app crate
WORKDIR /usr/src/app/reverse-proxy

# Pre-cache dependencies
RUN cargo fetch

# Build in release mode
RUN cargo build --release

# ------------------------------
# Stage 2: Runtime stage
# ------------------------------
FROM debian:bullseye-slim

ARG TARGETARCH
ENV STEP_CLI_VERSION=0.25.2

# Install runtime deps + step-cli (multi-arch)
RUN apt-get update && apt-get install -y \
    curl \
    ca-certificates \
    libcap2-bin \
    && echo "Installing step-cli for ${TARGETARCH}" \
    && curl -L https://github.com/smallstep/cli/releases/download/v${STEP_CLI_VERSION}/step_linux_${STEP_CLI_VERSION}_${TARGETARCH}.tar.gz \
    | tar -xz \
    && mv step_${STEP_CLI_VERSION}/bin/step /usr/local/bin/step \
    && rm -rf step_${STEP_CLI_VERSION} \
    # Allow non-root to bind to 443 (TLS-ALPN)
    && setcap 'cap_net_bind_service=+ep' /usr/local/bin/step \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

# Create a non-root user for better security (optional)
RUN useradd -m layer8

# Copy only the built binary from the builder stage
COPY --from=builder /usr/src/app/reverse-proxy/target/release/reverse-proxy /usr/local/bin/reverse-proxy
COPY --from=builder /usr/src/app/scripts /scripts

RUN chmod -R 755 /scripts && \
    mkdir -p /certs && \
    chown -R layer8:layer8 /certs /scripts

# Switch to non-root user
USER layer8

EXPOSE 6193

# Entrypoint (scripts should already be executable from host or COPY)
ENTRYPOINT ["/scripts/entrypoint.sh", "reverse-proxy"]
