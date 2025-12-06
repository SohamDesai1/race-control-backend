use std::sync::Arc;

use crate::utils::state::AppState;
use axum::{
    extract::{Query, State},
    response::IntoResponse,
    Json,
};
use http::StatusCode;
use serde_json::{from_str, Value};

use serde::Deserialize;

#[derive(Deserialize)]
pub struct WeatherQuery {
    session_key: Option<String>,
    meeting_key: Option<String>,
}

pub async fn get_weather(
    State(state): State<Arc<AppState>>,
    Query(params): Query<WeatherQuery>,
) -> impl IntoResponse {
    let session_key = params.session_key.unwrap_or_else(|| "latest".to_string());
    let meeting_key = params.meeting_key.unwrap_or_else(|| "latest".to_string());
    
    let res = state
        .http_client
        .get(format!(
            "https://api.openf1.org/v1/weather?meeting_key={meeting_key}&session_key={session_key}"
        ))
        .send()
        .await
        .unwrap();

    let body = res.text().await.unwrap();
    let res: Value = from_str(&body).unwrap();
    let res_body = &res;

    (StatusCode::OK, Json(res_body)).into_response()
}
