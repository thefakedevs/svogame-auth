FROM rust:1.90-alpine3.22 AS rust-builder
WORKDIR /app

RUN apk add --no-cache build-base musl-dev openssl-dev openssl-libs-static

RUN rustup target add x86_64-unknown-linux-musl
COPY Cargo.toml Cargo.lock ./
COPY src src

RUN cargo build --target x86_64-unknown-linux-musl --release

FROM node:22-alpine3.22 AS node-builder
WORKDIR /app

COPY app/package.json app/package-lock.json* ./
RUN npm install

COPY app .
RUN npm run build

FROM node:22-alpine3.22

WORKDIR /app

RUN apk add --no-cache supervisor nginx

COPY --from=rust-builder /app/target/x86_64-unknown-linux-musl/release/auth /app/auth

COPY --from=node-builder /app/dist /app/frontend/dist
COPY --from=node-builder /app/node_modules /app/frontend/node_modules
COPY app/package.json /app/frontend/
COPY app/vite.config.ts /app/frontend/

COPY supervisord.conf /etc/supervisord.conf
COPY nginx.conf /etc/nginx/nginx.conf

EXPOSE 3000

ENV RUST_BACKTRACE=1

CMD ["/usr/bin/supervisord", "-c", "/etc/supervisord.conf"]
