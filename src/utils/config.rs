#[derive(Debug, Clone)]
pub struct Config {
    pub supabase_project_url: String,
    pub supabase_anon_key: String,
    pub supabase_service_role_key: String,
    pub jwk_x: String,
    pub jwk_y: String,
}

impl Config {
    pub fn init() -> Self {
        Config {
            supabase_project_url: std::env::var("SUPABASE_PROJECT_URL")
                .expect("SUPABASE_PROJECT_URL not set"),
            supabase_anon_key: std::env::var("SUPABASE_ANON_KEY")
                .expect("SUPABASE_ANON_KEY not set"),
            supabase_service_role_key: std::env::var("SUPABASE_SERVICE_ROLE_KEY")
                .expect("SUPABASE_SERVICE_ROLE_KEY not set"),
            jwk_x: std::env::var("JWK_X").expect("JWK_X not set"),
            jwk_y: std::env::var("JWK_Y").expect("JWK_Y not set"),
        }
    }
}
