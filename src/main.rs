mod view;

use view::render_template;

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
    routing::get,
    routing::get_service,
    Router
};
use futures::{sink::SinkExt, stream::StreamExt};
use std::{net::SocketAddr, sync::Arc};
use tokio::sync::broadcast;
use tower_http::services::ServeDir;


// チャットメッセージを表す構造体
#[derive(Clone, Debug)]
struct ChatMessage {
    user_id: String,
    message: String,
}

// アプリケーションの状態
struct AppState {
    tx: broadcast::Sender<ChatMessage>,
}

#[tokio::main]
async fn main() {
    // ブロードキャストチャンネルを作成
    let (tx, _rx) = broadcast::channel(100);
    let app_state = Arc::new(AppState { tx });

    // ルーターを設定
    let app = Router::new()
        .route("/ws", get(ws_handler))
        .with_state(app_state);

    // ルーターを追加
    let app = app.route("/", get(hello_handler));

    // 静的ファイルを追加
    let app = app
        .nest_service("/static", get_service(ServeDir::new("static")).handle_error(
            |error| async move {
                (
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Unhandled internal error: {}", error),
                )
            },
        ));

    // サーバーを起動
    println!("Server running on http://localhost:3000");
    let addr= SocketAddr::from(([0, 0, 0, 0], 3000));
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn hello_handler() -> impl IntoResponse {
    return render_template("test".to_string());
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