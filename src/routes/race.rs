use crate::{
    handlers::{
        middleware::auth_middleware,
        race::{get_race_data_db, get_race_results, get_upcoming_race_data},
    },
    utils::state::AppState,
};
use axum::{extract::State, middleware::from_fn, routing::get, Router};
use std::sync::Arc;

pub fn race_routes(state: AppState) -> Router<AppState> {
    let race_router = Router::new()
        .route("/get_race_results", get(get_race_results))
        .route("/get_race_results/{round}", get(get_race_results))
        .route("/get_race_data", get(get_race_data_db))
        .route("/get_upcoming_race_data", get(get_upcoming_race_data))
        .with_state(state.clone());
    race_router.layer(from_fn(move |req, next| {
        auth_middleware(State(Arc::new(state.clone())), req, next)
    }))
}
