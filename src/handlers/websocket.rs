//use crate::util::Lock; // クレートルートからの相対パス
use crate::util::Lock::RedisLock;

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade}, Query, State
    }, 
    response::IntoResponse
};

use futures::{sink::SinkExt, stream::StreamExt};
use serde::Deserialize;
use std::sync::Arc;
use std::env;

use websocket_rust::AppState;
use websocket_rust::ChatMessage;
use websocket_rust::{get_app_state, MessageContent};

use redis::{Client, AsyncCommands, RedisError};
use crate::handlers::position::delete_user;

use std::time::Duration;
use tokio::time::sleep; // 
use std::time::Instant;
use tokio::runtime::Runtime;
//use metrics::increment_counter;

// Removed unused import as `lock` module does not exist

#[derive(Deserialize)]
pub struct WsParams {
    name: String,
}

fn connect_with_retry(client: &redis::Client, max_retries: u32) -> redis::RedisResult<redis::Connection> {
    for attempt in 1..=max_retries {
        match client.get_connection_with_timeout(Duration::from_secs(10)) {
            Ok(con) => return Ok(con),
            Err(e) => {
                eprintln!("attempt {} faile: {}", attempt, e);
                if attempt < max_retries {
                    let _ = sleep(Duration::from_secs(10));
                }
            }
        }
    }
    Err(redis::RedisError::from((redis::ErrorKind::IoError, "Over max retry count for redis connection")))
}


pub async fn redis_listener() -> Result<(), RedisError> {
    let redis_url = env::var("REDIS_URL").map_err(|e| RedisError::from((redis::ErrorKind::IoError, "Environment variable error")))?;
    let state = Arc::clone(&get_app_state());
    let client = redis::Client::open(redis_url)?;
    let con = connect_with_retry(&client, 5)?;
    let state_tx_clone = state.tx.clone(); // Sender のクローンを move するクロージャへ移動

    tokio::task::spawn_blocking(move || {
        let mut con = con; // move された con をシャドーイング
        let mut pubsub = con.as_pubsub();
        if let Err(e) = pubsub.subscribe("my_channel") {
            eprintln!("Failed to subscribe to Redis channel: {}", e);
            return Err(e.into()); // redis::RedisError -> crate::RedisError
        }

        println!("'my_channel' subscribed to my_channel");
        let rt = Runtime::new().unwrap();
        while let Ok(msg) = pubsub.get_message() {
            match msg.get_payload::<String>() {
                Ok(payload) => {
                    match serde_json::from_str::<ChatMessage>(&payload) {
                        Ok(chat_message) => {
                            // Avoid attempting a blocking send in spawn_blocking
                            // Perform asynchronous sends with block_on
                            rt.block_on(async {
                                if let Err(e) = state_tx_clone.send(chat_message) {
                                    eprintln!("Failed to send message to broadcast channel: {}", e);
                                } else {
                                    println!("put channel message: {}", payload);
                                }
                            });
                        }
                        Err(e) => eprintln!("Failed to parse message: {}", e),
                    }
                }
                Err(e) => eprintln!("Failed to get payload: {}", e),
            }
        }
        Ok::<(), RedisError>(())
    });

    Ok(())
}

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    Query(params): Query<WsParams>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    println!("{}: {}", params.name, "WebSocket connection established");

    ws.on_upgrade(|socket| handle_socket(socket, state, params.name))
}

async fn process_message(
    msg: ChatMessage,
    sender: Arc<tokio::sync::Mutex<futures::stream::SplitSink<WebSocket, Message>>> ,
    user_id: String,
) -> Result<(), Box<dyn std::error::Error>> {

    // 宛先チェック
    if !msg.to_id.is_empty() && msg.to_id != user_id {
        return Ok(()); // 対象外メッセージは無視
    }

    // JSONシリアライズ
    let json_string = serde_json::to_string(&msg)?;

    // WebSocket送信
    let lock_start = Instant::now();
    println!("Task {}: Attempting to acquire sender lock", user_id);
    let mut sender_guard = sender.lock().await;
    let lock_duration = lock_start.elapsed();
    println!("Task {}: Acquired sender lock in {:?}", user_id, lock_duration);

    let send_start = Instant::now();
    let send_result = sender_guard.send(Message::Text(json_string.clone())).await;
    let send_duration = send_start.elapsed();
    
    match send_result {
        Ok(_) => {
            println!("Task {}: Successfully sent message - {} in {:?}", user_id, &json_string, send_duration);
            Ok(())
        }
        Err(e) => {
            eprintln!("Task {}: Failed to send message - {} due to: {:?}", user_id, &json_string, e);
            Err(Box::new(e))
        }
    }
}

