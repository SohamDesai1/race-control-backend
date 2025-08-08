pub mod race;
pub mod session;
pub mod users;
use axum::{response::IntoResponse, routing::get, Json, Router};
use http::StatusCode;
use postgrest::Postgrest;
use serde_json::json;
use std::error::Error;
use supabase_auth::models::AuthClient;
pub use users::user_routes;
pub mod auth;
pub use auth::auth_routes;

use crate::{
    routes::{race::race_routes, session::session_routes},
    utils::{config::Config, state::AppState},
};

pub async fn make_app() -> Result<Router, Box<dyn Error>> {
    let config = Config::init();

    let supabase = Postgrest::new(&format!("{}/rest/v1", &config.supabase_project_url))
        .insert_header("apikey", &config.supabase_annon_key)
        .insert_header(
            "Authorization",
            &format!("Bearer {}", &config.supabase_jwt_token),
        );
    let supabase_auth = AuthClient::new(
        &config.supabase_project_url,
        &config.supabase_annon_key,
        &config.supabase_jwt_token,
    );
    let http_client = reqwest::Client::new();
    let state = AppState {
        supabase,
        supabase_auth,
        config,
        http_client,
    };
    let app = Router::new()
        .route("/", get(health_check))
        .nest("/auth", auth_routes())
        .nest("/users", user_routes(state.clone()))
        .nest("/race", race_routes(state.clone()))
        .nest("/session", session_routes(state.clone()))
        .with_state(state);
    Ok(app)
}

async fn health_check() -> impl IntoResponse {
    return (StatusCode::OK, Json(json!({"message": "Hello World"}))).into_response();
}
