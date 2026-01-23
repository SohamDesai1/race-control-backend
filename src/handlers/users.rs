use std::sync::Arc;

use crate::{
    models::user::User,
    utils::{hash_password::hash_password, state::AppState},
};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde_json::json;

pub async fn get_users(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let response = sqlx::query_as::<_, User>(r#"SELECT * FROM "Users""#)
        .fetch_all(&state.db_pool)
        .await;

    match response {
        Ok(users) => (StatusCode::OK, Json(json!({"data": users}))).into_response(),
        Err(e) => {
            tracing::error!("Database error fetching users: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to fetch users"})),
            )
                .into_response()
        }
    }
}

pub async fn get_user_by_id(
    Path(id): Path<i32>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let response = sqlx::query_as::<_, User>(r#"SELECT * FROM "Users" WHERE id = $1"#)
        .bind(id)
        .fetch_optional(&state.db_pool)
        .await;

    match response {
        Ok(Some(user)) => (StatusCode::OK, Json(json!({"data": user}))).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(json!({"error": "User not found"})),
        )
            .into_response(),
        Err(e) => {
            tracing::error!("Database error fetching user by id {}: {:?}", id, e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to fetch user"})),
            )
                .into_response()
        }
    }
}

pub async fn create_user(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<User>,
) -> impl IntoResponse {
    // Validate password exists
    let password = match &payload.hashed_password {
        Some(pwd) => pwd,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": "Password is required"})),
            )
                .into_response();
        }
    };

    // Hash the password
    let hashed = match hash_password(password.as_str()) {
        Ok(hashed) => hashed,
        Err(e) => {
            tracing::error!("Password hashing error: {:?}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to hash password"})),
            )
                .into_response();
        }
    };

    // Insert user into database
    let response = sqlx::query_as::<_, User>(
        r#"
        INSERT INTO "Users" (name, username, email, dob, hashed_password, auth_provider)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING *
        "#,
    )
    .bind(&payload.name)
    .bind(&payload.username)
    .bind(&payload.email)
    .bind(&payload.dob)
    .bind(&hashed)
    .bind(&payload.auth_provider)
    .fetch_one(&state.db_pool)
    .await;

    match response {
        Ok(user) => {
            let is_profile_complete = user.is_profile_complete.unwrap_or(false);

            (
                StatusCode::CREATED,
                Json(json!({
                    "needs_profile_completion": !is_profile_complete,
                    "data": user
                })),
            )
                .into_response()
        }
        Err(e) => {
            tracing::error!("Database error creating user: {:?}", e);

            // Check for specific database errors
            if let Some(db_err) = e.as_database_error() {
                // Handle unique constraint violations (duplicate email/username)
                if db_err.code() == Some(std::borrow::Cow::Borrowed("23505")) {
                    return (
                        StatusCode::CONFLICT,
                        Json(json!({"error": "User with this email or username already exists"})),
                    )
                        .into_response();
                }
            }

            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to create user"})),
            )
                .into_response()
        }
    }
}
