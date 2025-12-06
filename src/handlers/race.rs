use std::sync::Arc;

use crate::utils::state::AppState;
use axum::{
    extract::{Path, State},
    response::IntoResponse,
    Json,
};
use chrono::Utc;
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
pub async fn get_upcoming_race_data(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let today = Utc::now().date_naive().format("%Y-%m-%d").to_string();

    // Fetch all upcoming races
    let res = state
        .supabase
        .from("Races")
        .select("*")
        .gte("date", &today)
        .order("date.asc")
        .execute()
        .await;

    match res {
        Ok(result) => {
            let body = result.text().await.unwrap();
            let races: Value = from_str(&body).unwrap();

            if let Some(race_array) = races.as_array() {
                if race_array.is_empty() {
                    return (StatusCode::NOT_FOUND, Json(json!([]))).into_response();
                }

                let mut races_with_circuits = Vec::new();

                for race in race_array {
                    let mut race_data = race.as_object().cloned().unwrap_or_default();

                    if let Some(circuit_id) = race.get("circuitId").and_then(|v| v.as_str()) {
                        let circuit_res = state
                            .supabase
                            .from("Circuits")
                            .select("*")
                            .eq("circuitId", circuit_id)
                            .execute()
                            .await;

                        if let Ok(circuit_result) = circuit_res {
                            if let Ok(circuit_body) = circuit_result.text().await {
                                if let Ok(circuit_data) = from_str::<Value>(&circuit_body) {
                                    if let Some(circuit_array) = circuit_data.as_array() {
                                        if let Some(circuit) = circuit_array.get(0) {
                                            if let Some(circuit_obj) = circuit.as_object() {
                                                for (key, value) in circuit_obj {
                                                    race_data.insert(key.clone(), value.clone());
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    races_with_circuits.push(json!(race_data));
                }

                return (StatusCode::OK, Json(races_with_circuits)).into_response();
            }

            (StatusCode::OK, Json(json!([]))).into_response()
        }
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Failed to fetch races").into_response(),
    }
}

// pub async fn insert_race_and_circuit(State(state): State<Arc<AppState>>) -> impl IntoResponse {
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
