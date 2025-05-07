use axum::{extract::State, response::IntoResponse, routing::get, serve, Json, Router};
use dotenv::dotenv;
use http::StatusCode;
use postgrest::Postgrest;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::env;
use std::sync::Arc;
use tokio::net::TcpListener;

#[derive(Clone)]
struct AppState {
    supabase: Arc<Postgrest>,
}

#[derive(Debug, Deserialize, Serialize)]
struct User {
    id: Option<i32>,
    name: String,
}

#[tokio::main]
async fn main() {
    dotenv().ok();
    let url = env::var("SUPABASE_PROJECT_URL").unwrap();
    let api_key = env::var("SUPABASE_ANNON_KEY").unwrap();
    let jwt_token = env::var("SUPABASE_JWT_TOKEN").unwrap();

    let client = Postgrest::new(&format!("{}/rest/v1", url))
        .insert_header("apikey", &api_key)
        .insert_header("Authorization", &format!("Bearer {}", jwt_token));

    let state = AppState {
        supabase: Arc::new(client),
    };

    // Define routes
    let app = Router::new()
        .route("/", get(root))
        .route("/users", get(get_users).post(create_user))
        .with_state(state);

    // Bind to a TCP listener
    let listener = TcpListener::bind("127.0.0.1:3000").await.unwrap();
    println!("Listening on http://127.0.0.1:3000");

    // Serve the app using the listener
    serve(listener, app).await.unwrap();
}

async fn root() -> Json<Value> {
    return Json(json!({"message": "Hello World"}));
}

async fn get_users(State(state): State<AppState>) -> impl IntoResponse {
    let response = state.supabase.from("Users").select("*").execute().await;

    match response {
        Ok(resp) => {
            let body = resp.text().await.unwrap();
            (StatusCode::OK, body).into_response()
        }
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to fetch users".to_string(),
        )
            .into_response(),
    }
}

// Handler: POST /users
async fn create_user(
    State(state): State<AppState>,
    Json(payload): Json<User>,
) -> impl IntoResponse {
    let user_data = json!({ "name": payload.name });

    let response = state
        .supabase
        .from("users")
        .insert(user_data.to_string())
        .execute()
        .await;

    match response {
        Ok(resp) => {
            let body = resp.text().await.unwrap();
            (StatusCode::OK, body).into_response()
        }
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Insert failed".to_string(),
        )
            .into_response(),
    }
}
