use std::{collections::HashMap, sync::Arc};

use crate::{models::session::SessionWithRace, utils::state::AppState};
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
    #[allow(dead_code)]
    season: String,
    round: String,
    points_current: f64,
    position: Option<i32>,
    race_name: String,
}

pub async fn get_driver_championship_points(
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
        FROM "DriverPointsHistory" dch
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
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to fetch championship data"})),
            )
                .into_response()
        }
    }
}

#[derive(FromRow)]
struct ConstructorChampionshipWithRace {
    constructor_id: String,
    constructor_name: String,
    #[allow(dead_code)]
    season: String,
    round: String,
    points_current: f64,
    position: Option<i32>,
    race_name: String,
}

pub async fn get_constructor_championship_points(
    State(state): State<Arc<AppState>>,
    Path((season, constructor)): Path<(String, String)>,
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
        FROM "ConstructorPointsHistory" cch
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
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to fetch championship data"})),
            )
                .into_response()
        }
    }
}

pub async fn seed_championship_data_historical(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let sessions = match sqlx::query_as::<_, SessionWithRace>(
        r#"
        SELECT 
            s."sessionType" as session_type,
            s.session_key,
            s.meeting_key,
            r.season,
            r.round,
            r.id as race_id
        FROM "Sessions" s
        INNER JOIN "Races" r ON s."raceId" = r.id
        WHERE s."sessionType" = 'Race' AND r."season" = '2025'
        "#,
    )
    .fetch_all(&state.db_pool)
    .await
    {
        Ok(s) => s,
        Err(e) => {
            tracing::error!("Failed to fetch sessions: {:?}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to fetch sessions"})),
            )
                .into_response();
        }
    };

    if sessions.is_empty() {
        tracing::warn!("No race sessions found for championship seeding");
        return (
            StatusCode::OK,
            Json(json!({"message": "No race sessions found"})),
        )
            .into_response();
    }

    for session in sessions {
        let session_key = session.session_key.unwrap_or_default();
        let meeting_key = session.meeting_key.unwrap_or_default();

        /*
        DRIVER CHAMPIONSHIP
        */

        let drivers_url = format!(
            "https://api.openf1.org/v1/championship_drivers?session_key={}",
            session_key
        );

        let drivers_res = match state.http_client.get(&drivers_url).send().await {
            Ok(res) => res,
            Err(e) => {
                tracing::error!("Driver API request failed: {:?}", e);
                continue;
            }
        };

        if drivers_res.status().is_success() {
            let drivers_body = match drivers_res.text().await {
                Ok(b) => b,
                Err(_) => continue,
            };

            let drivers: Vec<Value> = serde_json::from_str(&drivers_body).unwrap_or_default();

            for driver in drivers {
                let driver_number = driver["driver_number"]
                    .as_i64()
                    .map(|n| n.to_string())
                    .unwrap_or_default();

                let points_start = driver["points_start"].as_f64().unwrap_or(0.0);
                let points_current = driver["points_current"].as_f64().unwrap_or(0.0);
                let position = driver["position_current"].as_i64().map(|p| p as i32);

                if let Err(e) = sqlx::query(
                    r#"
                    INSERT INTO "DriverPointsHistory" 
                        (driver_number, session_key, meeting_key, season, round, race_id, points_start, points_current, position)
                    VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)
                    ON CONFLICT (driver_number, session_key) DO UPDATE SET
                        points_start = EXCLUDED.points_start,
                        points_current = EXCLUDED.points_current,
                        position = EXCLUDED.position
                    "#,
                )
                .bind(&driver_number)
                .bind(session_key)
                .bind(meeting_key)
                .bind(&session.season)
                .bind(&session.round)
                .bind(&session.race_id)
                .bind(points_start)
                .bind(points_current)
                .bind(position)
                .execute(&state.db_pool)
                .await
                {
                    tracing::error!("Failed inserting driver history: {:?}", e);
                }
            }
        }

        /*
        CONSTRUCTOR CHAMPIONSHIP
        */

        let teams_url = format!(
            "https://api.openf1.org/v1/championship_teams?session_key={}",
            session_key
        );

        let teams_res = match state.http_client.get(&teams_url).send().await {
            Ok(res) => res,
            Err(e) => {
                tracing::error!("Teams API request failed: {:?}", e);
                continue;
            }
        };

        if teams_res.status().is_success() {
            let teams_body = match teams_res.text().await {
                Ok(b) => b,
                Err(_) => continue,
            };

            let teams: Vec<Value> = serde_json::from_str(&teams_body).unwrap_or_default();

            for team in teams {
                let constructor_id = team["team_name"].as_str().unwrap_or("unknown").to_string();

                let constructor_name = constructor_id.clone();

                let points_start = team["points_start"].as_f64().unwrap_or(0.0);
                let points_current = team["points_current"].as_f64().unwrap_or(0.0);
                let position = team["position_current"].as_i64().map(|p| p as i32);

                if let Err(e) = sqlx::query(
                    r#"
                    INSERT INTO "ConstructorPointsHistory" 
                        (constructor_id, constructor_name, session_key, meeting_key, season, round, race_id, points_start, points_current, position)
                    VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)
                    ON CONFLICT (constructor_id, session_key) DO UPDATE SET
                        points_start = EXCLUDED.points_start,
                        points_current = EXCLUDED.points_current,
                        position = EXCLUDED.position
                    "#,
                )
                .bind(&constructor_id)
                .bind(&constructor_name)
                .bind(session_key)
                .bind(meeting_key)
                .bind(&session.season)
                .bind(&session.round)
                .bind(session.race_id)
                .bind(points_start)
                .bind(points_current)
                .bind(position)
                .execute(&state.db_pool)
                .await
                {
                    tracing::error!("Failed inserting constructor history: {:?}", e);
                }
            }
        }

        tracing::info!(
            "Seeded championship data for session {} (season {}, round {})",
            session_key,
            session.season.clone(),
            session.round
        );
    }

    (
        StatusCode::OK,
        Json(json!({"message": "Championship data seeded successfully"})),
    )
        .into_response()
}
