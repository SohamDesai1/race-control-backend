use std::{collections::HashMap, sync::Arc};

use crate::utils::state::AppState;
use axum::{
    extract::{Path, Query, State},
    response::IntoResponse,
    Json,
};
use http::StatusCode;
use serde_json::{from_str, json, Value};
use tracing::warn;

pub async fn driver_standings(
    State(state): State<Arc<AppState>>,
    Path(season): Path<String>,
    Query(params): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let limit = params
        .get("limit")
        .and_then(|l| l.parse::<u32>().ok())
        .unwrap_or(30);
    let res = state
        .http_client
        .get(format!(
            "https://api.jolpi.ca/ergast/f1/{season}/driverstandings/?format=json&limit={limit}"
        ))
        .send()
        .await;
    match res {
        Ok(res) => {
            let body = res.text().await.unwrap();
            let res: Value = from_str(&body).unwrap();
            let res_body = &res["MRData"]["StandingsTable"]["StandingsLists"];
            if res_body.as_array().unwrap().is_empty() {
                return (StatusCode::OK, Json(json!([]))).into_response();
            }
            (StatusCode::OK, Json(&res_body[0]["DriverStandings"])).into_response()
        }
        Err(e) => {
            warn!("{:?}", e);
            (
                StatusCode::BAD_REQUEST,
                Json(json!({"error":"No data found" })),
            )
                .into_response()
        }
    }
}

pub async fn constructor_standings(
    State(state): State<Arc<AppState>>,
    Path(season): Path<String>,
    Query(params): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let limit = params
        .get("limit")
        .and_then(|l| l.parse::<u32>().ok())
        .unwrap_or(30);
    let res = state
        .http_client
        .get(format!(
            "https://api.jolpi.ca/ergast/f1/{season}/constructorstandings/?format=json&limit={limit}"
        ))
        .send()
        .await;
    match res {
        Ok(res) => {
            let body = res.text().await.unwrap();
            let res: Value = from_str(&body).unwrap();
            let res_body = &res["MRData"]["StandingsTable"]["StandingsLists"];
            if res_body.as_array().unwrap().is_empty() {
                return (StatusCode::OK, Json(json!([]))).into_response();
            }
            (StatusCode::OK, Json(&res_body[0]["ConstructorStandings"])).into_response()
        }
        Err(e) => {
            warn!("{:?}", e);
            (
                StatusCode::BAD_REQUEST,
                Json(json!({"error":"No data found" })),
            )
                .into_response()
        }
    }
}
