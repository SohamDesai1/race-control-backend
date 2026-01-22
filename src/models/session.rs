use chrono::{NaiveDate, NaiveTime};
use serde::{Deserialize, Serialize};
use sqlx::prelude::FromRow;

#[derive(FromRow, Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: i32,
    #[sqlx(rename = "raceId")]
    #[serde(rename = "raceId")]
    pub race_id: i32,
    #[sqlx(rename = "sessionType")]
    #[serde(rename = "sessionType")]
    pub session_type: String,
    pub date: Option<NaiveDate>,
    pub time: Option<NaiveTime>,
    pub session_key: Option<i32>,
    pub meeting_key: Option<i32>,
}
