use crate::{models::user::CreateUser, utils::state::AppState};
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
    Json(payload): Json<CreateUser>,
) -> impl IntoResponse {
    let body = json!({"name": payload.name, "username": payload.username });
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
            (StatusCode::OK, Json(json!({"data": json_body}))).into_response()
        }
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to fetch users".to_string(),
        )
            .into_response(),
    }
}
