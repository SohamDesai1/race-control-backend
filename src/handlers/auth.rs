use std::sync::Arc;

use crate::{
    models::user::User,
    utils::{hash_password::hash_password, state::AppState},
};
use axum::{extract::State, response::IntoResponse, Json};
use http::StatusCode;
use serde_json::{from_str, json, Value};

pub async fn register(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<User>,
) -> impl IntoResponse {
    if payload.password.is_none() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Password is required for email registration"})),
        )
            .into_response();
    }

    match hash_password(payload.password.as_ref().unwrap()) {
        Ok(hashed) => {
            let body = json!({
                "name": payload.name,
                "username": payload.username,
                "email": payload.email,
                "dob": payload.dob,
                "hashed_password": hashed,
                "auth_provider": "email"
            });
            let session = state
                .supabase_auth
                .sign_up_with_email_and_password(
                    &payload.email.clone(),
                    &payload.password.unwrap(),
                    None,
                )
                .await;
            match session {
                Ok(user) => {
                    let response = state
                        .supabase
                        .from("Users")
                        .insert(body.to_string())
                        .execute()
                        .await;

                    match response {
                        Ok(resp) => {
                            let body = resp.text().await.unwrap();
                            let json_body: Value = from_str(&body).unwrap();

                            match user {
                                supabase_auth::models::EmailSignUpResult::SessionResult(
                                    session,
                                ) => (
                                    StatusCode::CREATED,
                                    Json(json!({
                                        "message": "User registered",
                                        "data": {
                                            "access_token": session.access_token,
                                            "user": json_body
                                        }
                                    })),
                                ),
                                supabase_auth::models::EmailSignUpResult::ConfirmationResult(
                                    _,
                                ) => (
                                    StatusCode::CREATED,
                                    Json(json!({"message": "User registered", "data": json_body})),
                                ),
                            }
                            .into_response()
                        }
                        Err(_) => (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Json(json!({"error": "Failed to insert user into database"})),
                        )
                            .into_response(),
                    }
                }
                Err(e) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({"error": e.to_string()})),
                )
                    .into_response(),
            }
        }
        Err(e) => (StatusCode::CONFLICT, Json(json!({"error": e}))).into_response(),
    }
}

pub async fn login(State(state): State<Arc<AppState>>, Json(payload): Json<User>) -> impl IntoResponse {
    let email = payload.email.clone();
    let password = payload.password.unwrap_or_default();
    let session = state
        .supabase_auth
        .login_with_email(&email, &password)
        .await;
    match session {
        Ok(user) => {
            let response = state
                .supabase
                .from("Users")
                .eq("email", email)
                .select("*")
                .execute()
                .await;

            match response {
                Ok(resp) => {
                    let body = resp.text().await.unwrap();
                    let json_body: Value = from_str(&body).unwrap();
                    // let stored_hash = json_body[0]["hashed_password"].as_str().unwrap_or("");
                    // let parsed_hash = PasswordHash::new(stored_hash).unwrap();
                    // let argon2 = Argon2::default();

                    // if argon2
                    //     .verify_password(password.as_bytes(), &parsed_hash)
                    //     .is_ok()
                    // {
                    (
                    StatusCode::OK,
                    Json(json!({"message": "Login successful", "data": {"user":json_body,"acces_token": user.access_token,"refresh_token": user.refresh_token}})),
                )
                    .into_response()
                    // } else {
                    //     (
                    //         StatusCode::UNAUTHORIZED,
                    //         Json(json!({"error": "Invalid credentials"})),
                    //     )
                    //         .into_response()
                    // }
                }
                Err(_) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({"error": "User lookup failed"})),
                )
                    .into_response(),
            }
        }
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "User lookup failed"})),
        )
            .into_response(),
    }
}

pub async fn google_auth(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<User>,
) -> impl IntoResponse {
    let email = payload.email.clone();

    let response = state
        .supabase
        .from("Users")
        .eq("email", email.clone())
        .select("*")
        .execute()
        .await;

    match response {
        Ok(resp) => {
            let body = resp.text().await.unwrap();
            let existing_user: Value = from_str(&body).unwrap();

            if existing_user.as_array().unwrap().is_empty() {
                // Create user if doesn't exist
                let body = json!({
                    "name": payload.name,
                    "email": email,
                    "auth_provider": "google",
                });

                let insert_response = state
                    .supabase
                    .from("Users")
                    .insert(body.to_string())
                    .execute()
                    .await;

                match insert_response {
                    Ok(ins_resp) => {
                        let ins_body = ins_resp.text().await.unwrap();
                        let ins_json: Value = from_str(&ins_body).unwrap();
                        // let token =
                        //     jwt_encode(payload.email, None, state.config.jwt_secret.as_ref());
                        (
                            StatusCode::CREATED,
                            Json(json!({"message": "User created", "data": {"user":ins_json, "token": "token"}})),
                        )
                            .into_response()
                    }
                    Err(_) => (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!({"error": "Failed to create user"})),
                    )
                        .into_response(),
                }
            } else {
                (
                    StatusCode::OK,
                    Json(json!({"message": "User exists", "data": existing_user})),
                )
                    .into_response()
            }
        }
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "User lookup failed"})),
        )
            .into_response(),
    }
}

pub async fn refresh_token_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<User>,
) -> impl IntoResponse {
    let refresh_token = payload.refresh_token.unwrap_or_default();

    let session = state
        .supabase_auth
        .exchange_token_for_session( &refresh_token)
        .await;

    match session {
        Ok(user) => (
            StatusCode::OK,
            Json(json!({"message": "Token refreshed", "data": {"access_token": user.access_token, "refresh_token": user.refresh_token}})),
        )
            .into_response(),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Failed to refresh token"})),
        )
            .into_response(),
    }
}