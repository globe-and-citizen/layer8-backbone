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

# Create a non-root user for better security (optional)
RUN useradd -m layer8

# Copy only the built binary from the builder stage
COPY --from=builder /usr/src/app/reverse-proxy/target/release/reverse-proxy /usr/local/bin/reverse-proxy

# Switch to non-root user
USER layer8

EXPOSE 6193

# Default command
CMD ["reverse-proxy"]
