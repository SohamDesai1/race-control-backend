use std::sync::Arc;

use axum::{Router, middleware::from_fn, routing::{get, post}};

use crate::{
    handlers::{
        middleware::auth_middleware,
        standings::{
            get_constructor_championship_points, get_driver_championship_points,
            seed_championship_data_historical,
        },
    },
    utils::state::AppState,
};

pub fn points_routes(state: Arc<AppState>) -> Router<Arc<AppState>> {
    let points_router = Router::new()
        .route(
            "/drivers/{season}/{driver_number}",
            get(get_driver_championship_points),
        )
        .route(
            "/constructors/{season}/{constructor}",
            get(get_constructor_championship_points),
        )
        .route("/seed_points_historical", post(seed_championship_data_historical))
        .with_state(state.clone());

    points_router.layer(from_fn(move |req, next| {
        auth_middleware(axum::extract::State(state.clone()), req, next)
    }))
}
