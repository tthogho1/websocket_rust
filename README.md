# websocket_rust

## Introduction

[websocket_rust](https://github.com/taiki-ogawa/websocket_rust) is a websocket server based on axum and redis. It is a sample for how to use websocket in Rust
You can connect to a Redis server and save username and location information on the Redis server.

## Installation
git clone https://github.com/taiki-ogawa/websocket_rust
cd websocket_rust
cargo build --release

## env variables
PORT: port number
REDIS_URL: redis url 
WS_SERVER= address for websocket  ex: ws://localhost:8000/ws

## Run
* Place the static folder in the same directory as websocket_rust.exe
* execute websocket_rust.exe

## Usage
http://localhost:8080/Chat