#[derive(Debug, Clone)]
pub struct Config {
    pub supabase_project_url: String,
    pub _supabase_anon_key: String,
    pub supabase_service_role_key: String,
    pub jwt_secret: String,
}

impl Config {
    pub fn init() -> Self {
        Config {
            supabase_project_url: std::env::var("SUPABASE_PROJECT_URL")
                .expect("SUPABASE_PROJECT_URL not set"),
            _supabase_anon_key: std::env::var("SUPABASE_ANON_KEY")
                .expect("SUPABASE_ANON_KEY not set"),
            supabase_service_role_key: std::env::var("SUPABASE_SERVICE_ROLE_KEY")
                .expect("SUPABASE_SERVICE_ROLE_KEY not set"),
            jwt_secret: std::env::var("JWT_SECRET").expect("JWT_SECRET not set"),
        }
    }
}
