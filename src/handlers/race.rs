use std::sync::Arc;

use crate::utils::state::AppState;
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

pub async fn get_all_races_data_db(
    State(state): State<Arc<AppState>>,
    Path(year): Path<String>,
) -> impl IntoResponse {
    let res = state
        .supabase
        .from("Races")
        .eq("season", &year)
        .select("*")
        .order("date.asc")
        .execute()
        .await;

    match res {
        Ok(result) => {
            let body = result.text().await.unwrap();
            let res_body: Value = from_str(&body).unwrap();

            // Check if races array is empty
            if res_body.as_array().map_or(true, |arr| arr.is_empty()) {
                return (
                    StatusCode::NOT_FOUND,
                    Json(json!({"error": "No races found for this year"})),
                )
                    .into_response();
            }

            let circuits = state.supabase.from("Circuits").select("*").execute().await;

            match circuits {
                Ok(circuits_result) => {
                    let circuits_body = circuits_result.text().await.unwrap();
                    let circuits_res_body: Value = from_str(&circuits_body).unwrap();

                    // Combine race data with circuit data
                    let mut races_with_circuits = Vec::new();

                    for race in res_body.as_array().unwrap() {
                        let circuit_id = race["circuitId"].as_str().unwrap_or_default();

                        let circuit_data = circuits_res_body
                            .as_array()
                            .unwrap()
                            .iter()
                            .find(|c| c["circuitId"].as_str() == Some(circuit_id));

                        let mut race_with_circuit = race.clone();

                        if let Some(circuit) = circuit_data {
                            // Add circuit fields to the race object
                            if let Some(race_obj) = race_with_circuit.as_object_mut() {
                                race_obj
                                    .insert("locality".to_string(), circuit["locality"].clone());
                                race_obj.insert("country".to_string(), circuit["country"].clone());
                                race_obj.insert(
                                    "circuitName".to_string(),
                                    circuit["circuitName"].clone(),
                                );
                                race_obj.insert("lat".to_string(), circuit["lat"].clone());
                                race_obj.insert("long".to_string(), circuit["long"].clone());
                            }
                        }

                        races_with_circuits.push(race_with_circuit);
                    }

                    return (StatusCode::OK, Json(races_with_circuits)).into_response();
                }
                Err(err) => {
                    eprintln!("Failed to fetch circuits: {:?}", err);
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!({"error": "Failed to fetch circuits"})),
                    )
                        .into_response();
                }
            }
        }
        Err(err) => {
            eprintln!("Failed to fetch races: {:?}", err);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to fetch races"})),
            )
                .into_response();
        }
    }
}

