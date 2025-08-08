use crate::utils::{race_utils::map_session_name, state::AppState};
use axum::{
    extract::{Path, State},
    response::IntoResponse,
    Json,
};
use chrono::{DateTime, Utc};
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
