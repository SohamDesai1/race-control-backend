use std::sync::Arc;

use crate::{
    models::{jwt::RefreshClaims, user::User},
    utils::{
        hash_password::hash_password,
        jwt_encode::{jwt_encode, refresh_token_encode},
        state::AppState,
    },
};
use argon2::{Argon2, PasswordHash, PasswordVerifier};
use axum::{extract::State, response::IntoResponse, Json};
use http::StatusCode;
use jsonwebtoken::{DecodingKey, Validation};
use serde_json::{Value, json};

pub async fn register(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<Value>,
) -> impl IntoResponse {
    if payload["password"].as_str().is_none() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Password is required for email registration"})),
        )
            .into_response();
    }

    let hashed = match hash_password(payload["password"].as_str().unwrap()) {
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

    let response = sqlx::query_as::<_, User>(
        r#"
        INSERT INTO "Users" (name, username, email, dob, hashed_password, auth_provider)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING *
        "#,
    )
    .bind(&payload["name"].as_str().unwrap())   
    .bind(&payload["username"].as_str().unwrap())
    .bind(&payload["email"].as_str().unwrap())
    .bind(&payload["dob"].as_str().unwrap())
    .bind(&hashed)
    .bind("email")
    .fetch_one(&state.db_pool)
    .await;

    match response {
        Ok(user) => {
            // Create a sanitized version without the password hash
            let user_response = json!({
                "name": user.name,
                "username": user.username,
                "email": user.email,
                "dob": user.dob,
                "auth_provider": "email"
            });

            (
                StatusCode::CREATED,
                Json(json!({
                    "message": "User registered",
                    "data": {
                        "user": user_response
                    }
                })),
            )
                .into_response()
        }
        Err(e) => {
            tracing::error!("Database error during registration: {:?}", e);

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
                Json(json!({"error": "Failed to insert user into database"})),
            )
                .into_response()
        }
    }
}

pub async fn login(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<Value>,
) -> impl IntoResponse {
    let email = payload["email"].as_str().unwrap_or_default();
    let password = payload["password"].as_str().unwrap_or_default();

    let response = sqlx::query_as::<_, User>(
        r#"
    SELECT *
    FROM public."Users"
    WHERE LOWER(email) = LOWER($1)
    "#,
    )
    .bind(email.trim())
    .fetch_optional(&state.db_pool)
    .await;
    match response {
        Ok(Some(user)) => {
            let stored_hash = &user.hashed_password.clone().unwrap_or_default();
            let parsed_hash = match PasswordHash::new(stored_hash) {
                Ok(hash) => hash,
                Err(_) => {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!({"error": "Invalid password hash"})),
                    )
                        .into_response();
                }
            };

            let argon2 = Argon2::default();

            if argon2
                .verify_password(password.as_bytes(), &parsed_hash)
                .is_ok()
            {
                let token = jwt_encode(payload["email"].to_string(), state.config.jwt_secret.as_ref());
                let refresh_token =
                    refresh_token_encode(payload["email"].to_string(), state.config.jwt_secret.as_ref());

                (
                    StatusCode::OK,
                    Json(json!({
                        "message": "Login successful",
                        "data": {
                            "user": user,
                            "access_token": token,
                            "refresh_token": refresh_token
                        }
                    })),
                )
                    .into_response()
            } else {
                (
                    StatusCode::UNAUTHORIZED,
                    Json(json!({"error": "Invalid credentials"})),
                )
                    .into_response()
            }
        }
        Ok(None) => (
            StatusCode::UNAUTHORIZED,
            Json(json!({"error": "Invalid credentials"})),
        )
            .into_response(),
        Err(e) => {
            tracing::error!("Database error during login: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "User lookup failed"})),
            )
                .into_response()
        }
    }
}

pub async fn refresh_token_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<Value>,
) -> impl IntoResponse {
    let token_data: Result<jsonwebtoken::TokenData<RefreshClaims>, jsonwebtoken::errors::Error> =
        jsonwebtoken::decode::<RefreshClaims>(
            &payload["refresh_token"].as_str().unwrap_or_default(),
            &DecodingKey::from_secret(state.config.jwt_secret.as_bytes()),
            &Validation::default(),
        );

    let claims = match token_data {
        Ok(data) => data.claims,
        Err(_) => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "Invalid refresh token"})),
            )
                .into_response();
        }
    };

    let new_access_token = jwt_encode(claims.sub.clone(), state.config.jwt_secret.as_ref());
    let new_refresh_token = refresh_token_encode(claims.sub, state.config.jwt_secret.as_ref());

    (
        StatusCode::OK,
        Json(json!({
            "message": "Token refreshed",
            "data": {
                "access_token": new_access_token,
                "refresh_token": new_refresh_token
            }
        })),
    )
        .into_response()
}
