FROM rust:1.70-slim-buster AS builder

WORKDIR /app

# 依存関係のキャッシュ
COPY Cargo.toml Cargo.lock askama.toml ./

COPY src ./src
RUN ls -la ./src
COPY templates ./templates
RUN ls -la ./templates
COPY static ./static
RUN ls -la ./static

# RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release
RUN rm -rf src

#CMD ["/app/target/release/websocket_rust"]


# 実行ステージ
FROM debian:buster-slim

RUN apt-get update && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/websocket_rust /usr/local/bin/websocket_rust

CMD ["websocket_rust"]
