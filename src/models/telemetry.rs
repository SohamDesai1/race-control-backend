use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct TelemetryQuery {
    pub session_key: String,
    pub driver_number: String,
}

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

#[derive(Serialize)]
pub struct SpeedDistance {
    pub speed: f64,
    pub distance: f64,
}
