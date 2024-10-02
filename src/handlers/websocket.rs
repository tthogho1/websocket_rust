use axum::{
    response::IntoResponse,
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    }
};
use std::sync::Arc;
use bb8_redis::RedisConnectionManager;
use tokio::sync::broadcast;
use futures::{sink::SinkExt, stream::StreamExt};

pub struct AppState {
    pub pool: bb8::Pool<RedisConnectionManager>,
    pub tx: broadcast::Sender<ChatMessage>,
}

#[derive(Clone, Debug)]
pub struct ChatMessage {
    pub user_id: String,
    pub message: String,
}


pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: Arc<AppState>) {
    let (mut sender, mut receiver) = socket.split();

    // ユーザーIDを生成（本番環境では適切な認証を行うべきです）
    let user_id = uuid::Uuid::new_v4().to_string();
    let user_id_clone = user_id.clone();

    // ブロードキャストチャンネルの受信機を取得
    let mut rx = state.tx.subscribe();

    // 受信したメッセージをブロードキャストするタスク
    let mut send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            // 自分自身のメッセージは送信しない
          //  if msg.user_id != user_id {
                let _ = sender
                    .send(Message::Text(format!("{}: {}", msg.user_id, msg.message)))
                    .await;
          //  }
        }
    });

    // クライアントからのメッセージを処理するタスク
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(Message::Text(text))) = receiver.next().await {
            // 受信したメッセージをブロードキャスト
            let _ = state.tx.send(ChatMessage {
                user_id: user_id.clone(),
                message: text,
            });
        }
    });

    // どちらかのタスクが終了するまで待機
    tokio::select! {
        _ = (&mut send_task) => recv_task.abort(),
        _ = (&mut recv_task) => send_task.abort(),
    };

    println!("WebSocket connection closed: {}", user_id_clone);
}