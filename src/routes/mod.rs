pub mod race;
pub mod session;
pub mod standings;
pub mod users;
use axum::{middleware::from_fn, response::IntoResponse, routing::get, Json, Router};
use dashmap::DashMap;
use http::StatusCode;
use serde_json::json;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use std::{error::Error, str::FromStr, sync::Arc};
use tower_http::trace::TraceLayer;
use tracing::{info, Level};
use tracing_subscriber::{filter, layer::SubscriberExt, util::SubscriberInitExt, Registry};
pub use users::user_routes;
pub mod auth;
pub use auth::auth_routes;

use crate::{
    handlers::{middleware::auth_middleware, news::get_news, weather::get_weather},
    models::{
        cache::CacheEntry,
        telemetry::{DriverLapGraph, FastestLapSector, PacePoint, SpeedDistance},
    },
    routes::{race::race_routes, session::session_routes, standings::standings_routes},
    utils::{config::Config, state::AppState},
};

pub async fn make_app() -> Result<Router, Box<dyn Error>> {
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

    info!("Initializing application...");
    let config = Config::init();

    info!("Configuration loaded successfully");
    let connect_options = PgConnectOptions::from_str(&config.db_url)?.statement_cache_capacity(0);
    // Create database connection pool
    let db_pool = PgPoolOptions::new()
        .max_connections(5)
        .min_connections(1)
        .acquire_timeout(std::time::Duration::from_secs(10))
        .idle_timeout(Some(std::time::Duration::from_secs(60)))
        .connect_with(connect_options)
        .await?;

    info!("Database connection pool created successfully");
    let http_client = reqwest::Client::new();
    info!("External clients initialized successfully");

    let fetch_driver_telemetry_cache: DashMap<String, CacheEntry<Vec<SpeedDistance>>> =
        DashMap::new();
    let get_drivers_position_telemetry_cache: DashMap<String, CacheEntry<Vec<DriverLapGraph>>> =
        DashMap::new();
    let get_sector_timings_cache: DashMap<String, CacheEntry<Vec<FastestLapSector>>> =
        DashMap::new();
    let get_race_pace_cache: DashMap<String, CacheEntry<Vec<PacePoint>>> = DashMap::new();

    let state = Arc::new(AppState {
        db_pool,
        config,
        http_client,
        fetch_driver_telemetry_cache,
        get_drivers_position_telemetry_cache,
        get_sector_timings_cache,
        get_race_pace_cache,
    });

    let value1 = state.clone();
    let value2 = state.clone();
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
                auth_middleware(axum::extract::State(value1.clone()), req, next)
            })),
        )
        .route(
            "/get_news",
            get(get_news).route_layer(from_fn(move |req, next| {
                auth_middleware(axum::extract::State(value2.clone()), req, next)
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
