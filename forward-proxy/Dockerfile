FROM ubuntu:latest

RUN apt-get update && apt-get install -y \
    curl \
    cmake \
    build-essential \
    pkg-config \
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

RUN cargo build --release

EXPOSE 6191

# Run the binary
CMD ["./target/release/forward-proxy"]