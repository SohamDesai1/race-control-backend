use std::sync::Arc;
use axum::{extract::State, middleware::from_fn, routing::get, Router};
use crate::handlers::middleware::auth_middleware;
use crate::handlers::users::{create_user, get_user_by_id, get_users};
use crate::utils::state::AppState;

pub fn user_routes(state: AppState) -> Router<AppState> {
    let user_router = Router::new()
        .route("/", get(get_users).post(create_user))
        .route("/{id}", get(get_user_by_id))
        .with_state(state.clone());

    user_router.layer(from_fn(move |req, next| {
        auth_middleware(State(Arc::new(state.clone())), req, next)
    }))
}
