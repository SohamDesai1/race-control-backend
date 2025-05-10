use std::env;
use postgrest::Postgrest;

#[derive(Clone)]
pub struct AppState {
    pub supabase: Postgrest,
}

impl AppState {
    pub async fn init() -> Self {
        let url = env::var("SUPABASE_PROJECT_URL").unwrap();
        let api_key = env::var("SUPABASE_ANNON_KEY").unwrap();
        let jwt_token = env::var("SUPABASE_JWT_TOKEN").unwrap();

        let client = Postgrest::new(&format!("{}/rest/v1", url))
            .insert_header("apikey", &api_key)
            .insert_header("Authorization", &format!("Bearer {}", jwt_token));

        AppState { supabase: client }
    }
}
