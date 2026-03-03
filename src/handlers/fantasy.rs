use std::sync::Arc;

use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};

use crate::{
    models::fantasy::{
        CreateContestRequest, CreateFantasyTeamRequest, FantasyConstructor, FantasyContest,
        FantasyContestParticipant, FantasyDriver, FantasyTeam, FantasyTeamWithDetails,
        LeaderboardEntry,
    },
    models::jwt::Claims,
    utils::state::AppState,
};
use rand::Rng;
use serde::{Deserialize, Serialize};
use sqlx::prelude::FromRow;

#[derive(Serialize)]
struct GpInfoResponse {
    gp_id: i32,
    race_name: String,
    date: String,
    available_drivers: Vec<FantasyDriver>,
    available_constructors: Vec<FantasyConstructor>,
}

#[derive(FromRow, Serialize)]
struct RaceInfo {
    id: i32,
    race_name: String,
    date: Option<String>,
}

fn generate_invite_code() -> String {
    let charset: Vec<char> = "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789".chars().collect();
    let mut rng = rand::thread_rng();
    (0..6)
        .map(|_| charset[rng.gen_range(0..charset.len())])
        .collect()
}

pub async fn get_drivers(
    State(state): State<Arc<AppState>>,
    Query(params): Query<YearQuery>,
) -> impl IntoResponse {
    let year = params.year.unwrap_or(2026);

    let result = sqlx::query_as::<_, FantasyDriver>(
        r#"SELECT fd.*, fc.name as constructor_name
        FROM "fantasy_drivers" fd
        LEFT JOIN "fantasy_constructors" fc ON fd.team_id = fc.id
        WHERE fd.year = $1
        ORDER BY fd.salary DESC"#,
    )
    .bind(year)
    .fetch_all(&state.db_pool)
    .await;

    match result {
        Ok(drivers) => (StatusCode::OK, Json(drivers)).into_response(),
        Err(e) => {
            tracing::error!("Error fetching drivers: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Failed to fetch drivers"})),
            )
                .into_response()
        }
    }
}

#[derive(Deserialize)]
pub struct YearQuery {
    year: Option<i32>,
}

pub async fn get_constructors(
    State(state): State<Arc<AppState>>,
    Query(params): Query<YearQuery>,
) -> impl IntoResponse {
    let year = params.year.unwrap_or(2026);

    let result = sqlx::query_as::<_, FantasyConstructor>(
        r#"SELECT * FROM "fantasy_constructors" WHERE year = $1 ORDER BY salary DESC"#,
    )
    .bind(year)
    .fetch_all(&state.db_pool)
    .await;

    match result {
        Ok(constructors) => (StatusCode::OK, Json(constructors)).into_response(),
        Err(e) => {
            tracing::error!("Error fetching constructors: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Failed to fetch constructors"})),
            )
                .into_response()
        }
    }
}

pub async fn create_contest(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<CreateContestRequest>,
) -> impl IntoResponse {
    // Validate contest_type
    if payload.contest_type != "gp" && payload.contest_type != "season" {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "Invalid contest_type. Must be 'gp' or 'season'"})),
        )
            .into_response();
    }

    // For 'gp' contest, gp_id is required
    if payload.contest_type == "gp" && payload.gp_id.is_none() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "gp_id is required for 'gp' type contests"})),
        )
            .into_response();
    }

    // For 'season' contest, gp_id should be null
    if payload.contest_type == "season" && payload.gp_id.is_some() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "gp_id should be null for 'season' type contests"})),
        )
            .into_response();
    }

    let invite_code = generate_invite_code();

    let user_id = claims.id.clone();
    let result = sqlx::query_as::<_, FantasyContest>(
        r#"INSERT INTO "fantasy_contests" (name, creator_id, contest_type, gp_id, invite_code)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING *"#,
    )
    .bind(&payload.name)
    .bind(&user_id)
    .bind(&payload.contest_type)
    .bind(&payload.gp_id)
    .bind(&invite_code)
    .fetch_one(&state.db_pool)
    .await;

    match result {
        Ok(contest) => {
            sqlx::query(
                r#"INSERT INTO "fantasy_contest_participants" (contest_id, user_id) VALUES ($1, $2)"#,
            )
            .bind(contest.id)
            .bind(&user_id)
            .execute(&state.db_pool)
            .await
            .ok();

            (StatusCode::CREATED, Json(contest)).into_response()
        }
        Err(e) => {
            tracing::error!("Error creating contest: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Failed to create contest"})),
            )
                .into_response()
        }
    }
}

