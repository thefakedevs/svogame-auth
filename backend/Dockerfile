FROM rust:1.90-alpine3.22 AS builder
WORKDIR /app

RUN apk add --no-cache build-base musl-dev openssl-dev openssl-libs-static curl

RUN rustup target add x86_64-unknown-linux-musl

COPY Cargo.toml Cargo.lock ./
COPY src ./src

RUN cargo build --target x86_64-unknown-linux-musl --release

FROM alpine:3.22
WORKDIR /app

RUN apk add --no-cache ca-certificates

COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/auth /app/auth

EXPOSE 3000

ENV RUST_BACKTRACE=1

CMD ["/app/auth"]
