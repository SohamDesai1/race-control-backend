use std::sync::Arc;

use crate::{
    models::{race::RaceWithCircuit, session::Session},
    utils::state::AppState,
};
use axum::{
    extract::{Path, State},
    response::IntoResponse,
    Json,
};
use chrono::{Datelike, Utc};
use http::StatusCode;
use serde_json::{from_str, json, Value};
use tracing::info;

pub async fn get_race_results(
    State(state): State<Arc<AppState>>,
    round: Option<Path<String>>,
) -> impl IntoResponse {
    let round = round.map(|Path(r)| r).unwrap_or_else(|| "last".to_string());

    let now = chrono::Utc::now();
    let mut year = now.year();
    if now.month() < 3 || (now.month() == 3 && now.day() < 8) {
        year -= 1;
    }

    let url = format!("https://api.jolpi.ca/ergast/f1/{year}/{round}/results/?format=json");
    info!("Fetching race results from URL: {}", url);

    let res = match state.http_client.get(&url).send().await {
        Ok(r) => r,
        Err(e) => {
            tracing::error!("API request failed: {:?}", e);
            return (
                StatusCode::BAD_GATEWAY,
                Json(json!({"error": "Failed to fetch race results"})),
            )
                .into_response();
        }
    };

    let body = match res.text().await {
        Ok(text) => text,
        Err(e) => {
            tracing::error!("Failed reading body: {:?}", e);
            return (
                StatusCode::BAD_GATEWAY,
                Json(json!({"error": "Failed to read API response"})),
            )
                .into_response();
        }
    };

    let parsed: Value = match serde_json::from_str(&body) {
        Ok(val) => val,
        Err(e) => {
            tracing::error!("JSON parse error: {:?}", e);
            return (
                StatusCode::BAD_GATEWAY,
                Json(json!({"error": "Invalid JSON from API"})),
            )
                .into_response();
        }
    };

    let races = &parsed["MRData"]["RaceTable"]["Races"];

    if !races.is_array() || races.as_array().unwrap().is_empty() {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({"error": "No race results found"})),
        )
            .into_response();
    }

    let race = &races[0];
    info!(
        "year: {}, round: {}",
        year.to_string(),
        race["round"].as_str().unwrap_or_default()
    );
    // ---- Fetch DB id safely ----
    let db_id: Option<i64> = sqlx::query_scalar(
        r#"
        SELECT id
        FROM "Races"
        WHERE season = $1 AND round = $2
        "#,
    )
    .bind(year.to_string())
    .bind(race["round"].as_str().unwrap_or_default())
    .fetch_optional(&state.db_pool)
    .await
    .ok()
    .flatten();

    (StatusCode::OK, Json(json!({"id": db_id,"race": race}))).into_response()
}

pub async fn get_all_races_data_db(
    State(state): State<Arc<AppState>>,
    Path(year): Path<String>,
) -> impl IntoResponse {
    // Fetch all races for the year with circuit data using a JOIN
    let res = sqlx::query_as::<_, RaceWithCircuit>(
        r#"
        SELECT
            r.id,
            r.created_at,
            r.season,
            r.round,
            r.date,
            r.time,
            r."raceName" AS race_name,
            r."circuitId" AS circuit_id,
            c."circuitName" AS circuit_name,
            c.locality,
            c.country,
            c.lat,
            c.long
        FROM "Races" r
        LEFT JOIN "Circuits" c ON r."circuitId" = c."circuitId"
        WHERE r.season = $1
        ORDER BY r.date ASC
        "#,
    )
    .bind(&year)
    .fetch_all(&state.db_pool)
    .await;

    match res {
        Ok(races) => {
            if races.is_empty() {
                return (
                    StatusCode::NOT_FOUND,
                    Json(json!({"error": "No races found for this year"})),
                )
                    .into_response();
            }

            (StatusCode::OK, Json(races)).into_response()
        }
        Err(e) => {
            tracing::error!("Failed to fetch races for year {}: {:?}", year, e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to fetch races"})),
            )
                .into_response()
        }
    }
}

pub async fn get_race_data(
    State(state): State<Arc<AppState>>,
    Path((year, round)): Path<(String, String)>,
) -> impl IntoResponse {
    // Fetch race with circuit using existing RaceWithCircuit struct
    let race = sqlx::query_as::<_, RaceWithCircuit>(
        r#"
        SELECT
            r.id,
            r.created_at,
            r.season,
            r.round,
            r.date,
            r.time,
            r."raceName" AS race_name,
            r."circuitId" AS circuit_id,
            c."circuitName" AS circuit_name,
            c.locality,
            c.country,
            c.lat,
            c.long
        FROM "Races" r
        LEFT JOIN "Circuits" c ON r."circuitId" = c."circuitId"
        WHERE r.season = $1 AND r.round = $2
        "#,
    )
    .bind(&year)
    .bind(&round)
    .fetch_optional(&state.db_pool)
    .await;

    let race_data = match race {
        Ok(Some(r)) => r,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({"error": "Race not found"})),
            )
                .into_response();
        }
        Err(e) => {
            tracing::error!("Failed to fetch race: {:?}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to fetch race"})),
            )
                .into_response();
        }
    };

    // Fetch sessions
    let sessions = sqlx::query_as::<_, Session>(
        r#"
            SELECT
            id,
            "raceId",
            "sessionType",
            "date",
            "time"::time AS time,
            "session_key",
            "meeting_key"
            FROM "Sessions"
            WHERE "raceId" = $1
            ORDER BY id ASC
            "#,
    )
    .bind(race_data.id)
    .fetch_all(&state.db_pool)
    .await;

    let sessions_data = match sessions {
        Ok(data) => data,
        Err(e) => {
            tracing::error!("Failed to fetch sessions: {:?}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to fetch sessions"})),
            )
                .into_response();
        }
    };

    (
        StatusCode::OK,
        Json(json!({
            "race": race_data,
            "sessions": sessions_data
        })),
    )
        .into_response()
}

