FROM rust:1.85-bookworm AS builder
WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY crates crates
RUN cargo build --release --workspace

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /build/target/release/anitranslate /usr/local/bin/

ENV ANITRANSLATE_DATA_DIR=/data
VOLUME ["/data"]
WORKDIR /work

ENTRYPOINT ["/usr/local/bin/anitranslate"]
CMD ["--help"]
