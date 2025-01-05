use bb8_redis::{bb8, RedisConnectionManager};

use serde :: Deserialize;
use serde :: Serialize;
use tokio::sync::broadcast;
use tokio::sync::broadcast::Sender;
// use core::error;
use std::sync::Arc;
use once_cell::sync::OnceCell;
use bb8::PooledConnection;
use axum::http::StatusCode;
use axum::Json;
use redis::{AsyncCommands, RedisError};
use std::time::Duration;
//use redis::aio::MultiplexedConnection;



pub struct AppState {
    pub pool: bb8::Pool<RedisConnectionManager>,
    pub tx: broadcast::Sender<ChatMessage>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ChatMessage {
    pub user_id: String,
    pub to_id: String,
    pub message: MessageContent,
}

#[derive(Clone, Debug,Deserialize , Serialize)]
#[serde(untagged)]
pub enum MessageContent {
    Text(String),
    Sdp(Sdp),
    Ice(Ice),
    Close(Close),  // Close is not protocol , use for closing connection
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Sdp {
    pub r#type: String,
    pub sdp: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Ice{
    pub r#type: String,
    pub candidate: Candidate,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Close{
    pub r#type: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[allow(non_snake_case)]
pub struct Candidate {
    pub candidate: String,
    pub sdpMid: String,
    pub sdpMLineIndex: u32,
}

impl std::fmt::Display for MessageContent {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            MessageContent::Text(text) => write!(f, "{}", text),
            MessageContent::Sdp(sdp) => write!(f, "Sdp(type: {}, sdp: {})", sdp.r#type, sdp.sdp),
            MessageContent::Ice(ice) => write!(f, "Ice(type: {}, candidate: {})", ice.r#type, ice.candidate.candidate),
            MessageContent::Close(close) => write!(f, "Close(type: {})", close.r#type),
        }
    }
}

impl AppState {
    pub async fn get_redis_conn(&self) -> Result<PooledConnection<'_,RedisConnectionManager>, (StatusCode, Json<serde_json::Value>)> {
        match self.pool.get().await {
            Ok(conn) => Ok(conn),
            Err(e) => {
                eprintln!("Redis connection error: {:?}", e);
                Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({
                        "error": "Internal server error",
                        "message": "Failed to connect to Redis"
                    })),
                ))
            }
        }
    }
}

static APP_STATE: OnceCell<Arc<AppState>> = OnceCell::new();

pub fn init_app_state(pool: bb8::Pool<RedisConnectionManager>, tx: Sender<ChatMessage>) {
    let _ = APP_STATE.set(Arc::new(AppState { pool, tx  }));
}

pub fn get_app_state() -> &'static Arc<AppState> {
    APP_STATE.get().expect("AppState not initialized")
}

pub async fn create_pool(redis_url: &str) -> Result<bb8::Pool<RedisConnectionManager>, Box<dyn std::error::Error>> {
    let manager = RedisConnectionManager::new(redis_url)?;
    let pool = bb8::Pool::builder()
        .connection_timeout(Duration::from_secs(60))
        .max_size(15)
        .build(manager)
        .await?;
    
    Ok(pool)
}

pub async fn get_connection(pool: &bb8::Pool<RedisConnectionManager>) -> Result<bb8::PooledConnection<'_, RedisConnectionManager>, Box<dyn std::error::Error>> {
    let conn = pool.get().await?;
    Ok(conn)
}

async fn publish(pool: &bb8::Pool<RedisConnectionManager>, channel: &str, message: &str) -> redis::RedisResult<()> {
    let mut conn = pool.get().await.unwrap();
    conn.publish(channel, message).await
}


fn connect_to_redis(redis_url: &str) -> Result<redis::Connection, RedisError> {
    let client = redis::Client::open(redis_url)?;
    let con = client.get_connection()?;
    Ok(con)
}


fn subscribe_channel(channel: &str,redis_url: &str) -> redis::RedisResult<()> {

    //　Redisへ接続
    //let client = redis::Client::open(redis_url)?;
    //let mut con = client.get_connection()?;

    let conn = match connect_to_redis(redis_url) {
        Ok(con) => {
            println!("Successfully connected to Redis!");
            // 接続を使用して操作を行う
            Some(con)
        },
        Err(e) => {
            let error_message = format!("{}", e);
            println!("Failed to connect to Redis: {}", error_message);
            None
            // エラー処理を行う
        }
    };

    match conn {
        Some(mut con) => {
            let mut pubsub = con.as_pubsub();
            match  pubsub.subscribe(channel){
                Ok(_) => {
                    println!("subscribed channel: {}", channel);
                },
                Err(e) => {
                    let error_message = format!("{}", e);
                    println!("Failed to subscribe channel: {}", error_message);
                }
            }

            loop {
                let msg = pubsub.get_message()?;
                let payload : String = msg.get_payload()?;
                println!("channel '{}': {}", msg.get_channel_name(), payload);
        
                if payload == "exit" {
                    break;
                }
            }
        
        },
        None => {
            println!("Failed to get Redis connection");
        }
    }

    return Ok(());
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockall::predicate::*;
    use mockall::mock;
    use dotenv::dotenv;
    use std::env;

    mock! {
        RedisConnectionManager {}
        impl Clone for RedisConnectionManager {
            fn clone(&self) -> Self;
        }
    }

    mock! {
        Pool<RedisConnectionManager> {}
    }

    #[tokio::test]
    async fn test_create_pool_success() {
        dotenv().ok();
        let redis_url = env::var("REDIS_URL").expect("REDIS_URL must be set in .env file");

        let result = create_pool(&redis_url).await;

        // 結果を検証
        assert!(result.is_ok());

        let pool = result.unwrap();
        let result_con  = get_connection(&pool).await;

        assert!(result_con.is_ok());

        let _con = result_con.unwrap();

        let state = pool.state();
        let count = state.connections;

        let _= publish(&pool, "test_channel", "test_message").await;
        let _= publish(&pool, "test_channel", "exit").await;

        // 結果を検証
        assert_eq!(count, 1);

        // 結果を検証
        let result = subscribe_channel("test_channel", &redis_url);

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_create_pool_failure() {
        let redis_url = "invalid_url";

        // テスト実行
        let result = create_pool(redis_url).await;

        assert!(result.is_err());
    }

    //#[tokio::test]
    async fn test_get_connection() {
        dotenv().ok();
        let redis_url = env::var("REDIS_URL").expect("REDIS_URL must be set in .env file");
        let pool = create_pool(&redis_url).await.unwrap();

        let _= publish(&pool, "test_channel", "test_message").await;
        let _= publish(&pool, "test_channel", "exit").await;
        // 結果を検証
        let result = subscribe_channel("test_channel", &redis_url);

        assert!(result.is_ok());
    }


}