pub async fn get_user_contests(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
) -> impl IntoResponse {
    let result = sqlx::query_as::<_, FantasyContest>(
        r#"SELECT fc.* FROM "fantasy_contests" fc
        INNER JOIN "fantasy_contest_participants" fcp ON fc.id = fcp.contest_id
        WHERE fcp.user_id = $1
        ORDER BY fc.created_at DESC"#,
    )
    .bind(claims.sub)
    .fetch_all(&state.db_pool)
    .await;

    match result {
        Ok(contests) => (StatusCode::OK, Json(contests)).into_response(),
        Err(e) => {
            tracing::error!("Error fetching contests: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Failed to fetch contests"})),
            )
                .into_response()
        }
    }
}

pub async fn get_contest_by_invite(
    Path(invite_code): Path<String>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let result = sqlx::query_as::<_, FantasyContest>(
        r#"SELECT * FROM "fantasy_contests" WHERE invite_code = $1"#,
    )
    .bind(&invite_code)
    .fetch_optional(&state.db_pool)
    .await;

    match result {
        Ok(Some(contest)) => (StatusCode::OK, Json(contest)).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Contest not found"})),
        )
            .into_response(),
        Err(e) => {
            tracing::error!("Error fetching contest: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Failed to fetch contest"})),
            )
                .into_response()
        }
    }
}

pub async fn join_contest(
    Path(invite_code): Path<String>,
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
) -> impl IntoResponse {
    let user_id = claims.id.clone();
    let contest_result = sqlx::query_as::<_, FantasyContest>(
        r#"SELECT * FROM "fantasy_contests" WHERE invite_code = $1"#,
    )
    .bind(&invite_code)
    .fetch_optional(&state.db_pool)
    .await;

    let contest = match contest_result {
        Ok(Some(c)) => c,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": "Contest not found"})),
            )
                .into_response()
        }
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Database error"})),
            )
                .into_response()
        }
    };

    let check = sqlx::query(
        r#"SELECT 1 FROM "fantasy_contest_participants" WHERE contest_id = $1 AND user_id = $2"#,
    )
    .bind(contest.id)
    .bind(&user_id)
    .fetch_optional(&state.db_pool)
    .await;

    if check.is_ok() && check.unwrap().is_some() {
        return (
            StatusCode::CONFLICT,
            Json(serde_json::json!({"error": "Already joined this contest"})),
        )
            .into_response();
    }

    let result = sqlx::query(
        r#"INSERT INTO "fantasy_contest_participants" (contest_id, user_id) VALUES ($1, $2)"#,
    )
    .bind(contest.id)
    .bind(&user_id)
    .execute(&state.db_pool)
    .await;

    match result {
        Ok(_) => (StatusCode::OK, Json(contest)).into_response(),
        Err(e) => {
            tracing::error!("Error joining contest: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Failed to join contest"})),
            )
                .into_response()
        }
    }
}

pub async fn leave_contest(
    Path(contest_id): Path<i32>,
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
) -> impl IntoResponse {
    let user_id = claims.id.clone();
    let result = sqlx::query(
        r#"DELETE FROM "fantasy_contest_participants" WHERE contest_id = $1 AND user_id = $2"#,
    )
    .bind(contest_id)
    .bind(&user_id)
    .execute(&state.db_pool)
    .await;

    match result {
        Ok(res) => {
            if res.rows_affected() == 0 {
                return (
                    StatusCode::NOT_FOUND,
                    Json(serde_json::json!({"error": "Not a participant of this contest"})),
                )
                    .into_response();
            }
            (
                StatusCode::OK,
                Json(serde_json::json!({"message": "Left contest successfully"})),
            )
                .into_response()
        }
        Err(e) => {
            tracing::error!("Error leaving contest: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Failed to leave contest"})),
            )
                .into_response()
        }
    }
}

