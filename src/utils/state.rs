use postgrest::Postgrest;
use reqwest::Client;
use supabase_auth::models::AuthClient;
use crate::utils::config::Config;

#[derive(Clone)]
pub struct AppState {
    pub supabase: Postgrest,
    pub supabase_auth : AuthClient, 
    pub config: Config,
    pub http_client: Client,
}

