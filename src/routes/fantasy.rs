use std::sync::Arc;

use axum::{
    extract::State,
    middleware::from_fn,
    routing::{get, post},
    Router,
};

use crate::{
    handlers::{
        fantasy::{
            get_fantasy_catalog, preview_driver_price, preview_fantasy_score, validate_fantasy_team,
        },
        middleware::auth_middleware,
    },
    utils::state::AppState,
};

pub fn fantasy_routes(state: Arc<AppState>) -> Router<Arc<AppState>> {
    let fantasy_router = Router::new()
        .route("/catalog", get(get_fantasy_catalog))
        .route("/team/validate", post(validate_fantasy_team))
        .route("/score/preview", post(preview_fantasy_score))
        .route("/driver/price-preview", post(preview_driver_price))
        .with_state(state.clone());

    fantasy_router.layer(from_fn(move |req, next| {
        auth_middleware(State(state.clone()), req, next)
    }))
}
