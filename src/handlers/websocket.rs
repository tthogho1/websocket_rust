use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade}, Query, State
    }, 
    response::IntoResponse
};

use futures::{sink::SinkExt, stream::StreamExt};
use serde::Deserialize;
use std::sync::Arc;

use websocket_rust::AppState;
use websocket_rust::ChatMessage;
use websocket_rust::get_app_state;

use crate::handlers::position::delete_user;


#[derive(Deserialize)]
pub struct WsParams {
    name: String,
}

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    Query(params): Query<WsParams>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    println!("{}: {}", params.name, "WebSocket connection established");

    ws.on_upgrade(|socket| handle_socket(socket, state, params.name))
}

async fn handle_socket(socket: WebSocket, state: Arc<AppState>, name: String) {
    let (mut sender, mut receiver) = socket.split();

    let user_id = name;
    let user_id_clone = user_id.clone();
    let delete_id = user_id.clone();

    // ブロードキャストチャンネルの受信機を取得
    let mut rx = state.tx.subscribe();

    // 受信したメッセージをブロードキャストするタスク
    let mut send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            if msg.to_id.is_empty() || (msg.to_id == user_id_clone) {
                let _ = sender
                .send(Message::Text(format!("{}: {}", msg.user_id, msg.message)))
                .await;                
            }
        }
    });

    // クライアントからのメッセージを処理するタスク
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(Message::Text(jsontext))) = receiver.next().await {
            // 受信したメッセージをブロードキャスト
            println!("{}: {}", user_id.clone(), jsontext);
            let chat_message: ChatMessage = serde_json::from_str(&jsontext).unwrap();

            let _ = state.tx.send(chat_message).unwrap();
        }
    });

    // どちらかのタスクが終了するまで待機
    tokio::select! {
        _ = (&mut send_task) => recv_task.abort(),
        _ = (&mut recv_task) => send_task.abort(),
    };

    let delete_id_clone = delete_id.clone();
    println!("WebSocket connection closed {}", delete_id_clone);

    // delete user for redis if exists
    let app_state = Arc::clone(get_app_state());
    let _ = delete_user(State(app_state), delete_id).await.unwrap();
    
}