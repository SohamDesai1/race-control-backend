use crate::utils::state::AppState;
use crate::utils::race_utils::map_session_name;
use std::sync::Arc;
use chrono::Datelike;
use serde_json::Value;
use tracing::{info, error};

pub async fn initialize_database(state: &Arc<AppState>) {
    info!("Starting database initialization...");
    
    // Calculate previous year
    let year = chrono::Utc::now().year() - 1;
    let start_date = format!("{}-01-01", year);
    let end_date = format!("{}-12-31", year);
    
    // Check if we already have data for this year
    let row: (i64,) = match sqlx::query_as(
        r#"SELECT count(*) FROM "Races" WHERE season = $1"#
    )
    .bind(year.to_string())
    .fetch_one(&state.db_pool)
    .await {
        Ok(c) => c,
        Err(e) => {
            error!("Failed to check existing race data: {:?}", e);
            return;
        }
    };
    
    let count = row.0;

    if count > 0 {
        info!("Database already populated with {} races for {}. Skipping initialization.", count, year);
        return;
    }

    info!("Fetching race data for season {} from Ergast API...", year);
    
    let url = format!("https://api.jolpi.ca/ergast/f1/{}/races/?format=json", year);
    
    let res = match state.http_client.get(&url).send().await {
        Ok(r) => r,
        Err(e) => {
            error!("Failed to fetch from Ergast API: {:?}", e);
            return;
        }
    };

    let body = match res.text().await {
        Ok(t) => t,
        Err(e) => {
            error!("Failed to read Ergast response: {:?}", e);
            return;
        }
    };

    let api_data: Value = match serde_json::from_str(&body) {
        Ok(v) => v,
        Err(e) => {
            error!("Failed to parse Ergast JSON: {:?}", e);
            return;
        }
    };

    let races = match api_data["MRData"]["RaceTable"]["Races"].as_array() {
        Some(r) => r,
        None => {
            error!("No races found in Ergast response");
            return;
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

    for race_data in races {
        let round = race_data["round"].as_str().unwrap_or_default();
        let race_name = race_data["raceName"].as_str().unwrap_or_default();
        
        // 1. Insert Circuit
        let circuit = &race_data["Circuit"];
        let circuit_id = circuit["circuitId"].as_str().unwrap_or_default();
        let circuit_name = circuit["circuitName"].as_str().unwrap_or_default();
        let location = circuit["Location"]["locality"].as_str().unwrap_or_default();
        let country = circuit["Location"]["country"].as_str().unwrap_or_default();
        let lat = circuit["Location"]["lat"].as_str().unwrap_or_default();
        let long = circuit["Location"]["long"].as_str().unwrap_or_default();
        let locality = circuit["Location"]["locality"].as_str().unwrap_or_default();

        let _ = sqlx::query(
            r#"
            INSERT INTO "Circuits" ("circuitId", "circuitName", location, country, lat, long, locality)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            ON CONFLICT ("circuitId") DO NOTHING
            "#
        )
        .bind(circuit_id)
        .bind(circuit_name)
        .bind(location)
        .bind(country)
        .bind(lat)
        .bind(long)
        .bind(locality)
        .execute(&state.db_pool)
        .await;

        // 2. Insert Race
        let date = race_data["date"].as_str();
        let time = race_data["time"].as_str();
        
        let row: (i64,) = match sqlx::query_as(
            r#"
            INSERT INTO "Races" (season, round, date, time, "raceName", "circuitId")
            VALUES ($1, $2, $3::date, $4::time, $5, $6)
            RETURNING id
            "#
        )
        .bind(year.to_string())
        .bind(round)
        .bind(date)
        .bind(time)
        .bind(race_name)
        .bind(circuit_id)
        .fetch_one(&state.db_pool)
        .await {
            Ok(id) => id,
            Err(e) => {
                error!("Failed to insert race {}: {:?}", race_name, e);
                continue;
            }
        };
        
        let race_id = row.0;

        // 3. Insert Sessions
        for session_type in &session_types {
            let (s_date, s_time) = if *session_type == "Race" {
                (date, time)
            } else {
                if let Some(session) = race_data.get(*session_type) {
                    (
                        session.get("date").and_then(|v| v.as_str()),
                        session.get("time").and_then(|v| v.as_str()),
                    )
                } else {
                    (None, None)
                }
            };

            if let (Some(d), Some(t)) = (s_date, s_time) {
                let _ = sqlx::query(
                    r#"
                    INSERT INTO "Sessions" ("raceId", "sessionType", date, time)
                    VALUES ($1, $2, $3::date, $4::time)
                    "#
                )
                .bind(race_id)
                .bind(session_type)
                .bind(d)
                .bind(t)
                .execute(&state.db_pool)
                .await;
            }
        }
        
        info!("Initialized race: {}", race_name);
    }

    // --- Fetch OpenF1 Data for Session Keys ---
    info!("Fetching session keys from OpenF1 for year {}...", year);
    let openf1_url = format!("https://api.openf1.org/v1/sessions?year={}", year);

    let openf1_res = match state.http_client.get(&openf1_url).send().await {
        Ok(r) => r,
        Err(e) => {
            error!("Failed to fetch from OpenF1 API: {:?}", e);
            return;
        }
    };

    let openf1_body = match openf1_res.text().await {
        Ok(t) => t,
        Err(e) => {
            error!("Failed to read OpenF1 response: {:?}", e);
            return;
        }
    };

    let openf1_data: Value = match serde_json::from_str(&openf1_body) {
        Ok(v) => v,
        Err(e) => {
            error!("Failed to parse OpenF1 JSON: {:?}", e);
            return;
        }
    };

    if let Some(sessions) = openf1_data.as_array() {
        for session in sessions {
            let session_key = session["session_key"].as_i64();
            let meeting_key = session["meeting_key"].as_i64();
            let session_name = session["session_name"].as_str();
            let date_start = session["date_start"].as_str();

            if let (Some(sk), Some(mk), Some(name), Some(ds)) = (session_key, meeting_key, session_name, date_start) {
                if let Some(mapped_name) = map_session_name(name) {
                    // Extract date part YYYY-MM-DD
                    let date_str = ds.split('T').next().unwrap_or(ds);

                    // Update database
                    let result = sqlx::query(
                        r#"
                        UPDATE "Sessions"
                        SET "session_key" = $1, "meeting_key" = $2
                        WHERE "sessionType" = $3 AND "date" = $4::date
                        "#
                    )
                    .bind(sk as i32)
                    .bind(mk as i32)
                    .bind(mapped_name)
                    .bind(date_str)
                    .execute(&state.db_pool)
                    .await;

                    if let Err(e) = result {
                        error!("Failed to update session keys for {} on {}: {:?}", mapped_name, date_str, e);
                    }
                }
            }
        }
        info!("Session keys updated from OpenF1.");
    }

    info!("Database initialization complete.");
}
