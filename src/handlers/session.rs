use crate::{
    models::{
        cache::CacheEntry,
        session::Session,
        telemetry::{
            CarDataPoint, DriverLapGraph, FastestLapSector, Lap, LapPosition, LapRecord,
            LocationPoint, PacePoint, PaceQuery, PositionRecord, QualifyingRanking,
            QualifyingRankings, SpeedDistance,
        },
    },
    utils::{race_utils::map_session_name, state::AppState},
};
use axum::{
    extract::{Path, Query, State},
    response::IntoResponse,
    Json,
};
use chrono::{DateTime, Datelike, Duration, NaiveDate, Utc};
use http::StatusCode;
use serde_json::{from_str, json, Value};

use std::{
    cmp::Ordering::{Equal, Greater, Less},
    time::Duration as StdDuration,
};
use std::{collections::HashMap, sync::Arc};
use tokio::time::{sleep, Duration as TokioDuration};
use tracing::{info, warn};

pub async fn get_sessions(
    State(state): State<Arc<AppState>>,
    Path((race_id, year)): Path<(i32, Option<i32>)>,
) -> impl IntoResponse {
    let year = year.clone().unwrap_or_else(|| chrono::Utc::now().year());
    let start = NaiveDate::from_ymd_opt(year, 1, 1).unwrap();
    let end = NaiveDate::from_ymd_opt(year, 12, 31).unwrap();

    // Fetch sessions from database
    let res = sqlx::query_as::<_, Session>(
        r#"
    SELECT
    id,
    "raceId",
    "sessionType",
    "date",
    "time",
    "session_key",
    "meeting_key"
    FROM "Sessions"
    WHERE "date" >= $1
    AND "date" <= $2
    AND "raceId" = $3
    ORDER BY id ASC
    "#,
    )
    .bind(start)
    .bind(end)
    .bind(&race_id)
    .fetch_all(&state.db_pool)
    .await;

    match res {
        Ok(sessions) => {
            if sessions.is_empty() {
                return (
                    StatusCode::NOT_FOUND,
                    Json(json!({ "error": "No sessions found for this race" })),
                )
                    .into_response();
            }

            // Check which sessions are missing session_key
            let sessions_without_keys: Vec<&Session> = sessions
                .iter()
                .filter(|session| session.session_key.is_none())
                .collect();

            // If all sessions have keys, return immediately
            if sessions_without_keys.is_empty() {
                return (
                    StatusCode::OK,
                    Json(json!({"sessions": sessions, "status": "completed"})),
                )
                    .into_response();
            }

            // If some sessions are missing keys, try to fetch from OpenF1
            info!(
                "Found {} sessions without keys, attempting to fetch from OpenF1",
                sessions_without_keys.len()
            );

            // Get the date from the first session for date range query
            let start_date = match sessions.first().and_then(|s| s.date) {
                Some(d) => d,
                None => {
                    return (
                        StatusCode::OK,
                        Json(json!({
                            "sessions": sessions,
                            "status": "scheduled",
                            "message": "Future Event, data not yet available"
                        })),
                    )
                        .into_response();
                }
            };

            let end_date = match sessions.last().and_then(|s| s.date) {
                Some(d) => d,
                None => start_date,
            };

            // Create date range query
            let fallback_url = format!(
                "https://api.openf1.org/v1/sessions?year={}&date_start>={}&date_end<={}",
                year, start_date, end_date
            );

            info!("Fetching from OpenF1: {}", fallback_url);

            let fallback_res = state.http_client.get(&fallback_url).send().await;

            match fallback_res {
                Ok(response) => {
                    let fallback_body = response.text().await.unwrap_or_default();
                    let fallback_sessions: Vec<Value> =
                        from_str(&fallback_body).unwrap_or_default();

                    if fallback_sessions.is_empty() {
                        info!("No data available in OpenF1 yet, returning current sessions");
                        return (
                            StatusCode::OK,
                            Json(json!({
                                "sessions": sessions,
                                "status": "scheduled",
                                "message": "Future Event, data not yet available"
                            })),
                        )
                            .into_response();
                    }

                    // Only update sessions that don't have session_key
                    let mut updated_sessions = Vec::new();
                    let mut update_count = 0;

                    for session in fallback_sessions {
                        if let Some(ext_name) = session.get("session_name").and_then(|v| v.as_str())
                        {
                            if let Some(mapped_name) = map_session_name(ext_name) {
                                // Check if this session type is missing session_key in our DB
                                let needs_update = sessions_without_keys
                                    .iter()
                                    .any(|db_session| db_session.session_type == mapped_name);

                                if needs_update {
                                    if let (Some(session_key), Some(meeting_key)) = (
                                        session.get("session_key").and_then(|v| v.as_i64()),
                                        session.get("meeting_key").and_then(|v| v.as_i64()),
                                    ) {
                                        info!(
                                            "Updating {} with session_key: {}, meeting_key: {}",
                                            mapped_name, session_key, meeting_key
                                        );

                                        let update_res = sqlx::query(
                                            r#"
                                            UPDATE "Sessions"
                                            SET "session_key" = $1, "meeting_key" = $2
                                            WHERE "sessionType" = $3 AND "raceId" = $4
                                            "#,
                                        )
                                        .bind(session_key)
                                        .bind(meeting_key)
                                        .bind(mapped_name)
                                        .bind(&race_id)
                                        .execute(&state.db_pool)
                                        .await;

                                        if let Err(err) = update_res {
                                            tracing::error!(
                                                "Failed to update {} in database: {:?}",
                                                mapped_name,
                                                err
                                            );
                                        } else {
                                            updated_sessions.push(json!({
                                                "session_type": mapped_name,
                                                "session_key": session_key,
                                                "meeting_key": meeting_key
                                            }));
                                            update_count += 1;
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Always fetch the latest data from database after attempting updates
                    let updated_res = sqlx::query_as::<_, Session>(
                        r#"SELECT * FROM "Sessions" WHERE "raceId" = $1 ORDER BY id ASC"#,
                    )
                    .bind(&race_id)
                    .fetch_all(&state.db_pool)
                    .await;

                    match updated_res {
                        Ok(updated_sessions_data) => {
                            // Check if all sessions now have keys
                            let all_complete = updated_sessions_data
                                .iter()
                                .all(|session| session.session_key.is_some());

                            let status = if all_complete { "completed" } else { "partial" };

                            let mut response = json!({
                                "sessions": updated_sessions_data,
                                "status": status,
                            });

                            if update_count > 0 {
                                response["updated_count"] = json!(update_count);
                                response["updated_sessions"] = json!(updated_sessions);
                                response["message"] =
                                    json!(format!("Updated {} session(s)", update_count));
                            } else {
                                response["message"] =
                                    json!("Some sessions completed, others still scheduled");
                            }

                            return (StatusCode::OK, Json(response)).into_response();
                        }
                        Err(err) => {
                            tracing::error!("Failed to fetch updated sessions: {:?}", err);
                            return (
                                StatusCode::OK,
                                Json(json!({
                                    "sessions": sessions,
                                    "status": "partial",
                                    "message": "Some sessions updated but failed to refetch"
                                })),
                            )
                                .into_response();
                        }
                    }
                }
                Err(err) => {
                    tracing::error!("OpenF1 API request failed: {:?}", err);
                    return (
                        StatusCode::OK,
                        Json(json!({
                            "sessions": sessions,
                            "status": "partial",
                            "message": "Some sessions may be completed, OpenF1 API unavailable"
                        })),
                    )
                        .into_response();
                }
            }
        }
        Err(err) => {
            tracing::error!("Database query failed: {:?}", err);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "Failed to fetch sessions from database" })),
            )
                .into_response();
        }
    }
}

pub async fn get_session_data(
    State(state): State<Arc<AppState>>,
    Path(session_key): Path<String>,
) -> impl IntoResponse {
    let res = state
        .http_client
        .get(format!(
            "https://api.openf1.org/v1/session_result?session_key={session_key}"
        ))
        .send()
        .await
        .unwrap();

    let body = res.text().await.unwrap();
    let res: Value = from_str(&body).unwrap();

    return (StatusCode::OK, Json(res)).into_response();
}

fn _parse_lap_time(time_str: &str) -> Option<f64> {
    if time_str.is_empty() {
        return None;
    }

    let parts: Vec<&str> = time_str.split(':').collect();
    if parts.len() != 2 {
        return None;
    }

    let minutes: f64 = parts[0].parse().ok()?;
    let seconds: f64 = parts[1].parse().ok()?;

    Some(minutes * 60.0 + ((seconds * 100.0).round() / 100.0))
}

const TTL_SECONDS: i64 = 60 * 60;

pub async fn get_quali_session_data(
    State(state): State<Arc<AppState>>,
    Path((year, round)): Path<(String, String)>,
) -> impl IntoResponse {
    let cache_key = format!("quali_session_{}_{}", year, round);
    if let Some(entry) = state.quali_session_cache.get(&cache_key) {
        if !entry.is_expired() {
            info!("CACHE HIT for qual session {} round {}", year, round);
            return (StatusCode::OK, Json(entry.value.clone())).into_response();
        }
        info!(
            "CACHE EXPIRED for for qual session {} round {}, recomputing…",
            year, round
        );
        drop(entry);
        state.quali_session_cache.remove(&cache_key);
    }
    let res = state
        .http_client
        .get(format!(
            "https://api.jolpi.ca/ergast/f1/{}/{}/qualifying?format=json",
            year, round
        ))
        .send()
        .await;

    match res {
        Ok(res) => {
            let body: String = res.text().await.unwrap();
            let res_body: Value = from_str(&body).unwrap();

            let qualifying_results = match res_body["MRData"]["RaceTable"]["Races"]
                .as_array()
                .and_then(|races| races.first())
                .and_then(|race| race["QualifyingResults"].as_array())
            {
                Some(results) => results,
                None => {
                    return (
                        StatusCode::NOT_FOUND,
                        Json(json!({ "error": "No qualifying results found" })),
                    )
                        .into_response();
                }
            };

            let mut q1_rankings = Vec::new();
            let mut q2_rankings = Vec::new();
            let mut q3_rankings = Vec::new();

            // Collect all times for each session
            for result in qualifying_results {
                let driver_number = result["number"].as_str().unwrap_or("").to_string();
                let driver_code = result["Driver"]["code"].as_str().unwrap_or("").to_string();
                let driver_name = format!(
                    "{} {}",
                    result["Driver"]["givenName"].as_str().unwrap_or(""),
                    result["Driver"]["familyName"].as_str().unwrap_or("")
                );
                let constructor = result["Constructor"]["name"]
                    .as_str()
                    .unwrap_or("")
                    .to_string();

                if let Some(q1_time) = result["Q1"].as_str() {
                    if !q1_time.is_empty() {
                        q1_rankings.push(QualifyingRanking {
                            position: 0,
                            driver_number: Some(driver_number.clone()),
                            driver_code: Some(driver_code.clone()),
                            driver_name: Some(driver_name.clone()),
                            constructor: Some(constructor.clone()),
                            time: q1_time.to_string(),
                            time_seconds: _parse_lap_time(q1_time),
                        });
                    } else {
                        q1_rankings.push(QualifyingRanking {
                            position: 0,
                            driver_number: Some(driver_number.clone()),
                            driver_code: Some(driver_code.clone()),
                            driver_name: Some(driver_name.clone()),
                            constructor: Some(constructor.clone()),
                            time: "".to_string(),
                            time_seconds: None,
                        });
                    }
                }

                if let Some(q2_time) = result["Q2"].as_str() {
                    if !q2_time.is_empty() {
                        q2_rankings.push(QualifyingRanking {
                            position: 0,
                            driver_number: Some(driver_number.clone()),
                            driver_code: Some(driver_code.clone()),
                            driver_name: Some(driver_name.clone()),
                            constructor: Some(constructor.clone()),
                            time: q2_time.to_string(),
                            time_seconds: _parse_lap_time(q2_time),
                        });
                    } else {
                        q2_rankings.push(QualifyingRanking {
                            position: 0,
                            driver_number: Some(driver_number.clone()),
                            driver_code: Some(driver_code.clone()),
                            driver_name: Some(driver_name.clone()),
                            constructor: Some(constructor.clone()),
                            time: "".to_string(),
                            time_seconds: None,
                        });
                    }
                }

                if let Some(q3_time) = result["Q3"].as_str() {
                    if !q3_time.is_empty() {
                        q3_rankings.push(QualifyingRanking {
                            position: 0,
                            driver_number: Some(driver_number.clone()),
                            driver_code: Some(driver_code.clone()),
                            driver_name: Some(driver_name.clone()),
                            constructor: Some(constructor.clone()),
                            time: q3_time.to_string(),
                            time_seconds: _parse_lap_time(q3_time),
                        });
                    } else {
                        q3_rankings.push(QualifyingRanking {
                            position: 0,
                            driver_number: Some(driver_number.clone()),
                            driver_code: Some(driver_code.clone()),
                            driver_name: Some(driver_name.clone()),
                            constructor: Some(constructor.clone()),
                            time: "".to_string(),
                            time_seconds: None,
                        });
                    }
                }
            }

            q1_rankings.sort_by(|a, b| match (a.time_seconds, b.time_seconds) {
                (Some(time_a), Some(time_b)) => time_a.partial_cmp(&time_b).unwrap_or(Equal),
                (Some(_), None) => Less,
                (None, Some(_)) => Greater,
                (None, None) => Equal,
            });
            for (i, ranking) in q1_rankings.iter_mut().enumerate() {
                ranking.position = (i + 1) as u32;
            }

            q2_rankings.sort_by(|a, b| match (a.time_seconds, b.time_seconds) {
                (Some(time_a), Some(time_b)) => time_a.partial_cmp(&time_b).unwrap_or(Equal),
                (Some(_), None) => Less,
                (None, Some(_)) => Greater,
                (None, None) => Equal,
            });
            for (i, ranking) in q2_rankings.iter_mut().enumerate() {
                ranking.position = (i + 1) as u32;
            }

            q3_rankings.sort_by(|a, b| match (a.time_seconds, b.time_seconds) {
                (Some(time_a), Some(time_b)) => time_a.partial_cmp(&time_b).unwrap_or(Equal),
                (Some(_), None) => Less,
                (None, Some(_)) => Greater,
                (None, None) => Equal,
            });
            for (i, ranking) in q3_rankings.iter_mut().enumerate() {
                ranking.position = (i + 1) as u32;
            }

            let rankings = QualifyingRankings {
                q1: q1_rankings,
                q2: q2_rankings,
                q3: q3_rankings,
            };
            state
                .quali_session_cache
                .insert(cache_key, CacheEntry::new(rankings.clone(), TTL_SECONDS));
            return (StatusCode::OK, Json(rankings)).into_response();
        }
        Err(e) => {
            warn!("Failed to fetch qualifying data: {:?}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "Failed to fetch qualifying data" })),
            )
                .into_response();
        }
    }
}

pub async fn get_sprint_quali_session_data(
    State(state): State<Arc<AppState>>,
    Path(session_key): Path<String>,
) -> impl IntoResponse {
    let res = state
        .http_client
        .get(format!(
            "https://api.openf1.org/v1/session_result?session_key={session_key}"
        ))
        .send()
        .await;

    match res {
        Ok(response) => {
            let body: String = response.text().await.unwrap();
            let res_body: Value = from_str(&body).unwrap();

            // Extract meeting_key from the first result to fetch driver info
            let meeting_key = res_body
                .as_array()
                .and_then(|arr| arr.first())
                .and_then(|first| first["meeting_key"].as_u64())
                .map(|k| k as u32);

            if meeting_key.is_none() {
                return (
                    StatusCode::NOT_FOUND,
                    Json(json!({ "error": "No meeting key found in session results" })),
                )
                    .into_response();
            }

            let _meeting_key = meeting_key.unwrap();

            let mut q1_rankings = Vec::new();
            let mut q2_rankings = Vec::new();
            let mut q3_rankings = Vec::new();

            // Process each driver's qualifying result
            if let Some(results_array) = res_body.as_array() {
                for result in results_array {
                    let driver_number = result["driver_number"].as_u64().unwrap_or(0) as u32;
                    let _position = result["position"].as_u64().unwrap_or(0) as u32;

                    // Get driver info from our mapping

                    // Extract Q1, Q2, Q3 times from duration array
                    let duration_array = result["duration"].as_array();
                    let gap_array = result["gap_to_leader"].as_array();

                    // Process Q1
                    if let (Some(durations), Some(_gaps)) = (duration_array, gap_array) {
                        if let Some(q1_duration) = durations.get(0) {
                            if let Some(q1_time) = q1_duration.as_f64() {
                                q1_rankings.push(QualifyingRanking {
                                    position: 0, // Will be set after sorting
                                    driver_number: Some(driver_number.to_string()),
                                    time: format!("{:.3}", q1_time),
                                    time_seconds: Some(q1_time),
                                    driver_code: None,
                                    driver_name: None,
                                    constructor: None,
                                });
                            } else {
                                q1_rankings.push(QualifyingRanking {
                                    position: 0,
                                    driver_number: Some(driver_number.to_string()),
                                    time: "".to_string(),
                                    time_seconds: None,
                                    driver_code: None,
                                    driver_name: None,
                                    constructor: None,
                                });
                            }
                        }

                        // Process Q2
                        if let Some(q2_duration) = durations.get(1) {
                            if let Some(q2_time) = q2_duration.as_f64() {
                                q2_rankings.push(QualifyingRanking {
                                    position: 0,
                                    driver_number: Some(driver_number.to_string()),
                                    time: format!("{:.3}", q2_time),
                                    time_seconds: Some(q2_time),
                                    driver_code: None,
                                    driver_name: None,
                                    constructor: None,
                                });
                            } else {
                                q2_rankings.push(QualifyingRanking {
                                    position: 0,
                                    driver_number: Some(driver_number.to_string()),

                                    time: "".to_string(),
                                    time_seconds: None,
                                    driver_code: None,
                                    driver_name: None,
                                    constructor: None,
                                });
                            }
                        }

                        // Process Q3
                        if let Some(q3_duration) = durations.get(2) {
                            if let Some(q3_time) = q3_duration.as_f64() {
                                q3_rankings.push(QualifyingRanking {
                                    position: 0,
                                    driver_number: Some(driver_number.to_string()),
                                    time: format!("{:.3}", q3_time),
                                    time_seconds: Some(q3_time),
                                    driver_code: None,
                                    driver_name: None,
                                    constructor: None,
                                });
                            } else {
                                q3_rankings.push(QualifyingRanking {
                                    position: 0,
                                    driver_number: Some(driver_number.to_string()),
                                    time: "".to_string(),
                                    time_seconds: None,
                                    driver_code: None,
                                    driver_name: None,
                                    constructor: None,
                                });
                            }
                        }
                    }
                }
            }

            // Sort and assign positions for each session
            q1_rankings.sort_by(|a, b| match (a.time_seconds, b.time_seconds) {
                (Some(time_a), Some(time_b)) => time_a.partial_cmp(&time_b).unwrap_or(Equal),
                (Some(_), None) => Less,
                (None, Some(_)) => Greater,
                (None, None) => Equal,
            });
            for (i, ranking) in q1_rankings.iter_mut().enumerate() {
                ranking.position = (i + 1) as u32;
            }

            q2_rankings.sort_by(|a, b| match (a.time_seconds, b.time_seconds) {
                (Some(time_a), Some(time_b)) => time_a.partial_cmp(&time_b).unwrap_or(Equal),
                (Some(_), None) => Less,
                (None, Some(_)) => Greater,
                (None, None) => Equal,
            });
            for (i, ranking) in q2_rankings.iter_mut().enumerate() {
                ranking.position = (i + 1) as u32;
            }

            q3_rankings.sort_by(|a, b| match (a.time_seconds, b.time_seconds) {
                (Some(time_a), Some(time_b)) => time_a.partial_cmp(&time_b).unwrap_or(Equal),
                (Some(_), None) => Less,
                (None, Some(_)) => Greater,
                (None, None) => Equal,
            });
            for (i, ranking) in q3_rankings.iter_mut().enumerate() {
                ranking.position = (i + 1) as u32;
            }

            let rankings = QualifyingRankings {
                q1: q1_rankings,
                q2: q2_rankings,
                q3: q3_rankings,
            };

            return (StatusCode::OK, Json(rankings)).into_response();
        }
        Err(e) => {
            warn!("Failed to fetch qualifying data: {:?}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "Failed to fetch qualifying data" })),
            )
                .into_response();
        }
    }
}
// Helper to parse RFC3339 date string to chrono::DateTime<Utc>
fn _parse_date(date: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(date)
        .ok()
        .map(|dt| dt.with_timezone(&Utc))
}

pub async fn fetch_driver_telemetry(
    State(state): State<Arc<AppState>>,
    Path((session_key, driver_number)): Path<(i32, i32)>,
) -> impl IntoResponse {
    let session_key = session_key.clone();
    let driver_number = driver_number.clone();
    let cache_key = format!(
        "session_drivers_telemetry_graph_{}_{}",
        session_key, driver_number
    );
    if let Some(entry) = state.fetch_driver_telemetry_cache.get(&cache_key) {
        if !entry.is_expired() {
            info!(
                "CACHE HIT for session {} driver {}",
                session_key, driver_number
            );
            return (StatusCode::OK, Json(entry.value.clone())).into_response();
        }
        info!(
            "CACHE EXPIRED for session {} driver {}, recomputing…",
            session_key, driver_number
        );
        drop(entry);
        state.fetch_driver_telemetry_cache.remove(&cache_key);
    }
    // 1. Get latest lap for driver
    let laps_url = format!(
        "https://api.openf1.org/v1/laps?session_key={}&driver_number={}",
        session_key, driver_number
    );
    let laps_res = state.http_client.get(laps_url).send().await.unwrap();
    let laps_body = laps_res.text().await.unwrap();
    let laps: Vec<Value> = serde_json::from_str(&laps_body).unwrap();
    // Filter for lap_duration < 120.0 and get latest date_start
    let mut filtered: Vec<&Value> = laps
        .iter()
        .filter(|lap| {
            lap.get("lap_duration")
                .and_then(|v| v.as_f64())
                .map(|d| d < 120.0)
                .unwrap_or(false)
                && lap.get("date_start").and_then(|v| v.as_str()).is_some()
        })
        .collect();
    filtered.sort_by(|a, b| {
        let a_date = a.get("date_start").and_then(|v| v.as_str()).unwrap();
        let b_date = b.get("date_start").and_then(|v| v.as_str()).unwrap();
        b_date.cmp(a_date)
    });
    let latest_lap = match filtered.first() {
        Some(lap) => lap,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({"error": "No valid lap found"})),
            )
                .into_response();
        }
    };
    // Get date_start and lap_duration
    let start = _parse_date(latest_lap["date_start"].as_str().unwrap()).unwrap();
    let lap_duration = latest_lap["lap_duration"].as_f64().unwrap();
    let end = start + Duration::milliseconds((lap_duration * 1000.0) as i64);
    let date_start_str = start.to_rfc3339();
    let date_end_str = end.to_rfc3339();

    // 2. Fetch location data for this lap
    let location_url = format!(
        "https://api.openf1.org/v1/location?session_key={}&driver_number={}&date>{}&date<{}",
        session_key, driver_number, date_start_str, date_end_str
    );
    let location_res = state.http_client.get(&location_url).send().await.unwrap();
    let location_body = location_res.text().await.unwrap();
    let mut location_points: Vec<Value> = serde_json::from_str(&location_body).unwrap();
    // Sort by date
    location_points.sort_by(|a, b| a["date"].as_str().unwrap().cmp(b["date"].as_str().unwrap()));
    // Compute cumulative distance for each location point
    let mut cumulative = 0.0;
    let mut distances = Vec::with_capacity(location_points.len());
    distances.push(0.0);
    for i in 1..location_points.len() {
        let x1 = location_points[i - 1]["x"].as_f64().unwrap();
        let y1 = location_points[i - 1]["y"].as_f64().unwrap();
        let z1 = location_points[i - 1]["z"].as_f64().unwrap();
        let x2 = location_points[i]["x"].as_f64().unwrap();
        let y2 = location_points[i]["y"].as_f64().unwrap();
        let z2 = location_points[i]["z"].as_f64().unwrap();
        let d = ((x2 - x1).powi(2) + (y2 - y1).powi(2) + (z2 - z1).powi(2)).sqrt();
        cumulative += d;
        distances.push(cumulative);
    }
    // 3. Fetch car_data for this lap
    let car_data_url = format!(
        "https://api.openf1.org/v1/car_data?session_key={}&driver_number={}&date>{}&date<{}",
        session_key, driver_number, date_start_str, date_end_str
    );
    let car_data_res = state.http_client.get(&car_data_url).send().await.unwrap();
    let car_data_body = car_data_res.text().await.unwrap();
    let mut car_data_points: Vec<CarDataPoint> = from_str(&car_data_body).unwrap();
    // Sort by date
    car_data_points.sort_by(|a, b| a.date.cmp(&b.date));

    // 4. For each car_data point, find the closest location point by timestamp and assign its cumulative distance
    let location_times: Vec<_> = location_points
        .iter()
        .map(|p| _parse_date(p["date"].as_str().unwrap()).unwrap())
        .collect();
    let mut result = Vec::with_capacity(car_data_points.len());
    for car_point in &car_data_points {
        let car_time = _parse_date(&car_point.date).unwrap();
        // Find the closest location point
        let (closest_idx, _) = location_times
            .iter()
            .enumerate()
            .min_by_key(|(_, loc_time)| {
                (loc_time.timestamp_millis() - car_time.timestamp_millis()).abs()
            })
            .unwrap();
        let distance = distances[closest_idx];
        result.push(SpeedDistance {
            speed: car_point.speed,
            distance: distance / 10.0,
        });
    }
    // save to database
    state
        .fetch_driver_telemetry_cache
        .insert(cache_key, CacheEntry::new(result.clone(), TTL_SECONDS));

    (StatusCode::OK, Json(result)).into_response()
}

