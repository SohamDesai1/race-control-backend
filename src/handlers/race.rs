use axum::{response::IntoResponse, Json};
use http::StatusCode;
use serde_json::{from_str, Value};

pub async fn get_race_data() -> impl IntoResponse {
    let client = reqwest::Client::new();
    let res = client
        .get("https://api.jolpi.ca/ergast/f1/2025/last/status/1/results/?format=json")
        .send()
        .await
        .unwrap();
    let body = res.text().await.unwrap();
    let res: Value = from_str(&body).unwrap();
    let res_body = &res["MRData"];
    (StatusCode::OK, Json(res_body)).into_response()
}
