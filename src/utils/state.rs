use crate::{
    models::{cache::CacheEntry, telemetry::DriverLapGraph},
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
    pub cache: DashMap<String, CacheEntry<Vec<DriverLapGraph>>>,
}
