use serde::{Deserialize, Serialize};
use sqlx::prelude::FromRow;

#[derive(FromRow, Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: i32,
    pub race_id: String,
    #[serde(rename = "sessionType")]
    pub session_type: String,
    pub date: String,
    pub session_key: Option<i64>,
    pub meeting_key: Option<i64>,
    // Add other fields from your Sessions table
}