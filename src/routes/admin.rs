use crate::handlers::admin::*;
use crate::handlers::middleware::auth_middleware;
use crate::utils::state::AppState;
use axum::{
    extract::State,
    middleware::from_fn,
    routing::{get, post},
    Router,
};
use std::sync::Arc;

pub fn admin_routes(state: Arc<AppState>) -> Router<Arc<AppState>> {
    let admin_router = Router::new()
        .route("/fantasy/calculate/{gp_id}", post(calculate_gp_scores))
        .route("/fantasy/status", get(get_scoring_status))
        .route("/fantasy/lock/{gp_id}", post(lock_teams))
        .with_state(state.clone());

    admin_router.layer(from_fn(move |req, next| {
        auth_middleware(State(state.clone()), req, next)
    }))
}