pub async fn get_drivers_position_telemetry(
    State(state): State<Arc<AppState>>,
    Path(session_key): Path<String>,
) -> Json<Vec<DriverLapGraph>> {
    let cache_key = format!("session_drivers_position_graph_{}", session_key);

    if let Some(entry) = state.get_drivers_position_telemetry_cache.get(&cache_key) {
        if !entry.is_expired() {
            info!("CACHE HIT for session {}", session_key);
            return Json(entry.value.clone());
        }
        info!("CACHE EXPIRED for session {}, recomputing…", session_key);
        drop(entry);
        state
            .get_drivers_position_telemetry_cache
            .remove(&cache_key);
    }

    info!("CACHE MISS for session {}, computing…", session_key);

    let laps_url = format!("https://api.openf1.org/v1/laps?session_key={}", session_key);
    let laps_resp = state.http_client.get(&laps_url).send().await.unwrap();
    let laps_body = laps_resp.text().await.unwrap();
    let laps: Vec<LapRecord> = from_str(&laps_body).unwrap();

    let mut laps_by_driver: HashMap<u32, Vec<LapRecord>> = HashMap::new();
    for lap in laps {
        laps_by_driver
            .entry(lap.driver_number)
            .or_default()
            .push(lap);
    }

    let positions_url = format!(
        "https://api.openf1.org/v1/position?session_key={}",
        session_key
    );
    let positions_resp = state.http_client.get(&positions_url).send().await.unwrap();
    let positions_body = positions_resp.text().await.unwrap();
    let positions: Vec<PositionRecord> = from_str(&positions_body).unwrap();

    let mut positions_by_driver: HashMap<u32, Vec<PositionRecord>> = HashMap::new();
    for pos in positions {
        positions_by_driver
            .entry(pos.driver_number)
            .or_default()
            .push(pos);
    }

    for pos_list in positions_by_driver.values_mut() {
        pos_list.sort_by_key(|p| p.date);
    }

    let mut response = Vec::new();

    for (driver, mut driver_laps) in laps_by_driver {
        let mut graph = Vec::new();

        driver_laps.sort_by_key(|l| l.date_start);

        if let Some(pos_list) = positions_by_driver.get(&driver) {
            if pos_list.is_empty() {
                continue;
            }

            let mut pos_idx = 0usize;
            let mut last_pos = pos_list[0].position;

            graph.push(LapPosition {
                lap: 1,
                position: last_pos,
            });

            for lap in driver_laps {
                let Some(ts) = lap.date_start else { continue };

                while pos_idx < pos_list.len() && pos_list[pos_idx].date <= ts {
                    last_pos = pos_list[pos_idx].position;
                    pos_idx += 1;
                }

                graph.push(LapPosition {
                    lap: lap.lap_number,
                    position: last_pos,
                });
            }
        }

        response.push(DriverLapGraph {
            driver_number: driver,
            data: graph,
        });
    }

    response.sort_by(|a, b| {
        a.data
            .last()
            .unwrap()
            .position
            .cmp(&b.data.last().unwrap().position)
    });
    state
        .get_drivers_position_telemetry_cache
        .insert(cache_key, CacheEntry::new(response.clone(), TTL_SECONDS));

    Json(response)
}

