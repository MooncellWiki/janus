FROM rust:1.97-trixie AS build-stage
ENV SQLX_OFFLINE=true
WORKDIR /app
COPY . /app/
RUN cargo build --all --release

FROM debian:13
RUN apt-get update && apt-get -y install ca-certificates
WORKDIR /app
COPY --from=build-stage /app/target/release/janus /app
