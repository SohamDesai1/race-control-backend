use chrono::DateTime;
use serde::{Deserialize, Serialize};
use sqlx::prelude::FromRow;

#[derive(FromRow, Deserialize, Serialize, Clone, Debug)]
pub struct FantasyDriver {
    pub id: i32,
    pub name: String,
    pub code: String,
    pub team_id: Option<i32>,
    pub salary: i32,
    pub year: i32,
}

#[derive(FromRow, Deserialize, Serialize, Clone, Debug)]
pub struct FantasyConstructor {
    pub id: i32,
    pub name: String,
    pub salary: i32,
    pub year: i32,
}

#[derive(FromRow, Deserialize, Serialize, Clone, Debug)]
pub struct FantasyContest {
    pub id: i32,
    pub name: String,
    pub creator_id: i32,
    pub contest_type: String,
    pub gp_id: Option<i32>,
    pub invite_code: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<DateTime<chrono::Utc>>,
}

#[derive(FromRow, Deserialize, Serialize, Clone, Debug)]
pub struct FantasyContestParticipant {
    pub id: i32,
    pub contest_id: i32,
    pub user_id: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub joined_at: Option<DateTime<chrono::Utc>>,
}

#[derive(FromRow, Deserialize, Serialize, Clone, Debug)]
pub struct FantasyTeam {
    pub id: i32,
    pub user_id: i32,
    pub contest_id: Option<i32>,
    pub gp_id: i32,
    pub driver_1_id: i32,
    pub driver_2_id: i32,
    pub constructor_id: i32,
    pub booster_driver_id: Option<i32>,
    pub budget_used: i32,
    pub is_locked: bool,
    pub total_points: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<DateTime<chrono::Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTime<chrono::Utc>>,
}

#[derive(Deserialize)]
pub struct CreateFantasyTeamRequest {
    pub driver_1_id: i32,
    pub driver_2_id: i32,
    pub constructor_id: i32,
    pub booster_driver_id: Option<i32>,
    pub contest_id: Option<i32>,
}

#[derive(Deserialize)]
pub struct CreateContestRequest {
    pub name: String,
    pub contest_type: String,
    pub gp_id: Option<i32>,
}

#[derive(Serialize)]
pub struct FantasyTeamWithDetails {
    pub id: i32,
    pub user_id: i32,
    pub contest_id: Option<i32>,
    pub gp_id: i32,
    pub driver_1: FantasyDriver,
    pub driver_2: FantasyDriver,
    pub constructor: FantasyConstructor,
    pub booster_driver: Option<FantasyDriver>,
    pub budget_used: i32,
    pub is_locked: bool,
    pub total_points: i32,
    pub created_at: Option<DateTime<chrono::Utc>>,
    pub updated_at: Option<DateTime<chrono::Utc>>,
}

#[derive(Serialize)]
pub struct LeaderboardEntry {
    pub rank: i64,
    pub user_id: i64,
    pub username: Option<String>,
    pub total_points: i64,
    pub team: Option<FantasyTeamWithDetails>,
}