pub async fn get_contest_details(
    Path(contest_id): Path<i32>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let contest_result =
        sqlx::query_as::<_, FantasyContest>(r#"SELECT * FROM "fantasy_contests" WHERE id = $1"#)
            .bind(contest_id)
            .fetch_optional(&state.db_pool)
            .await;

    let contest = match contest_result {
        Ok(Some(c)) => c,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": "Contest not found"})),
            )
                .into_response()
        }
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Database error"})),
            )
                .into_response()
        }
    };

    let participants = sqlx::query_as::<_, FantasyContestParticipant>(
        r#"SELECT * FROM "fantasy_contest_participants" WHERE contest_id = $1"#,
    )
    .bind(contest_id)
    .fetch_all(&state.db_pool)
    .await
    .unwrap_or_default();

    let participant_count = participants.len() as i32;

    let response = serde_json::json!({
        "id": contest.id,
        "name": contest.name,
        "creator_id": contest.creator_id,
        "contest_type": contest.contest_type,
        "gp_id": contest.gp_id,
        "invite_code": contest.invite_code,
        "created_at": contest.created_at,
        "participants": participants,
        "participant_count": participant_count
    });

    (StatusCode::OK, Json(response)).into_response()
}

pub async fn get_race_info(
    Path(gp_id): Path<i32>,
    State(state): State<Arc<AppState>>,
    Query(params): Query<YearQuery>,
) -> impl IntoResponse {
    let year = params.year.unwrap_or(2026);

    let race_result = sqlx::query_as::<_, RaceInfo>(
        r#"SELECT id, "raceName" as race_name, TO_CHAR(date, 'YYYY-MM-DD') as date FROM "Races" WHERE id = $1"#,
    )
    .bind(gp_id)
    .fetch_optional(&state.db_pool)
    .await;

    let race = match race_result {
        Ok(Some(r)) => r,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": "GP not found"})),
            )
                .into_response()
        }
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Database error"})),
            )
                .into_response()
        }
    };

    let drivers = sqlx::query_as::<_, FantasyDriver>(
        r#"SELECT * FROM "fantasy_drivers" WHERE year = $1 ORDER BY salary DESC"#,
    )
    .bind(year)
    .fetch_all(&state.db_pool)
    .await
    .unwrap_or_default();

    let constructors = sqlx::query_as::<_, FantasyConstructor>(
        r#"SELECT * FROM "fantasy_constructors" WHERE year = $1 ORDER BY salary DESC"#,
    )
    .bind(year)
    .fetch_all(&state.db_pool)
    .await
    .unwrap_or_default();

    let response = GpInfoResponse {
        gp_id: race.id,
        race_name: race.race_name,
        date: race.date.unwrap_or_default(),
        available_drivers: drivers,
        available_constructors: constructors,
    };

    (StatusCode::OK, Json(response)).into_response()
}

pub async fn get_team_for_gp(
    Path(gp_id): Path<i32>,
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
) -> impl IntoResponse {
    let user_id = claims.id.clone();
    let result = sqlx::query_as::<_, FantasyTeam>(
        r#"SELECT * FROM "fantasy_teams" WHERE user_id = $1 AND gp_id = $2"#,
    )
    .bind(&user_id)
    .bind(gp_id)
    .fetch_optional(&state.db_pool)
    .await;

    match result {
        Ok(Some(team)) => (StatusCode::OK, Json(team)).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "No team found for this GP"})),
        )
            .into_response(),
        Err(e) => {
            tracing::error!("Error fetching team: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Failed to fetch team"})),
            )
                .into_response()
        }
    }
}

