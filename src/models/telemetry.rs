use serde::Deserialize;

#[derive(Deserialize)]
pub struct TelemetryQuery {
    pub session_key: String,
    pub driver_number: String,
}
