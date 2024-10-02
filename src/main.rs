mod view;
mod handlers; 

use view::render_template;
use axum::{
    response::IntoResponse,
    routing::get,
    routing::post,
    routing::get_service,
    Router
};

use std::{net::SocketAddr,  sync::Arc};
use tokio::sync::broadcast;
use tower_http::services::ServeDir;
use dotenv::dotenv;
use std::env;
use websocket_rust::create_pool;

use crate::handlers::position::getallusers_handler;
use crate::handlers::position::position_handler; 
use crate::handlers::websocket::ws_handler;
use crate::handlers::websocket::AppState;

// アプリケーションの状態
#[tokio::main]
async fn main() {
    dotenv().ok();

    // redis cloudへ接続
    let redis_url = env::var("REDIS_URL").unwrap().to_string();
    let port = env::var("PORT").expect("PORT environment variable not set").parse::<u16>().expect("PORT is not a number");

    let pool = create_pool(&redis_url).await.unwrap();
    let (tx, _rx) = broadcast::channel(100);
    let app_state = Arc::new(AppState { pool ,tx });

    let app = Router::new()
        .route("/ws", get(ws_handler))
        .with_state(Arc::clone(&app_state));
    let app = app.route("/", get(move || async move {hello_handler(port).await}));
    let app = app.route("/position",post(position_handler)).with_state(Arc::clone(&app_state));
    let app = app.route("/users",post(getallusers_handler)).with_state(Arc::clone(&app_state));
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
    let addr= SocketAddr::from(([0, 0, 0, 0], port));
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn hello_handler(port : u16) -> impl IntoResponse {
    return render_template("test".to_string(), port);
}