pub async fn get_sector_timings(
    State(state): State<Arc<AppState>>,
    Path(session_key): Path<String>,
) -> impl IntoResponse {
    let cache_key = format!("session_sector_timings_{}", session_key);

    if let Some(entry) = state.get_sector_timings_cache.get(&cache_key) {
        if !entry.is_expired() {
            info!("CACHE HIT for session {} for sector timings", session_key);
            return (StatusCode::OK, Json(entry.value.clone())).into_response();
        }
        info!(
            "CACHE EXPIRED for session {} for sector timings recomputing…",
            session_key
        );
        drop(entry);
        state.get_sector_timings_cache.remove(&cache_key);
    }
    info!(
        "CACHE MISS for session {} for sector timings, computing…",
        session_key
    );

    // ✅ Get top 3 drivers from session_result
    let result_url = format!(
        "https://api.openf1.org/v1/session_result?session_key={}&position<=3",
        session_key
    );

    let result_body = match state.http_client.get(&result_url).send().await {
        Ok(r) => match r.text().await {
            Ok(text) => text,
            Err(e) => {
                tracing::error!("Failed to read session_result response: {:?}", e);
                return (
                    StatusCode::BAD_GATEWAY,
                    Json(json!({ "error": "Failed to read session_result" })),
                )
                    .into_response();
            }
        },
        Err(e) => {
            tracing::error!("Failed to fetch session_result: {:?}", e);
            return (
                StatusCode::BAD_GATEWAY,
                Json(json!({ "error": "Failed to fetch session_result" })),
            )
                .into_response();
        }
    };

    // Check if we got an error response instead of an array
    let session_results: Vec<Value> = match serde_json::from_str(&result_body) {
        Ok(results) => results,
        Err(e) => {
            tracing::error!("Failed to parse session_result as array: {:?}", e);
            tracing::debug!("Response body: {}", result_body);

            // Check if it's a rate limit error
            if let Ok(error_obj) = serde_json::from_str::<Value>(&result_body) {
                if error_obj.get("error").is_some() || error_obj.get("message").is_some() {
                    tracing::warn!("OpenF1 API returned error: {:?}", error_obj);
                    return (
                        StatusCode::TOO_MANY_REQUESTS,
                        Json(json!({ "error": "OpenF1 API rate limit or error", "details": error_obj })),
                    )
                        .into_response();
                }
            }

            return (
                StatusCode::BAD_GATEWAY,
                Json(json!({ "error": "Invalid response from OpenF1 API" })),
            )
                .into_response();
        }
    };

    let mut response = Vec::new();

    // ✅ Process each top-3 driver with delays between requests
    for (idx, driver) in session_results.iter().enumerate() {
        // Add delay between requests to avoid rate limiting (except for first request)
        if idx > 0 {
            sleep(StdDuration::from_millis(300)).await; // 300ms delay between requests
        }

        let position = match driver["position"].as_u64() {
            Some(p) => p as u32,
            None => {
                tracing::warn!("Missing position for driver: {:?}", driver);
                continue;
            }
        };

        let driver_number = match driver["driver_number"].as_u64() {
            Some(d) => d as u32,
            None => {
                tracing::warn!("Missing driver_number for driver: {:?}", driver);
                continue;
            }
        };

        // ✅ Fetch all laps for this driver
        let laps_url = format!(
            "https://api.openf1.org/v1/laps?session_key={}&driver_number={}",
            session_key, driver_number
        );

        let laps_body = match state.http_client.get(&laps_url).send().await {
            Ok(r) => match r.text().await {
                Ok(text) => text,
                Err(e) => {
                    tracing::error!(
                        "Failed to read laps response for driver {}: {:?}",
                        driver_number,
                        e
                    );
                    continue;
                }
            },
            Err(e) => {
                tracing::error!("Failed to fetch laps for driver {}: {:?}", driver_number, e);
                continue;
            }
        };

        let laps: Vec<Value> = match serde_json::from_str(&laps_body) {
            Ok(laps) => laps,
            Err(e) => {
                tracing::error!("Failed to parse laps for driver {}: {:?}", driver_number, e);
                tracing::debug!("Laps response body: {}", laps_body);
                continue;
            }
        };

        let fastest_lap_data = laps
            .iter()
            .filter_map(|lap| {
                lap.get("lap_duration")
                    .and_then(|v| v.as_f64())
                    .map(|duration| (lap, duration))
            })
            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

        let (lap, fastest_lap) = match fastest_lap_data {
            Some((l, d)) => (l, d),
            None => {
                tracing::warn!("No valid lap duration found for driver {}", driver_number);
                continue;
            }
        };

        let sector_1 = lap
            .get("duration_sector_1")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);

        let sector_2 = lap
            .get("duration_sector_2")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);

        let sector_3 = lap
            .get("duration_sector_3")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);

        let entry = FastestLapSector {
            position,
            driver_number,
            fastest_lap,
            sector_1,
            sector_2,
            sector_3,
        };
        response.push(entry);
    }

    if response.is_empty() {
        tracing::warn!(
            "No sector timing data collected for session {}",
            session_key
        );
        return (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "No sector timing data available" })),
        )
            .into_response();
    }

    response.sort_by_key(|r| r.position);
    state
        .get_sector_timings_cache
        .insert(cache_key, CacheEntry::new(response.clone(), TTL_SECONDS));

    (StatusCode::OK, Json(response)).into_response()
}

