pub mod race;
pub mod session;
pub mod standings;
pub mod users;
use axum::{middleware::from_fn, response::IntoResponse, routing::get, Json, Router};
use dashmap::DashMap;
use http::StatusCode;
use postgrest::Postgrest;
use serde_json::json;
use std::{error::Error, sync::Arc};
use supabase_auth::models::AuthClient;
use tower_http::trace::TraceLayer;
use tracing::{info, Level};
use tracing_subscriber::{filter, layer::SubscriberExt, util::SubscriberInitExt, Registry};
pub use users::user_routes;
pub mod auth;
pub use auth::auth_routes;

use crate::{
    handlers::{middleware::auth_middleware, weather::get_weather},
    models::{cache::CacheEntry, telemetry::{DriverLapGraph, SpeedDistance}},
    routes::{race::race_routes, session::session_routes, standings::standings_routes},
    utils::{config::Config, state::AppState},
};

pub async fn make_app() -> Result<Router, Box<dyn Error>> {
    info!("Initializing application...");
    let config = Config::init();

    info!("Configuration loaded successfully");

    let supabase = Postgrest::new(&format!("{}/rest/v1", &config.supabase_project_url))
        .insert_header("apikey", &config.supabase_service_role_key);
    let supabase_auth = AuthClient::new(
        &config.supabase_project_url,
        &config.supabase_anon_key,
        &config.supabase_service_role_key,
    );

    let http_client = reqwest::Client::new();
    info!("External clients initialized successfully");
    
    let fetch_driver_telemetry_cache: DashMap<String, CacheEntry<Vec<SpeedDistance>>> =
        DashMap::new();
    let get_drivers_position_telemetry_cache: DashMap<String, CacheEntry<Vec<DriverLapGraph>>> =
        DashMap::new();

    let state = Arc::new(AppState {
        supabase,
        supabase_auth,
        config,
        http_client,
        fetch_driver_telemetry_cache,
        get_drivers_position_telemetry_cache,
    });

    let log_level = std::env::var("LOG_LEVEL")
        .unwrap_or_else(|_| "info".to_string())
        .to_lowercase();

    let level = match log_level.as_str() {
        "error" => Level::ERROR,
        "warn" => Level::WARN,
        "info" => Level::INFO,
        "debug" => Level::DEBUG,
        "trace" => Level::TRACE,
        _ => Level::INFO,
    };

    let filter = filter::Targets::new()
        .with_target("tower_http::trace::on_response", Level::TRACE)
        .with_target("tower_http::trace::on_request", Level::TRACE)
        .with_target("tower_http::trace::make_span", Level::DEBUG)
        .with_target("axum::rejection", Level::TRACE)
        .with_target(env!("CARGO_PKG_NAME"), level)
        .with_default(Level::INFO);

    let tracing_layer = tracing_subscriber::fmt::layer();

    Registry::default().with(tracing_layer).with(filter).init();

    let value = state.clone();
    let app = Router::new()
        .route("/", get(health_check))
        .nest("/auth", auth_routes())
        .nest("/users", user_routes(state.clone()))
        .nest("/race", race_routes(state.clone()))
        .nest("/session", session_routes(state.clone()))
        .nest("/standings", standings_routes(state.clone()))
        .route(
            "/get_weather",
            get(get_weather).route_layer(from_fn(move |req, next| {
                auth_middleware(axum::extract::State(value.clone()), req, next)
            })),
        )
        .layer(TraceLayer::new_for_http())
        .with_state(state);
    info!("Application initialized successfully");

    Ok(app)
}

async fn health_check() -> impl IntoResponse {
    return (StatusCode::OK, Json(json!({"message": "Hello World"}))).into_response();
}
