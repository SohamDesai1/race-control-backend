use std::sync::Arc;

use crate::utils::state::AppState;
use serde::{Deserialize, Serialize};

const DRIVER_POINTS: [i64; 10] = [25, 18, 15, 12, 10, 8, 6, 4, 2, 1];
const CONSTRUCTOR_POINTS: [i64; 10] = [25, 18, 15, 12, 10, 8, 6, 4, 2, 1];
const FASTEST_LAP_BONUS: i64 = 5;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaceResult {
    pub driver_id: i64,
    pub position: i64,
    pub fastest_lap: bool,
    pub dnf: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstructorResult {
    pub constructor_id: i64,
    pub position: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamScore {
    pub team_id: i64,
    pub driver_1_points: i64,
    pub driver_2_points: i64,
    pub constructor_points: i64,
    pub fastest_lap_bonus: i64,
    pub booster_points: i64,
    pub total_points: i64,
}

fn get_driver_finishing_points(position: i64) -> i64 {
    if position < 1 {
        return -10;
    }
    let idx = (position - 1) as usize;
    if idx < DRIVER_POINTS.len() {
        DRIVER_POINTS[idx]
    } else {
        -(position - 10)
    }
}
fn get_constructor_finishing_points(position: i64) -> i64 {
    if position < 1 {
        return -10;
    }
    let idx = (position - 1) as usize;
    if idx < CONSTRUCTOR_POINTS.len() {
        CONSTRUCTOR_POINTS[idx]
    } else {
        -(position - 10)
    }
}

pub fn calculate_team_score(
    driver_1_position: i64,
    driver_2_position: i64,
    constructor_position: i64,
    driver_1_fastest_lap: bool,
    driver_2_fastest_lap: bool,
    booster_driver_id: Option<i64>,
    is_driver_1: bool,
    driver_1_is_dnf: bool,
    driver_2_is_dnf: bool,
) -> TeamScore {
    let driver_1_finishing_points = if !driver_1_dnf(driver_1_position, driver_1_is_dnf) {
        get_driver_finishing_points(driver_1_position)
    } else {
        0
    };

    let driver_2_finishing_points = if !driver_2_dnf(driver_2_position, driver_2_is_dnf) {
        get_driver_finishing_points(driver_2_position)
    } else {
        0
    };

    let constructor_finishing_points = get_constructor_finishing_points(constructor_position);

    let fastest_lap_bonus = if driver_1_fastest_lap || driver_2_fastest_lap {
        FASTEST_LAP_BONUS
    } else {
        0
    };

    let mut booster_points = 0;
    if let Some(booster) = booster_driver_id {
        if booster == 1 && is_driver_1 {
            booster_points = driver_1_finishing_points;
        } else if booster == 2 && !is_driver_1 {
            booster_points = driver_2_finishing_points;
        }
    }

    let total_points = driver_1_finishing_points
        + driver_2_finishing_points
        + constructor_finishing_points
        + fastest_lap_bonus
        + booster_points;

    TeamScore {
        team_id: 0,
        driver_1_points: driver_1_finishing_points,
        driver_2_points: driver_2_finishing_points,
        constructor_points: constructor_finishing_points,
        fastest_lap_bonus,
        booster_points,
        total_points,
    }
}

fn driver_1_dnf(position: i64, dnf: bool) -> bool {
    dnf || position < 1
}

fn driver_2_dnf(position: i64, dnf: bool) -> bool {
    dnf || position < 1
}

pub async fn calculate_gp_scores(
    state: &Arc<AppState>,
    gp_id: i64,
    race_results: Vec<RaceResult>,
    constructor_results: Vec<ConstructorResult>,
    fastest_lap_driver_id: Option<i64>,
) -> Result<(), String> {
    tracing::info!("Calculating scores for GP {}", gp_id);

    let teams = sqlx::query_as::<_, (i64, i64, i64, i64, i64, Option<i64>)>(
        r#"
        SELECT id, driver_1_id, driver_2_id, constructor_id, gp_id, booster_driver_id 
        FROM "fantasy_teams" 
        WHERE gp_id = $1
        "#,
    )
    .bind(gp_id)
    .fetch_all(&state.db_pool)
    .await
    .map_err(|e| format!("Failed to fetch teams: {}", e))?;

    for (team_id, driver_1_id, driver_2_id, constructor_id, _, booster_driver_id) in teams {
        // Get driver results - treat missing drivers as DNF
        let driver_1_result = race_results
            .iter()
            .find(|r| r.driver_id == driver_1_id)
            .cloned()
            .unwrap_or(RaceResult {
                driver_id: driver_1_id,
                position: 0,
                fastest_lap: false,
                dnf: true,
            });
        
        let driver_2_result = race_results
            .iter()
            .find(|r| r.driver_id == driver_2_id)
            .cloned()
            .unwrap_or(RaceResult {
                driver_id: driver_2_id,
                position: 0,
                fastest_lap: false,
                dnf: true,
            });
        
        let constructor_result = constructor_results
            .iter()
            .find(|r| r.constructor_id == constructor_id)
            .cloned()
            .unwrap_or(ConstructorResult {
                constructor_id,
                position: 0,
            });

        // Validate positions and DNF status
        let driver_1_position = driver_1_result.position;
        let driver_1_is_dnf = driver_1_result.dnf;
        let driver_2_position = driver_2_result.position;
        let driver_2_is_dnf = driver_2_result.dnf;
        let constructor_position = constructor_result.position;

        tracing::info!(
            "Team {}: driver_1_id={}, driver_1_result={:?}, driver_2_id={}, driver_2_result={:?}",
            team_id,
            driver_1_id,
            driver_1_result,
            driver_2_id,
            driver_2_result
        );

        // Calculate fastest lap status
        let driver_1_fastest = driver_1_result.fastest_lap || fastest_lap_driver_id == Some(driver_1_id);
        let driver_2_fastest = driver_2_result.fastest_lap || fastest_lap_driver_id == Some(driver_2_id);

        // Check if booster driver is valid
        let is_driver_1 = booster_driver_id.map(|id| id == driver_1_id).unwrap_or(false);

        let score = calculate_team_score(
            driver_1_position,
            driver_2_position,
            constructor_position,
            driver_1_fastest,
            driver_2_fastest,
            booster_driver_id,
            is_driver_1,
            driver_1_is_dnf,
            driver_2_is_dnf,
        );

        sqlx::query(
            r#"
            UPDATE "fantasy_teams" 
            SET total_points = $1, is_locked = true, updated_at = NOW()
            WHERE id = $2
            "#,
        )
        .bind(score.total_points)
        .bind(team_id)
        .execute(&state.db_pool)
        .await
        .map_err(|e| format!("Failed to update team score: {}", e))?;

        tracing::info!(
            "Team {} scored {} points (D1: {}, D2: {}, C: {}, FL: {}, Boost: {})",
            team_id,
            score.total_points,
            score.driver_1_points,
            score.driver_2_points,
            score.constructor_points,
            score.fastest_lap_bonus,
            score.booster_points
        );
    }

    Ok(())
}

pub fn calculate_driver_price_change(position: i64) -> i64 {
    let change = match position {
        1 => 5_000_000,
        2 => 3_000_000,
        3 => 2_000_000,
        4 | 5 | 6 => 1_000_000,
        7 | 8 | 9 | 10 => 0,
        11 | 12 | 13 | 14 | 15 | 16 | 17 | 18 | 19 | 20 => -1_000_000,
        _ => -2_000_000,
    };

    change
}

pub fn calculate_constructor_price_change(
    combined_points: i64,
    both_in_points: bool,
    is_dominant: bool,
) -> i64 {
    let mut change = match combined_points {
        50 => 8_000_000,
        43 | 37 => 6_000_000,
        40 => 5_000_000,
        28..=39 => 4_000_000,
        15..=27 => 2_000_000,
        1..=14 => 1_000_000,
        0 => -2_000_000,
        _ => -3_000_000,
    };

    if is_dominant {
        change += 3_000_000;
    }

    if both_in_points {
        change += 1_000_000;
    }

    change
}

pub async fn update_prices_after_gp(
    state: &Arc<AppState>,
    gp_id: i64,
    year: i64,
    race_results: &[RaceResult],
    constructor_results: &[ConstructorResult],
) -> Result<(), String> {
    tracing::info!("Updating prices after GP {}", gp_id);

    let mut driver_price_changes: Vec<(i64, i64)> = Vec::new();
    let mut constructor_price_changes: Vec<(i64, i64)> = Vec::new();

    for result in race_results {
        if result.dnf || result.position <= 0 {
            continue;
        }

        let change = calculate_driver_price_change(result.position);
        driver_price_changes.push((result.driver_id, change));
    }

    let driver_constructor_map: Vec<(i64, i64)> = sqlx::query_as(
        r#"SELECT id, team_id FROM "fantasy_drivers" WHERE year = $1 AND team_id IS NOT NULL"#,
    )
    .bind(year)
    .fetch_all(&state.db_pool)
    .await
    .map_err(|e| format!("Failed to fetch driver-constructor mapping: {}", e))?;

    let driver_to_constructor: std::collections::HashMap<i64, i64> = driver_constructor_map
        .into_iter()
        .collect();

    let constructors: Vec<i64> = sqlx::query_as(
        r#"SELECT id FROM "fantasy_constructors" WHERE year = $1"#,
    )
    .bind(year)
    .fetch_all(&state.db_pool)
    .await
    .map_err(|e| format!("Failed to fetch constructors: {}", e))?
    .into_iter()
    .map(|(id,)| id)
    .collect();

    let mut constructor_driver_counts: std::collections::HashMap<i64, Vec<i64>> = std::collections::HashMap::new();
    for result in race_results {
        if result.dnf || result.position <= 0 {
            continue;
        }
        if let Some(&constructor_id) = driver_to_constructor.get(&result.driver_id) {
            constructor_driver_counts
                .entry(constructor_id)
                .or_default()
                .push(result.driver_id);
        }
    }

    for constructor_id in constructors {
        let constructor_result = constructor_results
            .iter()
            .find(|r| r.constructor_id == constructor_id);

        if let Some(c_result) = constructor_result {
            if c_result.position <= 0 {
                continue;
            }

            let drivers_for_constructor = constructor_driver_counts.get(&constructor_id);
            let drivers_count = drivers_for_constructor.map(|v| v.len()).unwrap_or(0);
            
            let both_in_points = drivers_count >= 2;
            
            let combined_points = if drivers_count > 0 {
                let mut points = 0;
                if let Some(drivers) = drivers_for_constructor {
                    for driver_id in drivers {
                        if let Some(result) = race_results.iter().find(|r| r.driver_id == *driver_id) {
                            points += get_driver_finishing_points(result.position);
                        }
                    }
                }
                points
            } else {
                0
            };

            let is_dominant = c_result.position == 1 && combined_points >= 25;
            let change = calculate_constructor_price_change(combined_points, both_in_points, is_dominant);
            constructor_price_changes.push((constructor_id, change));
        }
    }

    for (driver_id, change) in &driver_price_changes {
        sqlx::query(
            r#"UPDATE "fantasy_drivers" SET salary = salary + $1 WHERE id = $2"#,
        )
        .bind(change)
        .bind(driver_id)
        .execute(&state.db_pool)
        .await
        .map_err(|e| format!("Failed to update driver price: {}", e))?;

        tracing::info!("Driver {} price changed by {}", driver_id, change);
    }

    for (constructor_id, change) in &constructor_price_changes {
        sqlx::query(
            r#"UPDATE "fantasy_constructors" SET salary = salary + $1 WHERE id = $2"#,
        )
        .bind(change)
        .bind(constructor_id)
        .execute(&state.db_pool)
        .await
        .map_err(|e| format!("Failed to update constructor price: {}", e))?;

        tracing::info!("Constructor {} price changed by {}", constructor_id, change);
    }

    tracing::info!(
        "Price update complete: {} drivers, {} constructors updated",
        driver_price_changes.len(),
        constructor_price_changes.len()
    );

    Ok(())
}
