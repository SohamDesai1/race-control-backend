use crate::{
    handlers::{
        middleware::auth_middleware,
        session::{ get_session_data, get_sessions},
    },
    utils::state::AppState,
};
use axum::{extract::State, middleware::from_fn, routing::get, Router};
use std::sync::Arc;

pub fn session_routes(state: AppState) -> Router<AppState> {
    let session_router = Router::new()
        .route("/get_sessions/{race_id}", get(get_sessions))
        .route("/get_session_data/{session_key}", get(get_session_data))
        .with_state(state.clone());

    session_router.layer(from_fn(move |req, next| {
        auth_middleware(State(Arc::new(state.clone())), req, next)
    }))
}
