use std::{collections::HashMap, sync::Arc};

use crate::utils::state::AppState;
use axum::{
    extract::{Path, Query, State},
    response::IntoResponse,
    Json,
};
use http::StatusCode;
use serde_json::{from_str, json, Value};
use sqlx::FromRow;
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
            let body = res.text().await;
            if body.is_err() {
                tracing::error!("Route failed: driver_standings");
            }
            let body = body.unwrap();
            
            let res: Result<Value, _> = from_str(&body);
            if res.is_err() {
                tracing::error!("Route failed: driver_standings");
            }
            let res = res.unwrap();
            
            let res_body = &res["MRData"]["StandingsTable"]["StandingsLists"];
            let arr = res_body.as_array();
            if arr.is_none() {
                tracing::error!("Route failed: driver_standings");
            }
            if arr.unwrap().is_empty() {
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
            let body = res.text().await;
            if body.is_err() {
                tracing::error!("Route failed: constructor_standings");
            }
            let body = body.unwrap();
            
            let res: Result<Value, _> = from_str(&body);
            if res.is_err() {
                tracing::error!("Route failed: constructor_standings");
            }
            let res = res.unwrap();
            
            let res_body = &res["MRData"]["StandingsTable"]["StandingsLists"];
            let arr = res_body.as_array();
            if arr.is_none() {
                tracing::error!("Route failed: constructor_standings");
            }
            if arr.unwrap().is_empty() {
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

#[derive(FromRow)]
struct DriverChampionshipWithRace {
    driver_number: String,
    season: String,
    round: String,
    points_current: f64,
    position: Option<i32>,
    race_name: String,
}

pub async fn get_driver_championship(
    State(state): State<Arc<AppState>>,
    Path((season, driver_number)): Path<(String, String)>,
) -> impl IntoResponse {
    let result = sqlx::query_as::<_, DriverChampionshipWithRace>(
        r#"
        SELECT 
            dch.driver_number,
            dch.season,
            dch.round,
            dch.points_current,
            dch.position,
            r."raceName" as race_name
        FROM "DriverChampionshipHistory" dch
        INNER JOIN "Races" r ON dch.race_id = r.id
        WHERE dch.season = $1 AND dch.driver_number = $2
        ORDER BY dch.round ASC, dch.position ASC
        "#,
    )
    .bind(&season)
    .bind(&driver_number)
    .fetch_all(&state.db_pool)
    .await;

    match result {
        Ok(standings) => {
            let response = json!({
                "season": season,
                "standings": standings.iter().map(|s| json!({
                    "round": s.round,
                    "race_name": s.race_name,
                    "driver_number": s.driver_number,
                    "points_current": s.points_current,
                    "position": s.position
                })).collect::<Vec<_>>()
            });
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(e) => {
            tracing::error!("Failed to fetch driver championship: {:?}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Failed to fetch championship data"}))).into_response()
        }
    }
}

#[derive(FromRow)]
struct ConstructorChampionshipWithRace {
    constructor_id: String,
    constructor_name: String,
    season: String,
    round: String,
    points_current: f64,
    position: Option<i32>,
    race_name: String,
}

pub async fn get_constructor_championship(
    State(state): State<Arc<AppState>>,
    Path((season,constructor)): Path<(String, String)>,
) -> impl IntoResponse {
    let result = sqlx::query_as::<_, ConstructorChampionshipWithRace>(
        r#"
        SELECT 
            cch.constructor_id,
            cch.constructor_name,
            cch.season,
            cch.round,
            cch.points_current,
            cch.position,
            r."raceName" as race_name
        FROM "ConstructorChampionshipHistory" cch
        INNER JOIN "Races" r ON cch.race_id = r.id
        WHERE cch.season = $1 AND cch.constructor_id = $2
        ORDER BY cch.round ASC, cch.position ASC
        "#,
    )
    .bind(&season)
    .bind(&constructor)
    .fetch_all(&state.db_pool)
    .await;

    match result {
        Ok(standings) => {
            let response = json!({
                "season": season,
                "standings": standings.iter().map(|s| json!({
                    "round": s.round,
                    "race_name": s.race_name,
                    "constructor_id": s.constructor_id,
                    "constructor_name": s.constructor_name,
                    "points_current": s.points_current,
                    "position": s.position
                })).collect::<Vec<_>>()
            });
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(e) => {
            tracing::error!("Failed to fetch constructor championship: {:?}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Failed to fetch championship data"}))).into_response()
        }
    }
}
