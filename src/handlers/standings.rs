use crate::utils::state::AppState;
use axum::{
    extract::{Path, State},
    response::IntoResponse,
    Json,
};
use http::StatusCode;
use serde_json::{from_str, Value};

pub async fn driver_standings(
    State(state): State<AppState>,
    season: Option<Path<String>>,
) -> impl IntoResponse {
    let season = season
        .map(|Path(s)| s)
        .unwrap_or_else(|| "2025".to_string());
    let res = state
        .http_client
        .get(format!(
            "https://api.jolpi.ca/ergast/f1/{season}/driverstandings/?format=json"
        ))
        .send()
        .await
        .unwrap();
    let body = res.text().await.unwrap();
    let res: Value = from_str(&body).unwrap();
    let res_body = &res["MRData"]["StandingsTable"]["StandingsLists"][0]["DriverStandings"];
    (StatusCode::OK, Json(res_body)).into_response()
}

pub async fn constructor_standings(
    State(state): State<AppState>,
    season: Option<Path<String>>,
) -> impl IntoResponse {
    let season = season
        .map(|Path(s)| s)
        .unwrap_or_else(|| "2025".to_string());
    let res = state
        .http_client
        .get(format!(
            "https://api.jolpi.ca/ergast/f1/{season}/constructorstandings/?format=json"
        ))
        .send()
        .await
        .unwrap();
    let body = res.text().await.unwrap();
    let res: Value = from_str(&body).unwrap();
    let res_body = &res["MRData"]["StandingsTable"]["StandingsLists"][0]["ConstructorStandings"];
    (StatusCode::OK, Json(res_body)).into_response()
}
