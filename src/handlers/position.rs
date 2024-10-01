use axum::{
    extract::State,
    extract::Json,
    extract::rejection::JsonRejection,
    response::IntoResponse,
};
use std::sync::Arc;
use serde::{Deserialize, Serialize };
//use serde_json::json;
use crate::AppState;  // AppStateをmain.rsから参照
use redis::AsyncCommands;

#[derive(Deserialize, Serialize)]
pub struct UserData {
    name: String,
    latitude: f64,
    longitude: f64,
}

#[derive(Serialize)]
pub struct PositionResponse {
    message: String,
}

#[derive(Debug)]
enum AppError {
    JsonError(JsonRejection),
    RedisError(redis::RedisError),
}

pub async fn position_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<UserData>,
) -> impl IntoResponse {
    println!("{}: {}, {}", payload.name, payload.latitude, payload.longitude);
    let key_name = payload.name.clone();
    let json_string = serde_json::to_string(&payload).unwrap();
    //let redis_string = redis::Value::from(json_string);
    
    let mut con = state.pool.get().await.unwrap();
    let _: () = con.set(&key_name, &json_string).await.map_err(|e| AppError::RedisError(e)).expect("set error");
    // ハンドラーの実装
    ( Json(PositionResponse { message: "OK".to_string() }) ).into_response()
}