async fn get_fastest_lap(
    client: &reqwest::Client,
    session: &str,
    driver: u32,
) -> Option<(String, f64)> {
    let url = format!(
        "https://api.openf1.org/v1/laps?session_key={}&driver_number={}",
        session, driver
    );

    let laps: Vec<Lap> = client.get(url).send().await.ok()?.json().await.ok()?;

    let lap = laps
        .into_iter()
        .filter(|l| l.lap_duration.is_some() && l.date_start.is_some())
        .min_by(|a, b| {
            a.lap_duration
                .unwrap()
                .partial_cmp(&b.lap_duration.unwrap())
                .unwrap()
        })?;

    Some((lap.date_start.unwrap(), lap.lap_duration.unwrap()))
}

async fn get_telemetry_with_distance(
    client: &reqwest::Client,
    session: &str,
    driver: u32,
    start: &str,
    duration: f64,
) -> Vec<(f64, f64, f64)> {
    let start_dt = chrono::DateTime::parse_from_rfc3339(start).unwrap();
    let end_dt = start_dt + chrono::Duration::milliseconds((duration * 1000.0) as i64);

    let loc_url = format!(
        "https://api.openf1.org/v1/location?session_key={}&driver_number={}&date>{}&date<{}",
        session,
        driver,
        start_dt.to_rfc3339(),
        end_dt.to_rfc3339()
    );

    let car_url = format!(
        "https://api.openf1.org/v1/car_data?session_key={}&driver_number={}&date>{}&date<{}",
        session,
        driver,
        start_dt.to_rfc3339(),
        end_dt.to_rfc3339()
    );

    let locations: Vec<LocationPoint> = client
        .get(loc_url)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let car_data: Vec<CarDataPoint> = client
        .get(car_url)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    let mut _distance = 0.0;
    let mut output = vec![];

    for i in 1..locations.len() {
        let dx = locations[i].x - locations[i - 1].x;
        let dy = locations[i].y - locations[i - 1].y;
        let dz = locations[i].z - locations[i - 1].z;
        let d = (dx * dx + dy * dy + dz * dz).sqrt();
        _distance += d;

        let speed = car_data
            .iter()
            .min_by_key(|c| {
                let t1 = chrono::DateTime::parse_from_rfc3339(&c.date)
                    .unwrap()
                    .timestamp_millis();
                let t2 = chrono::DateTime::parse_from_rfc3339(&locations[i].date)
                    .unwrap()
                    .timestamp_millis();
                (t1 - t2).abs()
            })
            .map(|x| x.speed)
            .unwrap_or(0.0);

        output.push((locations[i].x, locations[i].y, speed));
    }

    output
}

