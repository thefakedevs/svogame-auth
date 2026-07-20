# syntax=docker/dockerfile:1.7

FROM rust:1.90-alpine3.22 AS builder

WORKDIR /app

RUN apk add --no-cache \
    build-base \
    musl-dev \
    openssl-dev \
    openssl-libs-static \
    curl

RUN rustup target add x86_64-unknown-linux-musl

COPY Cargo.toml Cargo.lock ./
COPY src ./src

RUN --mount=type=cache,id=auth-cargo-registry,target=/usr/local/cargo/registry \
    --mount=type=cache,id=auth-cargo-git,target=/usr/local/cargo/git \
    --mount=type=cache,id=auth-target-musl,target=/app/target \
    cargo build \
        --locked \
        --target x86_64-unknown-linux-musl \
        --release && \
    cp /app/target/x86_64-unknown-linux-musl/release/auth /tmp/auth


FROM alpine:3.22

WORKDIR /app

RUN apk add --no-cache ca-certificates

COPY --from=builder /tmp/auth /app/auth

EXPOSE 3000

ENV RUST_BACKTRACE=1

CMD ["/app/auth"]