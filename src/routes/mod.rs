pub mod users;
use std::error::Error;

use axum::{response::IntoResponse, routing::get, Json, Router};
use http::StatusCode;
use postgrest::Postgrest;
use serde_json::json;
pub use users::user_routes;
pub mod auth;
pub use auth::auth_routes;

use crate::utils::{config::Config, state::AppState};

pub async fn make_app() -> Result<Router, Box<dyn Error>> {
    let config = Config::init();

    let supabase = Postgrest::new(&format!("{}/rest/v1", config.supabase_project_url))
        .insert_header("apikey", &config.supabase_annon_key)
        .insert_header(
            "Authorization",
            &format!("Bearer {}", config.supabase_jwt_token),
        );
    let state = AppState { supabase, config };
    let app = Router::new()
        .route("/", get(health_check))
        .nest("/auth", auth_routes())
        .nest("/users", user_routes(state.clone()))
        .with_state(state);
    Ok(app)
}

async fn health_check() -> impl IntoResponse {
    return (StatusCode::OK, Json(json!({"message": "Hello World"}))).into_response();
}
