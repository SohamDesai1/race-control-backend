mod handlers;
mod models;
mod routes;
mod utils;
use axum::{routing::get, serve, Json, Router};
use routes::user_routes;
use serde_json::{json, Value};
use tokio::net::TcpListener;
use utils::state::AppState;

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    let state = AppState::init().await;

    let app = Router::new()
        .route("/", get(root))
        .nest("/users", user_routes())
        .with_state(state);

    // Bind to a TCP listener
    let listener = TcpListener::bind("127.0.0.1:3000").await;
    println!("Listening on http://127.0.0.1:3000");

    match listener {
        Ok(res) => serve(res, app).await.unwrap(),
        Err(err) => panic!("{}", err),
    }
}

async fn root() -> Json<Value> {
    return Json(json!({"message": "Hello World"}));
}
