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

pub async fn get_race_results(
    State(state): State<AppState>,
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

pub async fn get_race_data_db(State(state): State<AppState>) -> impl IntoResponse {
    let res = state
        .supabase
        .from("Races")
        .select("*")
        .order("date.asc")
        .execute()
        .await;
    // }
    match res {
        Ok(result) => {
            let body = result.text().await.unwrap();
            let res_body: Value = from_str(&body).unwrap();
            return (StatusCode::OK, Json(res_body.clone())).into_response();
        }
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to fetch races".to_string(),
            )
                .into_response()
        }
    }
}

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

                                println!(
                                    "Update result for {}: {:?}",
                                    mapped_name,
                                    update_res.as_ref().map(|r| r.status())
                                );

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

pub async fn get_latest_session_data(
    State(state): State<AppState>,
    session_key: Option<Path<String>>,
) -> impl IntoResponse {
    let session_key = session_key
        .map(|Path(sk)| sk)
        .unwrap_or_else(|| "latest".to_string());
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
        // Extract driver_number as string key
        let driver_number = lap["driver_number"].to_string();
        // Strip quotes around date_start
        // Handle null or missing date_start
        let Some(date_str) = lap["date_start"].as_str() else {
            continue; // Skip this lap if date_start is missing or null
        };

        let Ok(date) = DateTime::parse_from_rfc3339(date_str) else {
            continue; // Skip if date format is invalid
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

pub async fn _get_session_data(State(state): State<AppState>) {
    let res = state
        .http_client
        .get("https://api.openf1.org/v1/sessions?session_key=latest")
        .send()
        .await
        .unwrap();

    let body = res.text().await.unwrap();
    let _res: Value = from_str(&body).unwrap();
}
// pub async fn insert_race_and_circuit(State(state): State<AppState>) -> impl IntoResponse {
//     let client = reqwest::Client::new();
//     let res = client
//         .get("https://api.jolpi.ca/ergast/f1/2025/races/?format=json")
//         .send()
//         .await
//         .unwrap();

//     let body = res.text().await.unwrap();
//     let res: Value = from_str(&body).unwrap();
//     let api_res_body = &res["MRData"]["RaceTable"]["Races"];
//     let res = state
//         .supabase
//         .from("Races")
//         .select("*")
//         .execute()
//         .await
//         .unwrap();
//     let res_body: Value = from_str(&res.text().await.unwrap()).unwrap();
//     // let races = res_body.get("data").and_then(|d| d.as_array()).unwrap();
//     let session_types = [
//         "FirstPractice",
//         "SecondPractice",
//         "ThirdPractice",
//         "SprintQualifying",
//         "Sprint",
//         "Qualifying",
//         "Race",
//     ];
//     for api_race in api_res_body.as_array().unwrap() {
//         let round = api_race["round"].as_str().unwrap_or_default();

//         let db_race = res_body
//             .as_array()
//             .unwrap()
//             .iter()
//             .find(|r| r["round"].as_str() == Some(round));
//         println!("db");
//         if let Some(race) = db_race {
//             println!("if let");
//             let race_id = race["id"].as_i64().unwrap(); // Supabase ID

//             for session_type in &session_types {
//                 println!("session_type: {}", session_type);
//                 if let Some(session) = api_race.get(*session_type) {
//                     if let (Some(date), Some(time)) = (session.get("date"), session.get("time")) {
//                         let body = json!({
//                             "raceId": race_id,
//                             "sessionType": session_type,
//                             "date": date,
//                             "time": time,
//                         });

//                         let result = state
//                             .supabase
//                             .from("Sessions")
//                             .insert(body.to_string())
//                             .execute()
//                             .await;

//                         match result {
//                             Ok(_) => {
//                                 println!("✅ Inserted {:?}", result.unwrap().text().await.unwrap())
//                             }
//                             Err(e) => eprintln!(
//                                 "❌ Error inserting {session_type} for race {race_id}: {e:?}"
//                             ),
//                         }
//                     }
//                 }
//             }
//         }
//     }
//     (StatusCode::OK, Json(res_body.clone())).into_response()
// }
