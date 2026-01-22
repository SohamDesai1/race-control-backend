// In your state.rs file
use crate::{
    models::{
        cache::CacheEntry,
        telemetry::{DriverLapGraph, FastestLapSector, PacePoint, SpeedDistance},
    },
    utils::config::Config,
};
use dashmap::DashMap;
use sqlx::PgPool;
use reqwest::Client;

#[derive(Clone)]
pub struct AppState {
    pub db_pool: PgPool,
    pub config: Config,
    pub http_client: Client,
    pub fetch_driver_telemetry_cache: DashMap<String, CacheEntry<Vec<SpeedDistance>>>,
    pub get_drivers_position_telemetry_cache: DashMap<String, CacheEntry<Vec<DriverLapGraph>>>,
    pub get_sector_timings_cache: DashMap<String, CacheEntry<Vec<FastestLapSector>>>,
    pub get_race_pace_cache: DashMap<String, CacheEntry<Vec<PacePoint>>>,
}