pub async fn create_or_update_team(
    Path(gp_id): Path<i32>,
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<CreateFantasyTeamRequest>,
) -> impl IntoResponse {
    let user_id = claims.id.clone();
    if payload.driver_1_id == payload.driver_2_id {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "Cannot select same driver twice"})),
        )
            .into_response();
    }

    let driver1 =
        sqlx::query_as::<_, FantasyDriver>(r#"SELECT * FROM "fantasy_drivers" WHERE id = $1"#)
            .bind(payload.driver_1_id)
            .fetch_optional(&state.db_pool)
            .await
            .ok()
            .flatten();

    let driver2 =
        sqlx::query_as::<_, FantasyDriver>(r#"SELECT * FROM "fantasy_drivers" WHERE id = $1"#)
            .bind(payload.driver_2_id)
            .fetch_optional(&state.db_pool)
            .await
            .ok()
            .flatten();

    let constructor = sqlx::query_as::<_, FantasyConstructor>(
        r#"SELECT * FROM "fantasy_constructors" WHERE id = $1"#,
    )
    .bind(payload.constructor_id)
    .fetch_optional(&state.db_pool)
    .await
    .ok()
    .flatten();

    if driver1.is_none() || driver2.is_none() || constructor.is_none() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "Invalid driver or constructor ID"})),
        )
            .into_response();
    }

    let d1 = driver1.unwrap();
    let d2 = driver2.unwrap();
    let c = constructor.unwrap();

    // Validate booster is one of the selected drivers
    if let Some(booster_id) = payload.booster_driver_id {
        if booster_id != d1.id && booster_id != d2.id {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "Booster driver must be one of the selected drivers"})),
            )
                .into_response();
        }
    }

    let budget_used = d1.salary + d2.salary + c.salary;

    if budget_used > 100_000_000 {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "Budget exceeded. Max budget is $100M", "budget_used": budget_used})),
        )
            .into_response();
    }

    let existing = sqlx::query_as::<_, FantasyTeam>(
        r#"SELECT * FROM "fantasy_teams" WHERE user_id = $1 AND gp_id = $2"#,
    )
    .bind(&user_id)
    .bind(gp_id)
    .fetch_optional(&state.db_pool)
    .await
    .ok()
    .flatten();

    if let Some(team) = existing {
        if team.is_locked {
            return (
                StatusCode::FORBIDDEN,
                Json(serde_json::json!({"error": "Team is locked. Cannot modify after qualifying started"})),
            )
                .into_response();
        }

        let result = sqlx::query(
            r#"UPDATE "fantasy_teams" 
            SET driver_1_id = $1, driver_2_id = $2, constructor_id = $3, 
                booster_driver_id = $4, budget_used = $5, contest_id = $6, updated_at = NOW()
            WHERE id = $7"#,
        )
        .bind(payload.driver_1_id)
        .bind(payload.driver_2_id)
        .bind(payload.constructor_id)
        .bind(&payload.booster_driver_id)
        .bind(budget_used)
        .bind(&payload.contest_id)
        .bind(team.id)
        .execute(&state.db_pool)
        .await;

        match result {
            Ok(_) => {
                let updated = sqlx::query_as::<_, FantasyTeam>(
                    r#"SELECT * FROM "fantasy_teams" WHERE id = $1"#,
                )
                .bind(team.id)
                .fetch_one(&state.db_pool)
                .await;

                return (StatusCode::OK, Json(updated.unwrap())).into_response();
            }
            Err(e) => {
                tracing::error!("Error updating team: {:?}", e);
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({"error": "Failed to update team"})),
                )
                    .into_response();
            }
        }
    }

    let result = sqlx::query_as::<_, FantasyTeam>(
        r#"INSERT INTO "fantasy_teams" 
        (user_id, contest_id, gp_id, driver_1_id, driver_2_id, constructor_id, booster_driver_id, budget_used)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        RETURNING *"#,
    )
    .bind(&user_id)
    .bind(&payload.contest_id)
    .bind(gp_id)
    .bind(payload.driver_1_id)
    .bind(payload.driver_2_id)
    .bind(payload.constructor_id)
    .bind(&payload.booster_driver_id)
    .bind(budget_used)
    .fetch_one(&state.db_pool)
    .await;

    match result {
        Ok(team) => (StatusCode::CREATED, Json(team)).into_response(),
        Err(e) => {
            tracing::error!("Error creating team: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Failed to create team"})),
            )
                .into_response()
        }
    }
}

