use crate::{
    models::{
        cache::CacheEntry,
        telemetry::{
            CarDataPoint, DriverLapGraph, FastestLapSector, Lap, LapPosition, LapRecord,
            LocationPoint, PacePoint, PaceQuery, PositionRecord, SpeedDistance, TelemetryQuery,
        },
    },
    utils::{race_utils::map_session_name, state::AppState},
};
use axum::{
    extract::{Path, Query, State},
    response::IntoResponse,
    Json,
};
use chrono::{DateTime, Datelike, Duration, Utc};
use http::StatusCode;
use serde_json::{from_str, json, Value};
use std::{collections::HashMap, sync::Arc};
use tracing::info;


pub async fn get_sessions(
    State(state): State<Arc<AppState>>,
    Path((race_id, year)): Path<(String, Option<String>)>,
) -> impl IntoResponse {
    let year = year
        .clone()
        .unwrap_or_else(|| chrono::Utc::now().year().to_string());
    let start = format!("{}-01-01", year);
    let end = format!("{}-12-31", year);

    // Fetch sessions from database
    let res = state
        .supabase
        .from("Sessions")
        .gte("date", &start)
        .lte("date", &end)
        .eq("raceId", &race_id)
        .select("*")
        .order("id.asc")
        .execute()
        .await;

    match res {
        Ok(result) => {
            let body = result.text().await.unwrap();
            let res_body: Value = from_str(&body).unwrap();
            let sessions_array = res_body.as_array().unwrap();

            if sessions_array.is_empty() {
                return (
                    StatusCode::NOT_FOUND,
                    Json(json!({ "error": "No sessions found for this race" })),
                )
                    .into_response();
            }

            // Check which sessions are missing session_key
            let sessions_without_keys: Vec<&Value> = sessions_array
                .iter()
                .filter(|session| {
                    session.get("session_key").is_none()
                        || session.get("session_key").unwrap().is_null()
                })
                .collect();

            // If all sessions have keys, return immediately
            if sessions_without_keys.is_empty() {
                return (
                    StatusCode::OK,
                    Json(json!({"sessions": res_body,"status": "completed"})),
                )
                    .into_response();
            }

            // If some sessions are missing keys, try to fetch from OpenF1
            info!(
                "Found {} sessions without keys, attempting to fetch from OpenF1",
                sessions_without_keys.len()
            );

            // Get the date from the first session for date range query
            let start_date = sessions_array[0]["date"].as_str().unwrap_or("");
            let end_date = sessions_array
                .last()
                .and_then(|s| s["date"].as_str())
                .unwrap_or("");
            if start_date.is_empty() {
                return (
                    StatusCode::OK,
                    Json(json!({
                        "sessions": res_body,
                        "status": "scheduled",
                        "message": "Future Event, data not yet available"
                    })),
                )
                    .into_response();
            }

            // Create date range query
            let fallback_url = format!(
                "https://api.openf1.org/v1/sessions?year={}&date_start>={}&date_end<={}",
                year, start_date, end_date
            );

            info!("Fetching from OpenF1: {}", fallback_url);

            let fallback_res = state.http_client.get(&fallback_url).send().await;

            match fallback_res {
                Ok(response) => {
                    let fallback_body = response.text().await.unwrap();
                    let fallback_sessions: Vec<Value> =
                        from_str(&fallback_body).unwrap_or_default();

                    if fallback_sessions.is_empty() {
                        info!("No data available in OpenF1 yet, returning current sessions");
                        return (
                            StatusCode::OK,
                            Json(json!({
                                "sessions": res_body,
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
                                let needs_update = sessions_without_keys.iter().any(|db_session| {
                                    db_session
                                        .get("sessionType")
                                        .and_then(|v| v.as_str())
                                        .map(|t| t == mapped_name)
                                        .unwrap_or(false)
                                });

                                if needs_update {
                                    if let (Some(session_key), Some(meeting_key)) = (
                                        session.get("session_key").and_then(|v| v.as_i64()),
                                        session.get("meeting_key").and_then(|v| v.as_i64()),
                                    ) {
                                        let update_payload = json!({
                                            "session_key": session_key,
                                            "meeting_key": meeting_key
                                        });

                                        info!(
                                            "Updating {} with payload: {:?}",
                                            mapped_name, update_payload
                                        );

                                        let update_res = state
                                            .supabase
                                            .from("Sessions")
                                            .eq("sessionType", mapped_name.to_string())
                                            .eq("raceId", race_id.clone())
                                            .update(&update_payload.to_string())
                                            .execute()
                                            .await;

                                        if let Err(err) = update_res {
                                            eprintln!(
                                                "❌ Failed to update {} in Supabase: {:?}",
                                                mapped_name, err
                                            );
                                            // Don't return error, just log and continue
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
                            let updated_array = updated_sessions_data.as_array().unwrap();

                            // Check if all sessions now have keys
                            let all_complete = updated_array.iter().all(|session| {
                                session.get("session_key").is_some()
                                    && !session.get("session_key").unwrap().is_null()
                            });

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
                            eprintln!("Failed to fetch updated sessions: {:?}", err);
                            // Return original data if refetch fails
                            return (
                                StatusCode::OK,
                                Json(json!({
                                    "sessions": res_body,
                                    "status": "partial",
                                    "message": "Some sessions updated but failed to refetch"
                                })),
                            )
                                .into_response();
                        }
                    }
                }
                Err(err) => {
                    eprintln!("OpenF1 API request failed: {:?}", err);
                    // Return whatever we have from DB
                    return (
                        StatusCode::OK,
                        Json(json!({
                            "sessions": res_body,
                            "status": "partial",
                            "message": "Some sessions may be completed, OpenF1 API unavailable"
                        })),
                    )
                        .into_response();
                }
            }
        }
        Err(err) => {
            eprintln!("Database query failed: {:?}", err);
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

const TTL_SECONDS: i64 = 60 * 60;

pub async fn fetch_driver_telemetry(
    State(state): State<Arc<AppState>>,
    Query(params): Query<TelemetryQuery>,
) -> impl IntoResponse {
    let session_key = params.session_key.clone();
    let driver_number = params.driver_number.clone();
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
        state.get_sector_timings_cache.remove(&cache_key);
    }
    info!(
        "CACHE MISS for session {} for sector timings, computing…",
        session_key
    );
    //  Get fastest lap from session_result
    let result_url = format!(
        "https://api.openf1.org/v1/session_result?session_key={}&position<=3",
        session_key
    );

    let result_body = match state.http_client.get(result_url).send().await {
        Ok(r) => r.text().await.unwrap(),
        Err(_) => {
            return (
                StatusCode::BAD_GATEWAY,
                Json(json!({ "error": "Failed to fetch session_result" })),
            )
                .into_response();
        }
    };

    let session_results: Vec<Value> = serde_json::from_str(&result_body).unwrap_or_default();
    let mut response = Vec::new();

    // ✅ STEP 2: Process each top-3 driver
    for driver in session_results {
        let position = driver["position"].as_u64().unwrap() as u32;
        let driver_number = driver["driver_number"].as_u64().unwrap() as u32;

        // ✅ Fastest lap = last value from duration array
        let fastest_lap = match driver["duration"]
            .as_array()
            .and_then(|d| d.last())
            .and_then(|v| v.as_f64())
        {
            Some(v) => v,
            None => continue,
        };

        // ✅ STEP 3: Fetch laps for that driver
        let laps_url = format!(
            "https://api.openf1.org/v1/laps?session_key={}&driver_number={}",
            session_key, driver_number
        );

        let laps_body = match state.http_client.get(laps_url).send().await {
            Ok(r) => r.text().await.unwrap(),
            Err(_) => continue,
        };

        let laps: Vec<Value> = serde_json::from_str(&laps_body).unwrap_or_default();

        // ✅ STEP 4: Match fastest lap from laps API
        let matching_lap = laps.iter().find(|lap| {
            lap.get("lap_duration")
                .and_then(|v| v.as_f64())
                .map(|d| (d - fastest_lap).abs() < 0.001)
                .unwrap_or(false)
        });

        let lap = match matching_lap {
            Some(l) => l,
            None => continue,
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

        response.push(FastestLapSector {
            position,
            driver_number,
            fastest_lap,
            sector_1,
            sector_2,
            sector_3,
        });
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
    let num_minisectors = 25;
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
    Query(params): Query<PaceQuery>,
) -> Json<Vec<PacePoint>> {
    let session = params.session_key.clone();
    let d1 = params.driver_1;
    let d2 = params.driver_2;
    let cache_key = format!("race_pace_{}_{}_{}", session, d1, d2);
    if let Some(entry) = state.get_race_pace_cache.get(&cache_key) {
        if !entry.is_expired() {
            info!("CACHE HIT for session {} for sector timings", session);
            return Json(entry.value.clone());
        }
        info!(
            "CACHE EXPIRED for session {} for sector timings recomputing…",
            session
        );
        state.get_sector_timings_cache.remove(&cache_key);
    }
    info!(
        "CACHE MISS for session {} for sector timings, computing…",
        session
    );
    let (s1, dur1) = get_fastest_lap(&state.http_client, &session, d1)
        .await
        .unwrap();
    let (s2, dur2) = get_fastest_lap(&state.http_client, &session, d2)
        .await
        .unwrap();

    let t1 = get_telemetry_with_distance(&state.http_client, &session, d1, &s1, dur1).await;
    let t2 = get_telemetry_with_distance(&state.http_client, &session, d2, &s2, dur2).await;

    let result = compute_minisector_pace(t1, t2);
    state
        .get_race_pace_cache
        .insert(cache_key, CacheEntry::new(result.clone(), TTL_SECONDS));

    Json(result)
}
