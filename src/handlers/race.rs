use std::sync::Arc;

use crate::{
    models::race::{Race, RaceWithCircuit},
    utils::state::AppState,
};
use axum::{
    extract::{Path, State},
    response::IntoResponse,
    Json,
};
use chrono::Datelike;
use http::StatusCode;
use serde_json::{from_str, json, Value};

pub async fn get_race_results(
    State(state): State<Arc<AppState>>,
    round: Option<Path<String>>,
) -> impl IntoResponse {
    let round = round.map(|Path(r)| r).unwrap_or_else(|| "last".to_string());
    let res = state
        .http_client
        .get(format!(
            "https://api.jolpi.ca/ergast/f1/2025/{round}/results/?format=json"
        ))
        .send()
        .await
        .unwrap();
    let body = res.text().await.unwrap();
    let res: Value = from_str(&body).unwrap();
    let res_body = &res["MRData"]["RaceTable"]["Races"];
    (StatusCode::OK, Json(res_body)).into_response()
}

pub async fn get_race_data_db(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let response = sqlx::query_as::<_, Race>(r#"SELECT * FROM "Races""#)
        .fetch_all(&state.db_pool)
        .await;
    // }
    match response {
        Ok(users) => (StatusCode::OK, Json(users)).into_response(),
        Err(e) => {
            tracing::error!("Database error fetching users: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to fetch users"})),
            )
                .into_response()
        }
    }
}

pub async fn get_upcoming_race_data(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let today = chrono::Utc::now().date_naive();

    // Fetch all upcoming races with circuit data using a JOIN
    let res = sqlx::query_as::<_, RaceWithCircuit>(
        r#"
    SELECT
        r.id,
        r."created_at",
        r."season",
        r."round",
        r."date",
        r."time",
        r."raceName"   AS race_name,
        r."circuitId"  AS circuit_id,
        c."circuitName"       AS circuit_name,
        c."locality",
        c."country",
        c."lat",
        c."long"
    FROM "Races" r
    LEFT JOIN "Circuits" c
      ON r."circuitId" = c."circuitId"
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
    let db_races =
        match sqlx::query_as::<_, Race>(r#"SELECT * FROM "Races" WHERE date >= $1 AND date <= $2"#)
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

    for api_race in api_res_body.as_array().unwrap_or(&vec![]) {
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