pub async fn set_booster(
    Path(gp_id): Path<i32>,
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<serde_json::Value>,
) -> impl IntoResponse {
    let user_id = claims.id.clone();
    let booster_id = payload
        .get("booster_driver_id")
        .and_then(|v| v.as_i64())
        .map(|v| v as i32);

    let team = sqlx::query_as::<_, FantasyTeam>(
        r#"SELECT * FROM "fantasy_teams" WHERE user_id = $1 AND gp_id = $2"#,
    )
    .bind(&user_id)
    .bind(gp_id)
    .fetch_optional(&state.db_pool)
    .await
    .ok()
    .flatten();

    let team = match team {
        Some(t) => t,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": "No team found for this GP"})),
            )
                .into_response()
        }
    };

    if team.is_locked {
        return (
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({"error": "Team is locked. Cannot modify after qualifying started"})),
        )
            .into_response();
    }

    let result = sqlx::query(
        r#"UPDATE "fantasy_teams" SET booster_driver_id = $1, updated_at = NOW() WHERE id = $2"#,
    )
    .bind(booster_id)
    .bind(team.id)
    .execute(&state.db_pool)
    .await;

    match result {
        Ok(_) => {
            let updated =
                sqlx::query_as::<_, FantasyTeam>(r#"SELECT * FROM "fantasy_teams" WHERE id = $1"#)
                    .bind(team.id)
                    .fetch_one(&state.db_pool)
                    .await
                    .unwrap();

            (StatusCode::OK, Json(updated)).into_response()
        }
        Err(e) => {
            tracing::error!("Error setting booster: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Failed to set booster"})),
            )
                .into_response()
        }
    }
}

pub async fn delete_team(
    Path(gp_id): Path<i32>,
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
) -> impl IntoResponse {
    let user_id = claims.id.clone();
    let team = sqlx::query_as::<_, FantasyTeam>(
        r#"SELECT * FROM "fantasy_teams" WHERE user_id = $1 AND gp_id = $2"#,
    )
    .bind(&user_id)
    .bind(gp_id)
    .fetch_optional(&state.db_pool)
    .await
    .ok()
    .flatten();

    let team = match team {
        Some(t) => t,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": "No team found for this GP"})),
            )
                .into_response()
        }
    };

    if team.is_locked {
        return (
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({"error": "Team is locked. Cannot delete after qualifying started"})),
        )
            .into_response();
    }

    let result = sqlx::query(r#"DELETE FROM "fantasy_teams" WHERE id = $1"#)
        .bind(team.id)
        .execute(&state.db_pool)
        .await;

    match result {
        Ok(_) => (
            StatusCode::OK,
            Json(serde_json::json!({"message": "Team deleted successfully"})),
        )
            .into_response(),
        Err(e) => {
            tracing::error!("Error deleting team: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Failed to delete team"})),
            )
                .into_response()
        }
    }
}

pub async fn get_global_leaderboard(
    State(state): State<Arc<AppState>>,
    Query(params): Query<YearQuery>,
) -> impl IntoResponse {
    let year = params.year.unwrap_or(2026);

    let result = sqlx::query_as::<_, (i32, i32, Option<String>, i32)>(
        r#"
        SELECT ft.user_id, COUNT(ft.id) as team_count, u.username, COALESCE(SUM(ft.total_points), 0)::integer as total_points
        FROM "fantasy_teams" ft
        LEFT JOIN "Users" u ON ft.user_id = u.id
        INNER JOIN "Races" r ON ft.gp_id = r.id
        WHERE r.season = $1::text
        GROUP BY ft.user_id, u.username
        ORDER BY total_points DESC
        LIMIT 100
        "#,
    )
    .bind(year.to_string())
    .fetch_all(&state.db_pool)
    .await;

    match result {
        Ok(rows) => {
            let leaderboard: Vec<LeaderboardEntry> = rows
                .iter()
                .enumerate()
                .map(|(idx, row)| LeaderboardEntry {
                    rank: (idx + 1) as i64,
                    user_id: row.0 as i64,
                    username: row.2.clone(),
                    total_points: row.3 as i64,
                    team: None,
                })
                .collect();

            (StatusCode::OK, Json(leaderboard)).into_response()
        }
        Err(e) => {
            tracing::error!("Error fetching leaderboard: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Failed to fetch leaderboard"})),
            )
                .into_response()
        }
    }
}


