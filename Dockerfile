# React サブモジュールをビルドするためのステージ
FROM node:20 as react-builder

#環境変数
ARG REACT_APP_GOOGLE_MAPS_API_KEY
ARG REACT_APP_AWS_PROJECT_REGION
ARG REACT_APP_AWS_COGNITO_IDENTITY_POOL_ID
ARG REACT_APP_AWS_COGNITO_REGION
ARG REACT_APP_AWS_USER_POOLS_ID
ARG REACT_APP_AWS_USER_POOLS_CLIENT_ID
ARG REACT_APP_WEBSOCKET_URL

ENV REACT_APP_GOOGLE_MAPS_API_KEY=$REACT_APP_GOOGLE_MAPS_API_KEY
ENV REACT_APP_AWS_PROJECT_REGION=$REACT_APP_AWS_PROJECT_REGION
ENV REACT_APP_AWS_COGNITO_IDENTITY_POOL_ID=$REACT_APP_AWS_COGNITO_IDENTITY_POOL_ID
ENV REACT_APP_AWS_COGNITO_REGION=$REACT_APP_AWS_COGNITO_REGION
ENV REACT_APP_AWS_USER_POOLS_ID=$REACT_APP_AWS_USER_POOLS_ID
ENV REACT_APP_AWS_USER_POOLS_CLIENT_ID=$REACT_APP_AWS_USER_POOLS_CLIENT_ID
ENV REACT_APP_WEBSOCKET_URL=$REACT_APP_WEBSOCKET_URL


# 作業ディレクトリを設定
WORKDIR /app

# front ディレクトリに移動
WORKDIR /app/front

# 依存パッケージをインストール
COPY front/package*.json ./
RUN npm install

# ソースコードをコピー
COPY front/. .

# React アプリをビルド
RUN npm run build

#
# rust builder
#
FROM rust:1.75-slim-buster AS builder

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
COPY --from=builder /app/static /usr/local/bin/static
COPY --from=react-builder /app/front/build /usr/local/bin/static

EXPOSE 8000

WORKDIR /usr/local/bin
CMD ["websocket_rust"]