pub async fn get_upcoming_race_data(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let today = Utc::now().date_naive();

    // Fetch all upcoming races with circuit data using a JOIN
    let res = sqlx::query_as::<_, RaceWithCircuit>(
        r#"
        SELECT
            r.id,
            r.created_at,
            r.season,
            r.round,
            r.date,
            r.time,
            r."raceName" AS race_name,
            r."circuitId" AS circuit_id,
            c."circuitName" AS circuit_name,
            c.locality,
            c.country,
            c.lat,
            c.long
        FROM "Races" r
        LEFT JOIN "Circuits" c ON r."circuitId" = c."circuitId"
        WHERE r."date" >= $1
        ORDER BY r."date" ASC
        "#,
    )
    .bind(&today)
    .fetch_all(&state.db_pool)
    .await;

    match res {
        Ok(races) => {
            if races.is_empty() {
                return (StatusCode::OK, Json(json!([]))).into_response();
            }

            (StatusCode::OK, Json(races)).into_response()
        }
        Err(e) => {
            tracing::error!("Failed to fetch upcoming races: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to fetch races"})),
            )
                .into_response()
        }
    }
}

pub async fn _insert_race_and_circuit(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let year = chrono::Utc::now().year().to_string();
    let start = format!("{}-01-01", year);
    let end = format!("{}-12-31", year);

    tracing::info!("start: {}, end: {}", start, end);

    // Fetch races from external API
    let client = reqwest::Client::new();
    let res = match client
        .get(format!(
            "https://api.jolpi.ca/ergast/f1/{}/races/?format=json",
            year
        ))
        .send()
        .await
    {
        Ok(res) => res,
        Err(e) => {
            tracing::error!("Failed to fetch from external API: {:?}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to fetch race data"})),
            )
                .into_response();
        }
    };

    let body = match res.text().await {
        Ok(body) => body,
        Err(e) => {
            tracing::error!("Failed to read response body: {:?}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to parse race data"})),
            )
                .into_response();
        }
    };

    let api_data: Value = match from_str(&body) {
        Ok(data) => data,
        Err(e) => {
            tracing::error!("Failed to parse JSON: {:?}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to parse race data"})),
            )
                .into_response();
        }
    };

    let api_res_body = &api_data["MRData"]["RaceTable"]["Races"];

    // Fetch existing races from database
    let db_races = match sqlx::query_as::<_, RaceWithCircuit>(
        r#"SELECT * FROM "Races" WHERE date >= $1 AND date <= $2"#,
    )
    .bind(&start)
    .bind(&end)
    .fetch_all(&state.db_pool)
    .await
    {
        Ok(races) => races,
        Err(e) => {
            tracing::error!("Failed to fetch races from database: {:?}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to fetch races from database"})),
            )
                .into_response();
        }
    };

    let session_types = [
        "FirstPractice",
        "SecondPractice",
        "ThirdPractice",
        "SprintQualifying",
        "Sprint",
        "Qualifying",
        "Race",
    ];

    let mut inserted_count = 0;
    let mut error_count = 0;

    if let Some(arr) = api_res_body.as_array() {
        for api_race in arr {
            let round = api_race["round"].as_str().unwrap_or_default();

            // Find matching race in database
            let db_race = db_races.iter().find(|r| r.round == round);

            if let Some(race) = db_race {
                tracing::info!("Processing race {} (ID: {})", round, race.id);

                for session_type in &session_types {
                    let (date, time) = if *session_type == "Race" {
                        // For Race session, get date and time from top-level race object
                        (
                            api_race.get("date").and_then(|v| v.as_str()),
                            api_race.get("time").and_then(|v| v.as_str()),
                        )
                    } else {
                        // For other sessions, get from nested object
                        if let Some(session) = api_race.get(*session_type) {
                            (
                                session.get("date").and_then(|v| v.as_str()),
                                session.get("time").and_then(|v| v.as_str()),
                            )
                        } else {
                            (None, None)
                        }
                    };

                    if let (Some(date), Some(time)) = (date, time) {
                        let country = api_race["Circuit"]["Location"]["country"]
                            .as_str()
                            .unwrap_or_default();

                        let result = sqlx::query(
                            r#"
                        INSERT INTO "SessionsTest" (race_id, session_type, date, time, country)
                        VALUES ($1, $2, $3, $4, $5)
                        "#,
                        )
                        .bind(race.id)
                        .bind(*session_type)
                        .bind(date)
                        .bind(time)
                        .bind(country)
                        .execute(&state.db_pool)
                        .await;

                        match result {
                            Ok(result) => {
                                tracing::info!(
                                    "✅ Inserted {} for race {} (rows affected: {})",
                                    session_type,
                                    race.id,
                                    result.rows_affected()
                                );
                                inserted_count += 1;
                            }
                            Err(e) => {
                                tracing::error!(
                                    "❌ Error inserting {} for race {}: {:?}",
                                    session_type,
                                    race.id,
                                    e
                                );
                                error_count += 1;
                            }
                        }
                    }
                }
            }
        }
    }

    (
        StatusCode::OK,
        Json(json!({
            "inserted": inserted_count,
            "errors": error_count,
            "races_processed": db_races.len()
        })),
    )
        .into_response()
}