pub fn compute_minisector_pace(a: Vec<(f64, f64, f64)>, b: Vec<(f64, f64, f64)>) -> Vec<PacePoint> {
    let num_minisectors = 26; // Changed from 25 to 26
    let total_distance = a.len().max(b.len()) as f64;
    let minisector_len = total_distance / num_minisectors as f64;

    let mut results = vec![];

    for i in 0..num_minisectors {
        let start = (i as f64 * minisector_len) as usize;
        let end = ((i + 1) as f64 * minisector_len) as usize;

        let avg_a = a[start.min(a.len())..end.min(a.len())]
            .iter()
            .map(|x| x.2)
            .sum::<f64>()
            / (end - start).max(1) as f64;

        let avg_b = b[start.min(b.len())..end.min(b.len())]
            .iter()
            .map(|x| x.2)
            .sum::<f64>()
            / (end - start).max(1) as f64;

        let fastest = if avg_a > avg_b { 1 } else { 2 };

        if let Some(p) = a.get(start) {
            results.push(PacePoint {
                x: p.0,
                y: p.1,
                minisector: i as u32,
                fastest_driver: fastest,
            });
        }
    }

    results
}

pub async fn compare_race_pace(
    State(state): State<Arc<AppState>>,
    Path(session_key): Path<String>,
    Query(params): Query<PaceQuery>,
) -> Json<Vec<PacePoint>> {
    let session = session_key.clone();
    let d1 = params.driver_1;
    let d2 = params.driver_2;
    let cache_key = format!("race_pace_{}_{}_{}", session, d1, d2);

    if let Some(entry) = state.get_race_pace_cache.get(&cache_key) {
        if !entry.is_expired() {
            info!("CACHE HIT for session {} for race pace", session);
            return Json(entry.value.clone());
        }
        info!(
            "CACHE EXPIRED for session {} for race pace, recomputing…",
            session
        );
        drop(entry);
        state.get_race_pace_cache.remove(&cache_key);
    }
    info!(
        "CACHE MISS for session {} for race pace, computing…",
        session
    );

    let (s1, dur1) = get_fastest_lap(&state.http_client, &session, d1)
        .await
        .unwrap();

    sleep(TokioDuration::from_millis(300)).await; // Use tokio::time::sleep

    let (s2, dur2) = get_fastest_lap(&state.http_client, &session, d2)
        .await
        .unwrap();

    let t1 = get_telemetry_with_distance(&state.http_client, &session, d1, &s1, dur1).await;

    sleep(TokioDuration::from_millis(300)).await; // Use tokio::time::sleep

    let t2 = get_telemetry_with_distance(&state.http_client, &session, d2, &s2, dur2).await;

    let result = compute_minisector_pace(t1, t2);
    state
        .get_race_pace_cache
        .insert(cache_key, CacheEntry::new(result.clone(), TTL_SECONDS));

    Json(result)
}
