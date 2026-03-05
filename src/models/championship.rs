use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(FromRow, Debug, Clone, Serialize, Deserialize)]
pub struct _DriverPointsHistory {
    pub id: i64,
    pub driver_number: String,
    pub session_key: i32,
    pub meeting_key: Option<i32>,
    pub season: String,
    pub round: String,
    pub race_id: Option<i64>,
    pub points_start: f64,
    pub points_current: f64,
    pub position: Option<i32>,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(FromRow, Debug, Clone, Serialize, Deserialize)]
pub struct _ConstructorChampionshipHistory {
    pub id: i64,
    pub constructor_id: String,
    pub constructor_name: String,
    pub session_key: i32,
    pub meeting_key: Option<i32>,
    pub season: String,
    pub round: String,
    pub race_id: Option<i64>,
    pub points_start: f64,
    pub points_current: f64,
    pub position: Option<i32>,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
}
