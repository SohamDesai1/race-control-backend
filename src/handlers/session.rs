use crate::{
    models::telemetry::{CarDataPoint, SpeedDistance, TelemetryQuery},
    utils::{race_utils::map_session_name, state::AppState},
};
use axum::{
    extract::{Path, Query, State},
    response::IntoResponse,
    Json,
};
use chrono::{DateTime, Duration, Utc};
use http::StatusCode;
use serde_json::{from_str, json, Value};
use std::collections::HashMap;

pub async fn get_sessions(
    State(state): State<AppState>,
    Path(race_id): Path<String>,
) -> impl IntoResponse {
    let res = state
        .supabase
        .from("Sessions")
        .select("*")
        .eq("raceId", race_id.clone())
        .execute()
        .await;

    match res {
        Ok(result) => {
            let body = result.text().await.unwrap();
            let res_body: Value = from_str(&body).unwrap();
            println!("RESS {:?}", res_body);
            let sessions_array = res_body.as_array().unwrap();
            let all_have_session_keys = sessions_array.iter().all(|session| {
                session.get("session_key").is_some()
                    && !session.get("session_key").unwrap().is_null()
            });

            if all_have_session_keys && !sessions_array.is_empty() {
                return (StatusCode::OK, Json(json!({"sessions":res_body}))).into_response();
            } else {
                println!("fallback");
                let fallback_res = state
                    .http_client
                    .get(format!(
                        "https://api.openf1.org/v1/sessions?country_name={}&year=2025",
                        res_body[0]["country"].as_str().unwrap()
                    ))
                    .send()
                    .await
                    .unwrap();
                let fallback_body = fallback_res.text().await.unwrap();
                let fallback_sessions: Vec<Value> = from_str(&fallback_body).unwrap();

                // Update all matching sessions
                let mut updated_sessions = Vec::new();
                let mut update_count = 0;

                for session in fallback_sessions {
                    if let Some(ext_name) = session.get("session_name").and_then(|v| v.as_str()) {
                        if let Some(mapped_name) = map_session_name(ext_name) {
                            if let (Some(session_key), Some(meeting_key)) = (
                                session.get("session_key").and_then(|v| v.as_i64()),
                                session.get("meeting_key").and_then(|v| v.as_i64()),
                            ) {
                                // Create proper update payload with actual values
                                let update_payload = json!({
                                    "session_key": session_key,
                                    "meeting_key": meeting_key
                                });

                                println!(
                                    "Updating {} with payload: {:?}",
                                    mapped_name, update_payload
                                );

                                let update_res = state
                                    .supabase
                                    .from("Sessions")
                                    .eq("sessionType", mapped_name.to_string())
                                    .eq("raceId", race_id.clone()) // Also filter by race_id
                                    .update(&update_payload.to_string()) // Use actual payload
                                    .execute()
                                    .await;

                                if let Err(err) = update_res {
                                    eprintln!(
                                        "❌ Failed to update {} in Supabase: {:?}",
                                        mapped_name, err
                                    );
                                    return (
                                        StatusCode::INTERNAL_SERVER_ERROR,
                                        Json(json!({ "error": format!("Database update failed for {}", mapped_name) })),
                                    )
                                    .into_response();
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

                if update_count > 0 {
                    // Fetch updated data from database
                    let updated_res = state
                        .supabase
                        .from("Sessions")
                        .select("*")
                        .eq("raceId", race_id.clone())
                        .execute()
                        .await;

                    match updated_res {
                        Ok(result) => {
                            let updated_body = result.text().await.unwrap();
                            let updated_sessions_data: Value = from_str(&updated_body).unwrap();

                            let response = json!({
                                "sessions": updated_sessions_data,
                                "updated_count": update_count,
                                "updated_sessions": updated_sessions
                            });

                            return (StatusCode::CREATED, Json(response)).into_response();
                        }
                        Err(err) => {
                            eprintln!("Failed to fetch updated sessions: {:?}", err);
                            let response = json!({
                                "message": format!("Updated {} sessions successfully", update_count),
                                "updated_sessions": updated_sessions
                            });
                            return (StatusCode::CREATED, Json(response)).into_response();
                        }
                    }
                }

                // If no matching session found
                return (
                    StatusCode::NOT_FOUND,
                    Json(json!({ "error": "No matching session found" })),
                )
                    .into_response();
            }
        }
        Err(err) => {
            eprintln!("Database query failed: {:?}", err);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "Failed to fetch session" })),
            )
                .into_response();
        }
    }
}

pub async fn get_session_data(
    State(state): State<AppState>,
    Path(session_key): Path<String>,
) -> impl IntoResponse {
    let mut latest_laps: HashMap<String, Value> = HashMap::new();

    let res = state
        .http_client
        .get(format!(
            "https://api.openf1.org/v1/laps?session_key={session_key}"
        ))
        .send()
        .await
        .unwrap();

    let body = res.text().await.unwrap();
    let res: Value = from_str(&body).unwrap();

    for lap in res.as_array().unwrap() {
        let driver_number = lap["driver_number"].to_string();

        let Some(date_str) = lap["date_start"].as_str() else {
            continue;
        };

        let Ok(date) = DateTime::parse_from_rfc3339(date_str) else {
            continue;
        };

        let date = date.with_timezone(&Utc);
        latest_laps
            .entry(driver_number)
            .and_modify(|existing| {
                let existing_date_str = existing["date_start"].as_str().unwrap();
                let existing_date = DateTime::parse_from_rfc3339(existing_date_str)
                    .unwrap()
                    .with_timezone(&Utc);

                if date > existing_date {
                    *existing = lap.clone();
                }
            })
            .or_insert(lap.clone());
    }
    let res: Vec<Value> = latest_laps.into_values().collect();
    return (StatusCode::OK, Json(res)).into_response();
}

// Helper to parse RFC3339 date string to chrono::DateTime<Utc>
fn _parse_date(date: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(date)
        .ok()
        .map(|dt| dt.with_timezone(&Utc))
}

pub async fn fetch_telemetry(
    State(state): State<AppState>,
    Query(params): Query<TelemetryQuery>,
) -> impl IntoResponse {
    let session_key = params.session_key.clone();
    let driver_number = params.driver_number.clone();
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
    (StatusCode::OK, Json(result)).into_response()
}