pub async fn get_race_data(
    State(state): State<Arc<AppState>>,
    Path((year, round)): Path<(String, String)>,
) -> impl IntoResponse {
    let race = state
        .supabase
        .from("Races")
        .eq("season", &year)
        .eq("round", &round)
        .select("*")
        .execute()
        .await;
    // }
    match race {
        Ok(result) => {
            let body = result.text().await.unwrap();
            let result_res_body: Value = from_str(&body).unwrap();

            let circuits = state
                .supabase
                .from("Circuits")
                .eq(
                    "circuitId",
                    result_res_body[0]["circuitId"].as_str().unwrap_or_default(),
                )
                .select("*")
                .execute()
                .await;
            match circuits {
                Ok(circuits_result) => {
                    let circuits_body = circuits_result.text().await.unwrap();
                    let circuits_res_body: Value = from_str(&circuits_body).unwrap();

                    let race_id = result_res_body[0]["id"]
                        .as_i64()
                        .or_else(|| result_res_body[0]["id"].as_u64().map(|v| v as i64))
                        .unwrap_or(0);

                    let sessions = state
                        .supabase
                        .from("Sessions")
                        .eq("raceId", &race_id.to_string())
                        .select("*")
                        .execute()
                        .await;

                    match sessions {
                        Ok(sessions_result) => {
                            let sessions_body = sessions_result.text().await.unwrap();
                            let sessions_res_body: Value = from_str(&sessions_body).unwrap();

                            return (StatusCode::OK, Json(json!({"race":result_res_body, "circuit":circuits_res_body, "sessions":sessions_res_body}))).into_response();
                        }
                        Err(_) => {
                            return (
                                StatusCode::INTERNAL_SERVER_ERROR,
                                "Failed to fetch sessions".to_string(),
                            )
                                .into_response()
                        }
                    }
                }
                Err(_) => {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Failed to fetch circuits".to_string(),
                    )
                        .into_response()
                }
            }
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

pub async fn get_upcoming_race_data(
    State(state): State<Arc<AppState>>,
    Path(date): Path<String>,
) -> impl IntoResponse {
    let res = state
        .supabase
        .from("Races")
        .gte("date", &date)
        .select("*")
        .order("date.asc")
        .execute()
        .await;

    match res {
        Ok(result) => {
            let body = result.text().await.unwrap();
            let res_body: Value = from_str(&body).unwrap();

            // Check if races array is empty
            if res_body.as_array().map_or(true, |arr| arr.is_empty()) {
                return (
                    StatusCode::NOT_FOUND,
                    Json(json!({"error": "No races found for this year"})),
                )
                    .into_response();
            }

            let circuits = state.supabase.from("Circuits").select("*").execute().await;

            match circuits {
                Ok(circuits_result) => {
                    let circuits_body = circuits_result.text().await.unwrap();
                    let circuits_res_body: Value = from_str(&circuits_body).unwrap();

                    // Combine race data with circuit data
                    let mut races_with_circuits = Vec::new();

                    for race in res_body.as_array().unwrap() {
                        let circuit_id = race["circuitId"].as_str().unwrap_or_default();

                        let circuit_data = circuits_res_body
                            .as_array()
                            .unwrap()
                            .iter()
                            .find(|c| c["circuitId"].as_str() == Some(circuit_id));

                        let mut race_with_circuit = race.clone();

                        if let Some(circuit) = circuit_data {
                            // Add circuit fields to the race object
                            if let Some(race_obj) = race_with_circuit.as_object_mut() {
                                race_obj
                                    .insert("locality".to_string(), circuit["locality"].clone());
                                race_obj.insert("country".to_string(), circuit["country"].clone());
                                race_obj.insert(
                                    "circuitName".to_string(),
                                    circuit["circuitName"].clone(),
                                );
                                race_obj.insert("lat".to_string(), circuit["lat"].clone());
                                race_obj.insert("long".to_string(), circuit["long"].clone());
                            }
                        }

                        races_with_circuits.push(race_with_circuit);
                    }

                    return (StatusCode::OK, Json(races_with_circuits)).into_response();
                }
                Err(err) => {
                    eprintln!("Failed to fetch circuits: {:?}", err);
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!({"error": "Failed to fetch circuits"})),
                    )
                        .into_response();
                }
            }
        }
        Err(err) => {
            eprintln!("Failed to fetch races: {:?}", err);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to fetch races"})),
            )
                .into_response();
        }
    }
}

pub async fn _insert_race_and_circuit(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let year = chrono::Utc::now().year().to_string();
    let start = format!("{}-01-01", year);
    let end = format!("{}-12-31", year);
    println!("start: {}, end: {}", start, end);
    let client = reqwest::Client::new();
    let res = client
        .get(format!(
            "https://api.jolpi.ca/ergast/f1/{}/races/?format=json",
            year
        ))
        .send()
        .await
        .unwrap();

    let body = res.text().await.unwrap();
    let res: Value = from_str(&body).unwrap();
    let api_res_body = &res["MRData"]["RaceTable"]["Races"];
    let res = state
        .supabase
        .from("Races")
        .gte("date", start)
        .lte("date", end)
        .select("*")
        .execute()
        .await
        .unwrap();
    let res_body: Value = from_str(&res.text().await.unwrap()).unwrap();
    let session_types = [
        "FirstPractice",
        "SecondPractice",
        "ThirdPractice",
        "SprintQualifying",
        "Sprint",
        "Qualifying",
        "Race",
    ];
    for api_race in api_res_body.as_array().unwrap() {
        let round = api_race["round"].as_str().unwrap_or_default();

        let db_race = res_body
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["round"].as_str() == Some(round));
        println!("db");
        if let Some(race) = db_race {
            println!("if let");
            let race_id = race["id"].as_i64().unwrap(); // Supabase ID

            for session_type in &session_types {
                println!("session_type: {}", session_type);

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
                    let body = json!({
                        "raceId": race_id,
                        "sessionType": session_type,
                        "date": date,
                        "time": time,
                        "country": api_race["Circuit"]["Location"]["country"],
                    });

                    let result = state
                        .supabase
                        .from("SessionsTest")
                        .insert(body.to_string())
                        .execute()
                        .await;

                    match result {
                        Ok(_) => {
                            println!("✅ Inserted {:?}", result.unwrap().text().await.unwrap())
                        }
                        Err(e) => {
                            eprintln!("❌ Error inserting {session_type} for race {race_id}: {e:?}")
                        }
                    }
                }
            }
        }
    }
    (StatusCode::OK, Json(res_body.clone())).into_response()
}
