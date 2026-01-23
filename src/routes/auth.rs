use std::sync::Arc;

use axum::{routing::post, Router};

use crate::{
    handlers::auth::{login, refresh_token_handler, register},
    utils::state::AppState,
};

pub fn auth_routes() -> Router<Arc<AppState>> {
    Router::new()
        // .route("/google", post(google_auth))
        .route("/login", post(login))
        .route("/register", post(register))
        .route("/refresh", post(refresh_token_handler))
}
