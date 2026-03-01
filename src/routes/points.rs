use std::sync::Arc;

use axum::{middleware::from_fn, routing::get, Router};

use crate::{
    handlers::{
        middleware::auth_middleware,
        standings::{get_constructor_championship, get_driver_championship},
    },
    utils::state::AppState,
};

pub fn points_routes(state: Arc<AppState>) -> Router<Arc<AppState>> {
    let points_router = Router::new()
        .route(
            "/drivers/{season}/{driver_number}",
            get(get_driver_championship),
        )
        .route(
            "/constructors/{season}/{constructor}",
            get(get_constructor_championship),
        )
        .with_state(state.clone());

    points_router.layer(from_fn(move |req, next| {
        auth_middleware(axum::extract::State(state.clone()), req, next)
    }))
}
