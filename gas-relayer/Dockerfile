FROM rust:1-slim-bookworm AS builder

RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
RUN mkdir -p src/bin && \
    echo "fn main() {}" > src/main.rs && \
    echo "fn main() {}" > src/bin/migrate.rs && \
    cargo build --release && \
    rm -rf src \
           target/release/gas-relayer \
           target/release/gas-relayer-migrate \
           target/release/deps/gas_relayer*

COPY src ./src
RUN cargo build --release

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates libssl3 curl && rm -rf /var/lib/apt/lists/*

RUN useradd -m -u 1001 relayer
USER relayer
WORKDIR /home/relayer

COPY --from=builder --chown=relayer:relayer /app/target/release/gas-relayer ./gas-relayer
COPY --from=builder --chown=relayer:relayer /app/target/release/gas-relayer-migrate ./gas-relayer-migrate
COPY --chown=relayer:relayer config.yaml ./config.yaml

EXPOSE 8080

HEALTHCHECK --interval=30s --timeout=5s --start-period=10s \
    CMD curl -f http://localhost:8080/health || exit 1

ENTRYPOINT ["./gas-relayer"]
