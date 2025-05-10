use crate::handlers::users::{create_user, get_user_by_id, get_users};
use crate::utils::state::AppState;
use axum::{routing::get, Router};

pub fn user_routes() -> Router<AppState> {
    Router::new()
        .route("/", get(get_users).post(create_user))
        .route("/{id}", get(get_user_by_id))
}
