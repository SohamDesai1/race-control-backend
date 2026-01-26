// In your state.rs file
use crate::{
    models::{
        cache::CacheEntry,
        telemetry::{
            DriverLapGraph, FastestLapSector, PacePoint, QualifyingRankings, SpeedDistance,
        },
    },
    utils::{config::Config, rate_limiter::RateLimiter},
};
use dashmap::DashMap;
use reqwest::Client;
use sqlx::PgPool;

#[derive(Clone)]
pub struct AppState {
    pub db_pool: PgPool,
    pub config: Config,
    pub http_client: Client,
    pub fetch_driver_telemetry_cache: DashMap<String, CacheEntry<Vec<SpeedDistance>>>,
    pub get_drivers_position_telemetry_cache: DashMap<String, CacheEntry<Vec<DriverLapGraph>>>,
    pub get_sector_timings_cache: DashMap<String, CacheEntry<Vec<FastestLapSector>>>,
    pub get_race_pace_cache: DashMap<String, CacheEntry<Vec<PacePoint>>>,
    pub quali_session_cache: DashMap<String, CacheEntry<QualifyingRankings>>,
    pub rate_limiter: RateLimiter,
}
