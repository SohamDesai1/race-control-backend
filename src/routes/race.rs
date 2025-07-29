use std::sync::Arc;

use axum::{extract::State, middleware::from_fn, routing::get, Router};

use crate::{
    handlers::{
        middleware::auth_middleware,
        race::{get_race_data_db, get_race_results},
    },
    utils::state::AppState,
};

pub fn race_routes(state: AppState) -> Router<AppState> {
    let race_router = Router::new()
        .route("/get_race_results", get(get_race_results)) 
        .route("/get_race_results/{round}", get(get_race_results))
        .route("/get_race_data", get(get_race_data_db))
        .with_state(state.clone());
    race_router.layer(from_fn(move |req, next| {
        auth_middleware(State(Arc::new(state.clone())), req, next)
    }))
}
