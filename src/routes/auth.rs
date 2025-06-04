use axum::{routing::post, Router};

use crate::{
    handlers::auth::{google_auth, login, register},
    utils::state::AppState,
};

pub fn auth_routes() -> Router<AppState> {
    Router::new()
        .route("/google", post(google_auth))
        .route("/login", post(login))
        .route("/register", post(register))
}
