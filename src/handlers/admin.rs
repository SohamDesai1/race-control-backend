use std::sync::Arc;

use crate::{
    models::jwt::Claims,
    utils::{scoring, state::AppState},
};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Extension, Json,
};
use serde_json::json;
use tracing::info;

pub async fn calculate_gp_scores(
    Path(gp_id): Path<i64>,
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
) -> impl IntoResponse {
    if claims.sub != "\"admin@gmail.com\"" {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({"error": "Unauthorized: Only Admins can manually calculate scores"})),
        )
            .into_response();
    }
    tracing::info!("Manually triggering score calculation for GP {}", gp_id);

    let session = sqlx::query_as::<_, (Option<i64>, Option<String>)>(
        r#"
        SELECT "session_key"::integer, "sessionType" 
        FROM "Sessions" 
        WHERE "raceId" = $1 AND "sessionType" = 'Race'
        ORDER BY date DESC, time DESC
        LIMIT 1
        "#,
    )
    .bind(gp_id)
    .fetch_optional(&state.db_pool)
    .await;

    let (session_key, _) = match session {
        Ok(Some(s)) => s,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({"error": "No race session found for this GP"})),
            )
                .into_response()
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": format!("Database error: {}", e)})),
            )
                .into_response()
        }
    };

    let session_key = match session_key {
        Some(s) => s,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({"error": "No session key found for race"})),
            )
                .into_response()
        }
    };

    let url = format!(
        "https://api.openf1.org/v1/sessions?session_key={}",
        session_key
    );
    info!(
        "Fetching session data from OpenF1 for session key: {}",
        session_key
    );

    let response = state.http_client.get(&url).send().await;

    let session_data = match response {
        Ok(resp) => resp.json::<serde_json::Value>().await.ok(),
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": format!("Failed to fetch from OpenF1: {}", e)})),
            )
                .into_response()
        }
    };

    let session_end_str = session_data
        .as_ref()
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.first())
        .and_then(|v| v.get("date_end"))
        .and_then(|v| v.as_str());

    if let Some(session_end_str) = session_end_str {
        let session_end_time = chrono::DateTime::parse_from_rfc3339(session_end_str)
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(|_| chrono::Utc::now() - chrono::Duration::days(1));
        if session_end_time > chrono::Utc::now() {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": "Race has not finished yet", "status": session_end_str})),
            )
                .into_response();
        }
    }

    let results_url = format!(
        "https://api.openf1.org/v1/session_result?session_key={}",
        session_key
    );

    let results_response = state.http_client.get(&results_url).send().await;

    let results_data = match results_response {
        Ok(resp) => resp.json::<serde_json::Value>().await.ok(),
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": format!("Failed to fetch results: {}", e)})),
            )
                .into_response()
        }
    };

    let race_results: Vec<scoring::RaceResult> = results_data
        .as_ref()
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|racing| {
                    let position = racing
                        .get("position")
                        .and_then(|v| v.as_i64())
                        .map(|v| v as i64)
                        .unwrap_or(0);

                    let driver_id = racing
                        .get("driver_number")
                        .and_then(|v| v.as_i64())
                        .map(|v| v as i64)
                        .unwrap_or(0);

                    // Validate position and driver_id
                    if position <= 0 || driver_id <= 0 {
                        return None;
                    }

                    Some(scoring::RaceResult {
                        driver_id,
                        position,
                        fastest_lap: racing
                            .get("fastest_lap")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false),
                        dnf: racing.get("dnf").and_then(|v| v.as_bool()).unwrap_or(false),
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    tracing::info!("Race results from OpenF1: {:?}", race_results);

    let driver_mapping: std::collections::HashMap<i64, i64> = sqlx::query_as::<_, (i64, i64)>(
        r#"SELECT id, driver_id FROM "fantasy_drivers" WHERE year = 2026"#,
    )
    .fetch_all(&state.db_pool)
    .await
    .ok()
    .map(|rows| rows.into_iter().map(|(id, num)| (num, id)).collect())
    .unwrap_or_default();
    println!("Driver mapping: {:?}", driver_mapping);

    let race_results: Vec<scoring::RaceResult> = race_results
        .into_iter()
        .map(|r| {
            info!("Original race driver_id: {:?}", r.driver_id);
            let actual_driver_id: i64 = driver_mapping
                .get(&r.driver_id)
                .copied()
                .unwrap_or(r.driver_id);
            scoring::RaceResult {
                driver_id: actual_driver_id,
                ..r
            }
        })
        .collect();

    tracing::info!("Race results after mapping: {:?}", race_results);

    let constructors: Vec<(i64, String)> =
        sqlx::query_as(r#"SELECT id, name FROM "fantasy_constructors" WHERE year = 2026"#)
            .fetch_all(&state.db_pool)
            .await
            .ok()
            .map(|rows: Vec<(i64, String)>| rows)
            .unwrap_or_default();

    let mut constructor_results: Vec<scoring::ConstructorResult> = vec![];

    let teams_url = format!(
        "https://api.openf1.org/v1/championship_teams?session_key={}",
        session_key
    );

    if let Ok(response) = state.http_client.get(&teams_url).send().await {
        if let Ok(data) = response.json::<serde_json::Value>().await {
            if let Some(arr) = data.as_array() {
                for team_data in arr {
                    let openf1_team_name = team_data
                        .get("team_name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");

                    let position = team_data
                        .get("position_current")
                        .and_then(|v| v.as_i64())
                        .map(|v| v as i64)
                        .unwrap_or(0);

                    // Normalize team names for better matching
                    let normalized_openf1 = openf1_team_name.to_lowercase().replace(" ", "");

                    for (constructor_id, fantasy_name) in &constructors {
                        let normalized_fantasy = fantasy_name.to_lowercase().replace(" ", "");

                        // Try exact match first
                        if normalized_openf1 == normalized_fantasy {
                            constructor_results.push(scoring::ConstructorResult {
                                constructor_id: *constructor_id,
                                position,
                            });
                            break;
                        }

                        // Fallback to contains match
                        if normalized_openf1.contains(&normalized_fantasy) {
                            constructor_results.push(scoring::ConstructorResult {
                                constructor_id: *constructor_id,
                                position,
                            });
                            break;
                        }
                    }
                }
            }
        }
    }

    let fastest_lap_driver_id = race_results
        .iter()
        .find(|r| r.fastest_lap)
        .map(|r| r.driver_id);

    if let Err(e) = scoring::calculate_gp_scores(
        &state,
        gp_id,
        race_results.clone(),
        constructor_results.clone(),
        fastest_lap_driver_id,
    )
    .await
    {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("Failed to calculate scores: {}", e)})),
        )
            .into_response();
    }

    let race_info: Option<(String,)> = sqlx::query_as(
        r#"SELECT season FROM "Races" WHERE id = $1"#,
    )
    .bind(gp_id)
    .fetch_optional(&state.db_pool)
    .await
    .ok()
    .flatten();

    let year = race_info
        .map(|(s,)| s.parse::<i64>().unwrap_or(2026))
        .unwrap_or(2026);

    if let Err(e) = scoring::update_prices_after_gp(
        &state,
        gp_id,
        year,
        &race_results,
        &constructor_results,
    )
    .await
    {
        tracing::warn!("Failed to update prices: {}", e);
    }

    (
        StatusCode::OK,
        Json(json!({"message": "Scores calculated successfully", "gp_id": gp_id})),
    )
        .into_response()
}

