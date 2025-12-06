use crate::{
    models::{cache::CacheEntry, telemetry::{DriverLapGraph, SpeedDistance}},
    utils::config::Config,
};
use dashmap::DashMap;
use postgrest::Postgrest;
use reqwest::Client;
use supabase_auth::models::AuthClient;

#[derive(Clone)]
pub struct AppState {
    pub supabase: Postgrest,
    pub supabase_auth: AuthClient,
    pub config: Config,
    pub http_client: Client,
    pub fetch_driver_telemetry_cache: DashMap<String, CacheEntry<Vec<SpeedDistance>>>,
    pub get_drivers_position_telemetry_cache: DashMap<String, CacheEntry<Vec<DriverLapGraph>>>,
}
