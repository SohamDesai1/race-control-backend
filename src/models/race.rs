use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(FromRow, Debug, Clone, Serialize, Deserialize)]
pub struct _Race {
    pub id: i64,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    pub season: String,
    pub round: String,
    pub date: Option<chrono::NaiveDate>,
    pub time: Option<chrono::NaiveTime>,
    #[sqlx(rename = "raceName")]
    #[serde(rename = "raceName")]
    pub race_name: String,
    #[sqlx(rename = "circuitId")]
    #[serde(rename = "circuitId")]
    pub circuit_id: String,
}

#[derive(FromRow, Debug, Clone, Serialize, Deserialize)]
pub struct _Circuit {
    #[serde(rename = "circuitId")]
    pub circuit_id: String,
    #[serde(rename = "circuitName")]
    pub circuit_name: String,
    pub location: Option<String>,
    pub country: Option<String>,
    pub lat: Option<String>,
    pub long: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct RaceWithCircuit {
    pub id: i64,                                // BIGINT
    pub created_at: Option<DateTime<Utc>>,
    pub season: String,
    pub round: String,
    pub date: Option<NaiveDate>,                // DATE
    pub time: Option<NaiveTime>,                // TIME
    pub race_name: String,
    pub circuit_id: String,
    pub circuit_name: String,
    pub locality: Option<String>,
    pub country: Option<String>,
    pub lat: Option<String>,
    pub long: Option<String>,
}