async fn handle_socket(socket: WebSocket, state: Arc<AppState>, name: String) {
    let (sender, mut receiver) = socket.split();
    let sender = Arc::new(tokio::sync::Mutex::new(sender));

    let user_id = name;
    let user_id_clone = user_id.clone();
    let delete_id = user_id.clone();
    let app_state = Arc::clone(get_app_state());
    
    let _pool = app_state.pool.clone();
    
    // ブロードキャストチャンネルの受信機を取得
    println!("WebSocket connection established: {}", user_id);
    let mut rx = state.tx.subscribe();
    println!("Number of subscribers in receiver: {}", state.tx.receiver_count());


    let mut send_task = tokio::spawn(async move {
        let rx_addr = format!("{:p}", &rx);
        println!("send_task started with receiver address: {}", rx_addr);

        while let Ok(msg) = rx.recv().await {
            let sender_clone: Arc<tokio::sync::Mutex<futures::stream::SplitSink<WebSocket, Message>>> = Arc::clone(&sender);
            let user_id = user_id_clone.clone();
            println!("receive id : {}", &user_id);
          //  tokio::spawn(async move {
                // メッセージ処理ロジックの分離
                match process_message(msg, sender_clone, user_id).await {
                    Ok(_) => println!("Message processed successfully"),
                    Err(e) => {
                        println!("Message processing failed: {}", e);
                    }
                }
            //});
        }
    });

    let redis_client = Arc::new(state.redis_client.clone());
    // クライアントからのメッセージを処理するタスク
    let mut recv_task = tokio::spawn(async move {

        while let Some(Ok(Message::Text(jsontext))) = receiver.next().await {
            // 受信したメッセージをブロードキャスト
            println!("recive message : {}: {}", user_id.clone(), jsontext);
            
            let chat_message: ChatMessage = serde_json::from_str(&jsontext).unwrap();
            let  message = chat_message.message.clone();
            // 
            match message {
                MessageContent::Notice(notice) => {
                    println!("Notice type: {}", notice.r#type);
                    if notice.r#type == "OpenVideo" {
                        let lock_key = format!("vchat:user:{}", user_id.clone());
                        // 各タスクで新しい RedisLock インスタンスを作成 (同じ Redis クライアントとロックキーを使用)
                        let redis_lock = RedisLock::new(&redis_client, &lock_key);
                        
                        match redis_lock.lock(10) {
                            Ok(acquired) => {
                                if acquired {
                                    println!("Lock acquired by thread {} for user: {}", tokio::task::id(), user_id.clone());
                                    // ロック保護された処理
                                    // 

                                } else {
                                    println!("Failed to acquire lock by thread {} for user: {}", tokio::task::id(), user_id.clone());
                                }
                            }
                            Err(e) => eprintln!("Error acquiring lock by thread {}: {}", tokio::task::id(), e),
                        }
                        // lock and set user

                    }
                }
                _ => { // do nothinng
                }
            }
            //let get_type = get_message_type(&chat_message);
            let mut con = _pool.get().await.unwrap(); 
            let json_string = serde_json::to_string(&chat_message).unwrap();    
            let json_string_clone = json_string.clone();

            let _: () = con.publish("my_channel", json_string).await.unwrap();
            println!("publish message to redis {}", json_string_clone);
            // original
            // let _ = state.tx.send(chat_message).unwrap();
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
    let _ = delete_user(State(app_state), delete_id).await.unwrap();
    
}