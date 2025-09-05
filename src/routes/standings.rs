use crate::{
    handlers::{
        middleware::auth_middleware,
        standings::{constructor_standings, driver_standings},
    },
    utils::state::AppState,
};
use axum::{extract::State, middleware::from_fn, routing::get, Router};
use std::sync::Arc;

pub fn standings_routes(state: AppState) -> Router<AppState> {
    let standings_router = Router::new()
        .route("/driver_standings/{season}", get(driver_standings))
        .route("/constructor_standings/{season}", get(constructor_standings))
        .with_state(state.clone());

    standings_router.layer(from_fn(move |req, next| {
        auth_middleware(State(Arc::new(state.clone())), req, next)
    }))
}
