use std::sync::Arc;
use std::time::Duration;

use chrono::{Datelike, Utc};
use crate::utils::{scoring, state::AppState};

const POLL_INTERVAL_IDLE: u64 = 3600; // 1 hour when no race
const POLL_INTERVAL_ACTIVE: u64 = 300 * 3; // 15 minutes during race

pub fn start_scoring_worker(state: Arc<AppState>) {
    tokio::spawn(async move {
        let mut current_interval = POLL_INTERVAL_IDLE;
        
        loop {
            tokio::time::sleep(Duration::from_secs(current_interval)).await;

            match check_and_calculate_scores(&state).await {
                Ok(has_active_race) => {
                    if has_active_race {
                        current_interval = POLL_INTERVAL_ACTIVE;
                        tracing::debug!("Active race detected, switching to 5-min polling");
                    } else {
                        current_interval = POLL_INTERVAL_IDLE;
                        tracing::debug!("No active race, switching to 1-hour polling");
                    }
                }
                Err(e) => {
                    tracing::error!("Error in scoring worker: {}", e);
                }
            }
        }
    });
}

fn is_race_weekend() -> bool {
    let today = Utc::now().date_naive();
    let weekday = today.weekday();
    
    // F1 race weekends are typically Friday-Sunday
    match weekday {
        chrono::Weekday::Fri | chrono::Weekday::Sat | chrono::Weekday::Sun => true,
        _ => false,
    }
}

async fn check_for_active_race(state: &Arc<AppState>) -> Result<Option<(i64, i64)>, String> {
    // Query our database for upcoming/ongoing races this weekend
    let races = sqlx::query_as::<_, (i64, Option<i64>, String)>(
        r#"
        SELECT r.id, s."session_key"::integer, r.date::text
        FROM "Races" r
        LEFT JOIN "Sessions" s ON r.id = s."raceId" AND s."sessionType" = 'Race'
        WHERE r.date >= CURRENT_DATE - INTERVAL '3 days'
          AND r.date <= CURRENT_DATE + INTERVAL '3 days'
        ORDER BY r.date ASC
        LIMIT 1
        "#,
    )
    .fetch_all(&state.db_pool)
    .await
    .map_err(|e| format!("Failed to fetch races: {}", e))?;

    if races.is_empty() {
        return Ok(None);
    }

    let (gp_id, session_key, _race_date) = &races[0];
    let session_key = match session_key {
        Some(key) => *key,
        None => return Ok(None),
    };

    // Check if race is still ongoing (not finished)
    let url = format!("https://api.openf1.org/v1/session?session_key={}", session_key);
    
    let response = state
        .http_client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch session: {}", e))?;

    let session_data: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse session: {}", e))?;

    let session_status = session_data
        .as_array()
        .and_then(|arr| arr.first())
        .and_then(|v| v.get("session_status"))
        .and_then(|v| v.as_str());

    // If race is finished, return None (will switch to idle polling)
    if session_status == Some("Finished") {
        return Ok(None);
    }

    // Race is in progress or upcoming
    Ok(Some((*gp_id, session_key)))
}

