FROM ubuntu:latest

RUN apt-get update && apt-get install -y \
    curl \
    cmake \
    build-essential \
    pkg-config \
    openssl \
    libssl-dev \
    perl \
    git \
    clang

# Install Rust
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

RUN mkdir /app
WORKDIR /app
COPY . .

# FIXME: We have hardcoded the `x86_64` architecture here.
RUN OPENSSL_DIR=/usr/ OPENSSL_LIB_DIR=/usr/lib/x86_64-linux-gnu/ OPENSSL_NO_VENDOR=Y cargo build --release

EXPOSE 6191

# Run the binary
CMD ["./target/release/forward-proxy"]