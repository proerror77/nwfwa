FROM rust:1.96-bookworm AS builder

WORKDIR /workspace
COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY crates ./crates
COPY apps ./apps
ENV CARGO_INCREMENTAL=0 \
    CARGO_PROFILE_DEV_DEBUG=0
RUN cargo build --locked -p api-server

FROM debian:bookworm-slim AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /workspace/target/debug/api-server /usr/local/bin/api-server
EXPOSE 8080
CMD ["api-server"]