pub async fn get_contest_leaderboard(
    Path(contest_id): Path<i32>,
    State(state): State<Arc<AppState>>,
    Query(params): Query<ContestLeaderboardQuery>,
) -> impl IntoResponse {
    let gp_id = params.gp_id;

    let query = if let Some(gp) = gp_id {
        sqlx::query_as::<_, (i32, i32, Option<String>, i32)>(
            r#"
            SELECT ft.user_id, ft.id, u.username, ft.total_points::integer
            FROM "fantasy_teams" ft
            LEFT JOIN "Users" u ON ft.user_id = u.id
            WHERE ft.contest_id = $1 AND ft.gp_id = $2
            ORDER BY ft.total_points DESC
            "#,
        )
        .bind(contest_id)
        .bind(gp as i32)
    } else {
        sqlx::query_as::<_, (i32, i32, Option<String>, i32)>(
            r#"
            SELECT ft.user_id, ft.id, u.username, COALESCE(SUM(ft.total_points), 0)::integer as total_points
            FROM "fantasy_teams" ft
            LEFT JOIN "Users" u ON ft.user_id = u.id
            WHERE ft.contest_id = $1
            GROUP BY ft.user_id, ft.id, u.username
            ORDER BY total_points DESC
            "#,
        )
        .bind(contest_id)
    };

    let result = query.fetch_all(&state.db_pool).await;

    match result {
        Ok(rows) => {
            let mut leaderboard: Vec<LeaderboardEntry> = Vec::new();

            for (idx, row) in rows.iter().enumerate() {
                let team_id = row.1;
                
                // Fetch team details
                let team_details = fetch_team_with_details(&state, team_id).await;

                leaderboard.push(LeaderboardEntry {
                    rank: (idx + 1) as i64,
                    user_id: row.0 as i64,
                    username: row.2.clone(),
                    total_points: row.3 as i64,
                    team: team_details,
                });
            }

            (StatusCode::OK, Json(leaderboard)).into_response()
        }
        Err(e) => {
            tracing::error!("Error fetching contest leaderboard: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Failed to fetch leaderboard"})),
            )
                .into_response()
        }
    }
}

async fn fetch_team_with_details(
    state: &Arc<AppState>,
    team_id: i32,
) -> Option<FantasyTeamWithDetails> {
    let team = sqlx::query_as::<_, FantasyTeam>(
        r#"SELECT * FROM "fantasy_teams" WHERE id = $1"#,
    )
    .bind(team_id)
    .fetch_optional(&state.db_pool)
    .await
    .ok()?;

    let team = team?;

    let driver_1 = sqlx::query_as::<_, FantasyDriver>(
        r#"SELECT * FROM "fantasy_drivers" WHERE id = $1"#,
    )
    .bind(team.driver_1_id)
    .fetch_optional(&state.db_pool)
    .await
    .ok()??;

    let driver_2 = sqlx::query_as::<_, FantasyDriver>(
        r#"SELECT * FROM "fantasy_drivers" WHERE id = $1"#,
    )
    .bind(team.driver_2_id)
    .fetch_optional(&state.db_pool)
    .await
    .ok()??;

    let constructor = sqlx::query_as::<_, FantasyConstructor>(
        r#"SELECT * FROM "fantasy_constructors" WHERE id = $1"#,
    )
    .bind(team.constructor_id)
    .fetch_optional(&state.db_pool)
    .await
    .ok()??;

    let booster_driver = if let Some(booster_id) = team.booster_driver_id {
        sqlx::query_as::<_, FantasyDriver>(
            r#"SELECT * FROM "fantasy_drivers" WHERE id = $1"#,
        )
        .bind(booster_id)
        .fetch_optional(&state.db_pool)
        .await
        .ok()?
    } else {
        None
    };

    Some(FantasyTeamWithDetails {
        id: team.id,
        user_id: team.user_id,
        contest_id: team.contest_id,
        gp_id: team.gp_id,
        driver_1,
        driver_2,
        constructor,
        booster_driver,
        budget_used: team.budget_used,
        is_locked: team.is_locked,
        total_points: team.total_points,
        created_at: team.created_at,
        updated_at: team.updated_at,
    })
}

#[derive(Deserialize)]
pub struct ContestLeaderboardQuery {
    gp_id: Option<i64>,
}
