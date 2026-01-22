use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(FromRow, Debug, Clone, Serialize, Deserialize)]
pub struct Race {
    pub id: i32,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    pub season: String,
    pub round: String,
    pub date: String,
    pub time: String,
    #[serde(rename = "raceName")]
    pub race_name: String,
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

#[derive(FromRow, Debug, Clone, Serialize, Deserialize)]
pub struct RaceWithCircuit {
    pub id: i32,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    pub season: String,
    pub round: String,
    pub date: String,
    pub time: String,
    #[serde(rename = "raceName")]
    pub race_name: String,
    #[serde(rename = "circuitId")]
    pub circuit_id: String,
    #[serde(rename = "circuitName")]
    pub circuit_name: String,
    pub location: Option<String>,
    pub country: Option<String>,
    pub lat: Option<String>,
    pub long: Option<String>,
}
