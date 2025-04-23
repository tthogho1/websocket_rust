use axum::{
    extract::State,
    extract::Json,
    response::IntoResponse,
    http::StatusCode,
};
//use bb8::State;
use std::sync::Arc;
use serde::{Deserialize, Serialize };
use websocket_rust::AppState;
use redis::geo::{ RadiusOptions, RadiusOrder, Unit, Coord,RadiusSearchResult };
use redis::AsyncCommands;
use websocket_rust::ChatMessage;
use websocket_rust::User;
use websocket_rust::RMUser;
use websocket_rust::Location;
use websocket_rust::MessageContent;


#[derive(Deserialize, Serialize)]
pub struct UserData {
    name: String,
    location: Location,
    connecting_user: String
}

#[derive(Serialize)]
pub struct PositionResponse {
    message: String,
}


fn send_userData_to_others(userData: &UserData, State(state): State<Arc<AppState>>) {
    // Create user
    let user_name = userData.name.clone();
    let user_data = User { r#type: "user".to_string()
                                , user_id: user_name.clone()    
                                ,location: Location { lat: userData.location.lat, lng: userData.location.lng } };    
    // ChatMessgae
    let chat_message = ChatMessage { user_id: user_name.clone(), 
                                                    to_id: "".to_string(),
                                                    message:  MessageContent::User(user_data)};
    // send user data to other users                                            
    let _ = state.tx.send(chat_message);

} 

pub async fn position_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<UserData>,
) -> impl IntoResponse {
    println!("{}: {}, {}", payload.name, payload.location.lat, payload.location.lng);
    let key_name = payload.name.clone();
    let json_string = serde_json::to_string(&payload).unwrap();
    
    let mut con = state.get_redis_conn().await.unwrap();

    let _: () = con.geo_add(
        "user_locations",
        (Coord::lon_lat(payload.location.lng, payload.location.lat), &payload.name)
    ).await.unwrap();

    let result: Result<(), redis::RedisError> = con.set(&key_name, &json_string).await;
    match result {
        Ok(_) => {
            println!("Data stored successfully {}", key_name);
            send_userData_to_others(&payload, State(state.clone()));

            return Json(PositionResponse { message: "OK".to_string() }).into_response();
        }
        Err(e) => {
            // Log the error
            eprintln!("Redis error: {:?}", e);    
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

#[derive(Deserialize, Serialize)]
pub struct LocationQuery {
    location: Location,
    radius: f64,
}

pub async fn get_users_in_bounds(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<LocationQuery>,
) ->  impl IntoResponse  {
    let mut con = state.get_redis_conn().await.unwrap();

    let opts = RadiusOptions::default().with_coord().order(RadiusOrder::Asc).limit(200);
    // get name ,lat, lng
    let user_infos: Vec<RadiusSearchResult> = con.geo_radius(
        "user_locations",
        payload.location.lng,
        payload.location.lat,
        payload.radius,
        Unit::Meters,
        opts
    ).await.unwrap();



    // ユーザーデータを取得
    let mut users = Vec::new();

    for user_info in user_infos {
        let value: String = match con.get(&user_info.name).await {
            Ok(v) => v,
            Err(e) => {
                eprintln!("Redis get error for key {}: {:?}", &user_info.name, e);
                continue; 
            }
        };

        // JSON文字列をUserDataにデシリアライズ
        match serde_json::from_str::<UserData>(&value) {
            Ok(mut user_data) => {
                if let Some(coord) = user_info.coord {
                    user_data.location.lat = coord.latitude;
                    user_data.location.lng = coord.longitude;
                } else {
                    eprintln!("Missing coordinates for user {}", user_info.name);
                    continue;
                }

                users.push(user_data)
            },
            Err(e) => {
                eprintln!("JSON parse error for key {}: {:?}", &user_info.name, e);
                continue; 
            }
        }
    }

    println!("return {} users in bounds", users.len());
    let res = Json(UserListResponse { users }).into_response();
    return res;
}


pub async fn delete_user (
    State(state): State<Arc<AppState>>,
    name: String) -> Result<(), String>  {
    let mut con = state.get_redis_conn().await.unwrap();
    
    println!("delete user {}", name);

    // 削除するユーザーの情報を取得
    let user_json_data: String = match con.get(&name).await {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Redis get error for key {}: {:?}", name, e);
            return Err(format!("Failed to get user data: {}", e));
        }
    };

    // JSON文字列をUserDataにデシリアライズ
    let user_data: UserData = match serde_json::from_str(&user_json_data) {
        Ok(user_data) => user_data,
        Err(e) => {
            eprintln!("JSON parse error for key {}: {:?}", name, e);
            return Err(format!("Failed to parse user data: {}", e));
        }
    };

    // ZREMコマンドの実行
    match con.zrem::<_, _, i32>("user_locations", &name).await {
        Ok(_) => println!("Successfully removed user location for: {}", name),
        Err(e) => {
            eprintln!("Failed to remove user location for {}: {}", name, e);
        }
    }
    
    // DELコマンドの実行
    match con.del::<_, i32>(&name).await {
        Ok(_) => println!("Successfully deleted user data for: {}", name),
        Err(e) => {
            eprintln!("Failed to delete user data for {}: {}", name, e);
            return Err(format!("Failed to delete user data: {}", e));
        }
    }

    // Send Remove User Data to other users
    let rm_user = RMUser { r#type: "rmuser".to_string()
                                , user_id: name.clone()
                                ,location: Location { lat: user_data.location.lat, lng: user_data.location.lng } };    
    // ChatMessgae
    let chat_message = ChatMessage { user_id: name.clone(), 
                                                    to_id: "".to_string(),
                                                    message:  MessageContent::RMUser(rm_user)};
    // send user data to other users                                            
    let _ = state.tx.send(chat_message);

    println!("User {} successfully deleted", name);
    Ok(())
}
