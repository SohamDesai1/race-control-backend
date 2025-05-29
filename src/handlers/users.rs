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
use serde_json::{from_str, json, Value};

pub async fn get_users(State(state): State<AppState>) -> impl IntoResponse {
    let response = state.supabase.from("Users").select("*").execute().await;
    match response {
        Ok(resp) => {
            let body = resp.text().await.unwrap();
            let json_body: Value = from_str(&body).unwrap();
            (StatusCode::OK, Json(json!({"data": json_body}))).into_response()
        }
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to fetch users".to_string(),
        )
            .into_response(),
    }
}

pub async fn get_user_by_id(
    Path(id): Path<i32>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let response = state
        .supabase
        .from("Users")
        .eq("id", id.to_string())
        .select("*")
        .execute()
        .await;
    match response {
        Ok(resp) => {
            let body = resp.text().await.unwrap();
            let json_body: Value = from_str(&body).unwrap();
            (StatusCode::OK, Json(json!({"data": json_body}))).into_response()
        }
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to fetch users".to_string(),
        )
            .into_response(),
    }
}

pub async fn create_user(
    State(state): State<AppState>,
    Json(payload): Json<User>,
) -> impl IntoResponse {
    let hashed_password = hash_password(payload.password.unwrap().as_str());
    match hashed_password {
        Ok(hashed) => {
            let body = json!({"name": payload.name, "username": payload.username, "email": payload.email, "dob": payload.dob, "password": hashed, "auth_provider":payload.auth_provider});
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

                    let is_profile_complete = json_body
                        .get("is_profile_complete")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);

                    if !is_profile_complete {
                        return (
                            StatusCode::OK,
                            Json(json!({
                                "needs_profile_completion": true,
                                "data": json_body
                            })),
                        )
                            .into_response();
                    }

                    (
                        StatusCode::OK,
                        Json(json!({
                            "needs_profile_completion": false,
                            "data": json_body
                        })),
                    )
                        .into_response()
                }
                Err(_) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to fetch users".to_string(),
                )
                    .into_response(),
            }
        }
        Err(e) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e}))).into_response();
        }
    }
}
