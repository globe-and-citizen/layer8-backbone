FROM rust:1.88 AS builder

# Install system dependencies
# RUN apt-get update && apt-get install -y \
#     openssl \
#     musl-tools \
#     pkg-config \
#     libssl-dev \
#     build-essential \
#     clang \
#     cmake \
#     llvm \
#     perl \
#     ca-certificates \
#     && rm -rf /var/lib/apt/lists/*

RUN apt-get update && apt-get install -y cmake gcc sshguard pkg-config libssl-dev git build-essential libclang-dev

# Use vendored OpenSSL to avoid all system library issues
# ENV OPENSSL_STATIC=1
# ENV OPENSSL_VENDORED=1

RUN mkdir /app
WORKDIR /app
COPY . .

# RUN apt-file list libssl-dev | grep libssl.a | awk '{ print $2 }'

# Build the application in release mode
# RUN OPENSSL_DIR=/usr/ OPENSSL_LIB_DIR=/usr/lib/aarch64-linux-gnu/ PKG_CONFIG_PATH=/usr/lib/aarch64-linux-gnu/pkgconfig/ cargo build --release
RUN cargo build --release

EXPOSE 6191

# Run the binary
CMD ["./target/release/forward-proxy"]