use postgrest::Postgrest;
use crate::utils::config::Config;

#[derive(Clone)]
pub struct AppState {
    pub supabase: Postgrest,
    pub config: Config,
}

