# Cross-compile Rust backend as fully static musl binary (works on any Linux)
FROM rust:1-alpine AS builder

RUN apk add --no-cache musl-dev pkgconfig openssl-dev openssl-libs-static sqlite-dev sqlite-static perl make

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src/ src/
COPY migrations/ migrations/

ENV OPENSSL_STATIC=1
ENV OPENSSL_DIR=/usr
ENV SQLITE3_STATIC=1

RUN cargo build --release --target x86_64-unknown-linux-musl

# Output: fully static binary
FROM scratch AS export
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/homesoil /homesoil
