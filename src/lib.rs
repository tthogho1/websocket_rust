use bb8_redis::{bb8, RedisConnectionManager};

use serde :: Deserialize;
use serde :: Serialize;
use tokio::sync::broadcast;
use tokio::sync::broadcast::Sender;
use std::sync::Arc;
use once_cell::sync::OnceCell;
use bb8::PooledConnection;
use axum::http::StatusCode;
use axum::Json;

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
    let _ = APP_STATE.set(Arc::new(AppState { pool, tx }));
}

pub fn get_app_state() -> &'static Arc<AppState> {
    APP_STATE.get().expect("AppState not initialized")
}

pub async fn create_pool(redis_url: &str) -> Result<bb8::Pool<RedisConnectionManager>, Box<dyn std::error::Error>> {
    let manager = RedisConnectionManager::new(redis_url)?;
    let pool = bb8::Pool::builder()
        .max_size(15)
        .build(manager)
        .await?;
    Ok(pool)
}

pub async fn get_connection(pool: &bb8::Pool<RedisConnectionManager>) -> Result<bb8::PooledConnection<'_, RedisConnectionManager>, Box<dyn std::error::Error>> {
    let conn = pool.get().await?;
    Ok(conn)
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

        // 結果を検証
        assert_eq!(count, 1);
    }

    #[tokio::test]
    async fn test_create_pool_failure() {
        let redis_url = "invalid_url";

        // テスト実行
        let result = create_pool(redis_url).await;

        assert!(result.is_err());
    }
}



