use axum::{
    extract::State,
    extract::Json,
    response::IntoResponse,
    http::StatusCode,
};
use bb8_redis::RedisConnectionManager;
use std::sync::Arc;
use serde::{Deserialize, Serialize };
use bb8::PooledConnection;
use crate::AppState;  
use redis::AsyncCommands;

//
// connection poolからRedisコネクションを取得 する機能を追加
//
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

#[derive(Deserialize, Serialize)]
pub struct Location {
    lat: f64,
    lng: f64,
}

#[derive(Deserialize, Serialize)]
pub struct UserData {
    name: String,
    location: Location
}

#[derive(Serialize)]
pub struct PositionResponse {
    message: String,
}

pub async fn position_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<UserData>,
) -> impl IntoResponse {
    println!("{}: {}, {}", payload.name, payload.location.lat, payload.location.lng);
    let key_name = payload.name.clone();
    let json_string = serde_json::to_string(&payload).unwrap();
    
    let mut con = state.get_redis_conn().await.unwrap();
    let result: Result<(), redis::RedisError> = con.set(&key_name, &json_string).await;

    match result {
        Ok(_) => {
            println!("Data stored successfully {}", key_name);
            return Json(PositionResponse { message: "OK".to_string() }).into_response();
        }
        Err(e) => {
            // Log the error
            eprintln!("Redis error: {:?}", e);
    
            // Return an error response
            let error_response = PositionResponse {
                message: format!("Error storing data: {}", e)
            };
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(error_response)).into_response();
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct UserListResponse {
    users: Vec<UserData>,
}

#[derive(Deserialize, Serialize)]
pub struct UserQuery {
    name: String,
}

pub async fn getallusers_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<UserQuery>,
) -> impl IntoResponse {
    let mut con = state.get_redis_conn().await.unwrap();
    let username = if payload.name.is_empty() {
        "*".to_string()
    } else {
        payload.name.clone()
    };
    
    // すべてのキーを取得
    let keys: Vec<String> = match con.keys(&username).await {
    // let keys: Vec<String> = match con.keys("*").await {
        Ok(k) => k,
        Err(e) => {
            eprintln!("Redis keys error: {:?}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(UserListResponse { users: vec![] }),
            )
                .into_response();
        }
    };

    let mut users = Vec::new();

    // 各キーに対応する値を取得
    for key in keys {
        let value: String = match con.get(&key).await {
            Ok(v) => v,
            Err(e) => {
                eprintln!("Redis get error for key {}: {:?}", key, e);
                continue; 
            }
        };

        // JSON文字列をUserDataにデシリアライズ
        match serde_json::from_str::<UserData>(&value) {
            Ok(user_data) => users.push(user_data),
            Err(e) => {
                eprintln!("JSON parse error for key {}: {:?}", key, e);
                continue; 
            }
        }
    }
    println!("return {} users", users.len());
    Json(UserListResponse { users }).into_response()
}