## Stage 1: Builder
##FROM rust:1.77 as builder
#FROM rustlang/rust:nightly-bullseye AS builder
#
#WORKDIR /usr/src/forward-proxy
#
## Pre-cache dependencies
#COPY ./forward-proxy/Cargo.toml ./
##COPY ./forward-proxy/Cargo.lock ./
#RUN mkdir src && echo "fn main() {}" > src/main.rs
#RUN cargo build --release
#RUN rm -rf src
#
## Copy actual source
#COPY . .
#RUN cargo build --release
#
## Stage 2: Runtime
#FROM debian:bullseye-slim
#
## Create a non-root user (optional, for security)
#RUN useradd -m appuser
#
## Copy binary from builder
#COPY --from=builder /usr/src/forward-proxy/target/release/forward-proxy /usr/local/bin/forward-proxy
#
## Use non-root user
#USER appuser
#
## Run the binary
#CMD ["forward-proxy"]

# Use the official Rust image
FROM rust:1.87

#RUN apt-get update && apt-get install -y curl

RUN apt-get update && apt-get install -y curl \
  cmake \
  perl \
  git \
  pkg-config \
  clang \
  build-essential \
  ninja-build \
  && rm -rf /var/lib/apt/lists/*

# Set working directory
WORKDIR /usr/src/myapp

# Copy source code
COPY ./pingora-router ./pingora-router
COPY ./utils ./utils
COPY ./forward-proxy ./forward-proxy

WORKDIR /usr/src/myapp/forward-proxy
# Build the application
RUN cargo update
RUN cargo build --release

# Run the app
CMD ["./target/release/myapp"]
