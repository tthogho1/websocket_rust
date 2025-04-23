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

use redis::{Client, AsyncCommands};
use crate::handlers::position::delete_user;

use std::time::Duration;
use tokio::time::sleep; // 

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
                    let _ = sleep(Duration::from_secs(5));
                }
            }
        }
    }
    Err(redis::RedisError::from((redis::ErrorKind::IoError, "Over max retry count for redis connection")))
}


pub async fn redis_listener() {
    let redis_url = env::var("REDIS_URL").unwrap().to_string();
    let state = Arc::clone(get_app_state());

    let client = Arc::new(Client::open(redis_url).unwrap());
    let mut con = connect_with_retry(&client, 10).unwrap();
    let mut pubsub = con.as_pubsub();
    pubsub.subscribe("my_channel").unwrap();

    println!("Listening for messages on channel 'my_channel'");
    loop {
        match pubsub.get_message() {
            Ok(msg) => {
                if let Ok(payload) = msg.get_payload::<String>() {
                    println!("get message from channel '{}': {}", msg.get_channel_name(), payload);

                    match serde_json::from_str::<ChatMessage>(&payload) {
                        Ok(chat_message) => {
                            println!("send message to tx.send {}", payload);
                            if let Err(e) = state.tx.send(chat_message) {
                                eprintln!("Failed to send message: {}", e);
                            }
                        },
                        Err(e) => eprintln!("Failed to parse message: {}", e),
                    }
                }
            },
            Err(e) => {
                eprintln!("Failed to get message from channel: {}", e);
                // wait to retry
                let _ = tokio::time::sleep(Duration::from_secs(3)).await;
            },
        }
    }
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
    let app_state = Arc::clone(get_app_state());
    
    let pool = app_state.pool.clone();
    
    // ブロードキャストチャンネルの受信機を取得
    let mut rx = state.tx.subscribe();

    let mut send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            println!("receive from rx {}", msg.message);

            if !msg.to_id.is_empty(){ 
                if msg.to_id == user_id_clone {
                    println!("{}: {}", msg.user_id, msg.message);
                    let json_string = serde_json::to_string(&msg).unwrap();
                    let _ = sender
                            .send(Message::Text(json_string))
                            .await;                        
                }
            }else{
                let json_string = serde_json::to_string(&msg).unwrap();
                println!(" send to {}: {}", msg.user_id, msg.message);
                let _ = sender
                .send(Message::Text(json_string))
                .await; 
            } 
        }
    });

    let redis_client = Arc::new(state.redis_client.clone());
    // クライアントからのメッセージを処理するタスク
    let mut recv_task = tokio::spawn(async move {

        while let Some(Ok(Message::Text(jsontext))) = receiver.next().await {
            // 受信したメッセージをブロードキャスト
            println!("{}: {}", user_id.clone(), jsontext);
            
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
            // let mut con = pool.get().await.unwrap(); 
            // let json_string = serde_json::to_string(&chat_message).unwrap();    
            // let json_string_clone = json_string.clone();

            // let _: () = con.publish("my_channel", json_string).await.unwrap();
            // println!("publish message to redis {}", json_string_clone);
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
    let _ = delete_user(State(app_state), delete_id).await.unwrap();
    
}