async fn check_and_calculate_scores(state: &Arc<AppState>) -> Result<bool, String> {
    // Option 2: Time-based - only run during race weekends (Fri-Sun)
    if !is_race_weekend() {
        tracing::debug!("Not a race weekend, skipping scoring worker check");
        return Ok(false);
    }

    tracing::debug!("Running scoring worker check (race weekend)...");

    // Lock teams if qualifying has started
    if let Err(e) = lock_teams_for_qualifying(state).await {
        tracing::warn!("Error locking teams: {}", e);
    }

    // Check for active race session
    let active_race = check_for_active_race(state).await?;
    
    if active_race.is_none() {
        tracing::debug!("No active race found during race weekend");
        return Ok(false);
    }

    let (gp_id, session_key) = active_race.unwrap();

    // Check if already scored
    let already_scored = sqlx::query_scalar::<_, (i64,)>(
        r#"
        SELECT COUNT(*) FROM "fantasy_teams" 
        WHERE gp_id = $1 AND is_locked = true AND total_points > 0
        "#,
    )
    .bind(gp_id)
    .fetch_one(&state.db_pool)
    .await
    .map_err(|e| format!("Failed to check if scored: {}", e))?;

    if already_scored.0 > 0 {
        tracing::debug!("GP {} already scored, skipping", gp_id);
        return Ok(true); // Still return true to keep active polling
    }

    // Fetch session status to check if finished
    let url = format!("https://api.openf1.org/v1/session?session_key={}", session_key);
    
    let response = state
        .http_client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch session: {}", e))?;

    let session_data: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse session: {}", e))?;

    let session_status = session_data
        .as_array()
        .and_then(|arr| arr.first())
        .and_then(|v| v.get("session_status"))
        .and_then(|v| v.as_str());

    if session_status != Some("Finished") {
        tracing::debug!("GP {} race not finished yet (status: {:?})", gp_id, session_status);
        return Ok(true); // Keep polling actively
    }

    tracing::info!("Race finished for GP {}, calculating scores...", gp_id);

    // Fetch race results
    let results_url = format!(
        "https://api.openf1.org/v1/session_results?session_key={}",
        session_key
    );

    let results_response = state
        .http_client
        .get(&results_url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch results: {}", e))?;

    let results_data: serde_json::Value = results_response
        .json()
        .await
        .map_err(|e| format!("Failed to parse results: {}", e))?;

    let race_results: Vec<scoring::RaceResult> = results_data
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|racing| {
                    let position = racing
                        .get("position")
                        .and_then(|v| v.as_i64())
                        .map(|v| v as i64)
                        .unwrap_or(0);

                    Some(scoring::RaceResult {
                        driver_id: racing
                            .get("driver_number")
                            .and_then(|v| v.as_i64())
                            .map(|v| v as i64)
                            .unwrap_or(0),
                        position,
                        fastest_lap: racing
                            .get("fastest_lap")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false),
                        dnf: racing
                            .get("dnf")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false),
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    let constructors: Vec<(i64, String)> = sqlx::query_as(
        r#"SELECT id, name FROM "fantasy_constructors" WHERE year = 2026"#
    )
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
                    let openf1_team_name = team_data.get("team_name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    
                    let position = team_data.get("position_current")
                        .and_then(|v| v.as_i64())
                        .map(|v| v as i64)
                        .unwrap_or(0);

                    for (constructor_id, fantasy_name) in &constructors {
                        if openf1_team_name.to_lowercase().contains(&fantasy_name.to_lowercase()) {
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

    scoring::calculate_gp_scores(
        state,
        gp_id,
        race_results,
        constructor_results,
        fastest_lap_driver_id,
    )
    .await?;

    tracing::info!("Successfully calculated scores for GP {}", gp_id);

    // Return false to switch back to idle polling after scoring
    Ok(false)
}

async fn lock_teams_for_qualifying(state: &Arc<AppState>) -> Result<(), String> {
    // Find GPs where qualifying has started but race hasn't finished
    let races = sqlx::query_as::<_, (i64, Option<i64>, Option<i64>)>(
        r#"
        SELECT r.id, 
               q.session_key::integer as quali_key,
               race.session_key::integer as race_key
        FROM "Races" r
        LEFT JOIN "Sessions" q ON r.id = q."raceId" AND q."sessionType" = 'Qualifying'
        LEFT JOIN "Sessions" race ON r.id = race."raceId" AND race."sessionType" = 'Race'
        WHERE r.date >= CURRENT_DATE - INTERVAL '7 days'
        "#,
    )
    .fetch_all(&state.db_pool)
    .await
    .map_err(|e| format!("Failed to fetch races: {}", e))?;

    for (gp_id, quali_key, race_key) in races {
        // Skip if no qualifying session or if race already finished
        let quali_key = match quali_key {
            Some(k) => k,
            None => continue,
        };

        // Check if race already finished (teams should already be locked)
        if let Some(race_key) = race_key {
            let race_url = format!("https://api.openf1.org/v1/session?session_key={}", race_key);
            if let Ok(resp) = state.http_client.get(&race_url).send().await {
                if let Ok(data) = resp.json::<serde_json::Value>().await {
                    if data.as_array()
                        .and_then(|arr| arr.first())
                        .and_then(|v| v.get("session_status"))
                        .and_then(|v| v.as_str()) == Some("Finished") {
                        continue;
                    }
                }
            }
        }

        // Check if qualifying has started (not finished)
        let quali_url = format!("https://api.openf1.org/v1/session?session_key={}", quali_key);
        
        let response = match state.http_client.get(&quali_url).send().await {
            Ok(resp) => resp,
            Err(e) => {
                tracing::warn!("Failed to check qualifying status for GP {}: {}", gp_id, e);
                continue;
            }
        };

        let quali_data: serde_json::Value = match response.json().await {
            Ok(data) => data,
            Err(e) => {
                tracing::warn!("Failed to parse qualifying data for GP {}: {}", gp_id, e);
                continue;
            }
        };

        let quali_status = quali_data
            .as_array()
            .and_then(|arr| arr.first())
            .and_then(|v| v.get("session_status"))
            .and_then(|v| v.as_str());

        // Lock teams if qualifying has started (any status other than Completed)
        // Teams should lock when qualifying begins, unlock after race
        if quali_status.is_some() && quali_status != Some("Completed") {
            // Check if teams already locked
            let already_locked = sqlx::query_scalar::<_, (i64,)>(
                r#"SELECT COUNT(*) FROM "fantasy_teams" WHERE gp_id = $1 AND is_locked = true"#
            )
            .bind(gp_id)
            .fetch_one(&state.db_pool)
            .await
            .map_err(|e| format!("Failed to check lock status: {}", e))?;

            if already_locked.0 == 0 {
                // Lock all teams for this GP
                let result = sqlx::query(
                    r#"UPDATE "fantasy_teams" SET is_locked = true WHERE gp_id = $1 AND is_locked = false"#
                )
                .bind(gp_id)
                .execute(&state.db_pool)
                .await;

                match result {
                    Ok(_) => tracing::info!("Locked teams for GP {} (qualifying started)", gp_id),
                    Err(e) => tracing::error!("Failed to lock teams for GP {}: {}", gp_id, e),
                }
            }
        }
    }

    Ok(())
}
