use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[allow(dead_code)]
#[derive(Deserialize)]
pub struct CarDataPoint {
    pub date: String,
    pub session_key: u32,
    pub driver_number: u32,
    pub throttle: Option<f64>,
    pub meeting_key: u32,
    pub brake: Option<f64>,
    pub n_gear: Option<u32>,
    pub rpm: Option<u32>,
    pub speed: f64,
    pub drs: Option<u32>,
}

#[derive(Serialize, Clone)]
pub struct SpeedDistance {
    pub speed: f64,
    pub distance: f64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct LapRecord {
    pub lap_number: u32,

    #[serde(rename = "date_start")]
    pub date_start: Option<DateTime<Utc>>,

    pub driver_number: u32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct PositionRecord {
    pub position: u32,
    pub driver_number: u32,
    pub date: DateTime<Utc>,
}

#[derive(Debug, Serialize, Clone)]
pub struct LapPosition {
    pub lap: u32,
    pub position: u32,
}

#[derive(Debug, Serialize, Clone)]
pub struct DriverLapGraph {
    pub driver_number: u32,
    pub data: Vec<LapPosition>,
}

#[derive(Serialize, Clone)]
pub struct FastestLapSector {
    pub position: u32,
    pub driver_number: u32,
    pub fastest_lap: f64,
    pub sector_1: f64,
    pub sector_2: f64,
    pub sector_3: f64,
}

#[derive(Deserialize)]
pub struct PaceQuery {
    pub driver_1: u32,
    pub driver_2: u32,
}

#[derive(Deserialize)]
pub struct LocationPoint {
    pub date: String,
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

#[derive(Deserialize)]
pub struct Lap {
    pub lap_duration: Option<f64>,
    pub date_start: Option<String>,
}

#[derive(Serialize, Clone)]
pub struct PacePoint {
    pub x: f64,
    pub y: f64,
    pub minisector: u32,
    pub fastest_driver: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct QualifyingRanking {
    pub position: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub driver_number: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub driver_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub driver_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub constructor: Option<String>,
    pub time: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_seconds: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct QualifyingRankings {
    pub q1: Vec<QualifyingRanking>,
    pub q2: Vec<QualifyingRanking>,
    pub q3: Vec<QualifyingRanking>,
}
