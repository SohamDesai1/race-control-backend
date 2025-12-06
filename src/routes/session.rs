use crate::{
    handlers::{
        middleware::auth_middleware,
        session::{get_session_data, fetch_driver_telemetry, get_sessions, get_drivers_position_telemetry},
    },
    utils::state::AppState,
};
use axum::{extract::State, middleware::from_fn, routing::get, Router};
use std::sync::Arc;

pub fn session_routes(state: Arc<AppState>) -> Router<Arc<AppState>> {
    let session_router = Router::new()
        .route("/get_sessions/{race_id}", get(get_sessions))
        .route("/get_session_data/{session_key}", get(get_session_data))
        .route("/fetch_driver_telemetry", get(fetch_driver_telemetry))
        .route("/get_drivers_position_telemetry/{session_key}", get(get_drivers_position_telemetry))
        .with_state(state.clone());

    session_router.layer(from_fn(move |req, next| {
        auth_middleware(State(state.clone()), req, next)
    }))
}