pub async fn get_scoring_status(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
) -> impl IntoResponse {
    if claims.sub != "\"admin@gmail.com\"" {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({"error": "Unauthorized: Only Admins can manually calculate scores"})),
        )
            .into_response();
    }
    let active_gps = sqlx::query_as::<_, (i64, String, Option<String>)>(
        r#"
        SELECT r.id, r."raceName", r.season
        FROM "Races" r
        WHERE r.date >= CURRENT_DATE - INTERVAL '7 days'
        ORDER BY r.date DESC
        "#,
    )
    .fetch_all(&state.db_pool)
    .await;

    match active_gps {
        Ok(gps) => {
            let gps_with_teams: Vec<serde_json::Value> = gps
                .iter()
                .map(|(id, name, season)| {
                    json!({
                        "gp_id": id,
                        "race_name": name,
                        "season": season,
                    })
                })
                .collect();

            (StatusCode::OK, Json(gps_with_teams)).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("Failed to fetch status: {}", e)})),
        )
            .into_response(),
    }
}

pub async fn lock_teams(
    Path(gp_id): Path<i64>,
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
) -> impl IntoResponse {
    if claims.sub != "\"admin@gmail.com\"" {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({"error": "Unauthorized: Only Admins can manually calculate scores"})),
        )
            .into_response();
    }
    tracing::info!("Manually triggering team lock for GP {}", gp_id);

    // Get qualifying session key
    let session = sqlx::query_as::<_, (Option<i64>,)>(
        r#"SELECT "session_key"::integer FROM "Sessions" WHERE "raceId" = $1 AND "sessionType" = 'Qualifying'"#,
    )
    .bind(gp_id)
    .fetch_optional(&state.db_pool)
    .await;

    let session_key = match session {
        Ok(Some(s)) => s.0,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({"error": "No qualifying session found for this GP"})),
            )
                .into_response()
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": format!("Database error: {}", e)})),
            )
                .into_response()
        }
    };

    let session_key = match session_key {
        Some(s) => s,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({"error": "No session key found for qualifying"})),
            )
                .into_response()
        }
    };

    // Check qualifying status via OpenF1
    let url = format!(
        "https://api.openf1.org/v1/session?session_key={}",
        session_key
    );

    let response = match state.http_client.get(&url).send().await {
        Ok(resp) => resp,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": format!("Failed to fetch from OpenF1: {}", e)})),
            )
                .into_response()
        }
    };

    let session_data: serde_json::Value = match response.json().await {
        Ok(data) => data,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": format!("Failed to parse OpenF1 response: {}", e)})),
            )
                .into_response()
        }
    };

    let quali_status = session_data
        .as_array()
        .and_then(|arr| arr.first())
        .and_then(|v| v.get("session_status"))
        .and_then(|v| v.as_str());

    if quali_status.is_none() {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({"error": "Could not determine qualifying status"})),
        )
            .into_response();
    }

    // Lock teams
    let result = sqlx::query(
        r#"UPDATE "fantasy_teams" SET is_locked = true WHERE gp_id = $1 AND is_locked = false"#,
    )
    .bind(gp_id)
    .execute(&state.db_pool)
    .await;

    match result {
        Ok(res) => (
            StatusCode::OK,
            Json(json!({
                "message": "Teams locked successfully",
                "gp_id": gp_id,
                "qualifying_status": quali_status,
                "teams_locked": res.rows_affected()
            })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("Failed to lock teams: {}", e)})),
        )
            .into_response(),
    }
}
