use std::sync::Arc;

use axum::{extract::State, middleware::from_fn, routing::get, Router};

use crate::{handlers::{middleware::auth_middleware, race::get_race_data}, utils::state::AppState};

pub fn race_routes(state: AppState) -> Router<AppState> {
    let race_router = Router::new()
        .route("/get_races", get(get_race_data))
        .with_state(state.clone());
    race_router.layer(from_fn(move |req, next| {
        auth_middleware(State(Arc::new(state.clone())), req, next)
    }))
}
