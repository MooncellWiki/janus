# syntax=docker/dockerfile:1

FROM rust:1.99.1-trixie AS chef
RUN cargo install cargo-chef --version 0.1.78 --locked
ENV CARGO_PROFILE_RELEASE_STRIP=symbols
WORKDIR /app

FROM chef AS planner
COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY src ./src
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS build-stage
COPY --from=planner /app/recipe.json ./recipe.json
# Keep compiled dependencies in a layer that can be exported to the CI cache.
RUN cargo chef cook --locked --release --bin janus --recipe-path recipe.json
COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY src ./src
ARG BUILD_SHA=dev
RUN BUILD_SHA="$BUILD_SHA" cargo build --locked --release --bin janus

FROM debian:trixie-slim AS runtime
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates libssl3t64 \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=build-stage /app/target/release/janus /app/janus
USER 10001:10001
EXPOSE 25150
CMD ["/app/janus", "server", "--config", "/app/config.toml"]
