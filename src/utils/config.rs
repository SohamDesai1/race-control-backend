#[derive(Debug, Clone)]
pub struct Config {
    pub jwt_secret: String,
    pub supabase_project_url: String,
    pub supabase_annon_key: String,
    pub supabase_jwt_token: String,
}

impl Config {
    pub fn init() -> Self {
        Config {
            jwt_secret: std::env::var("JWT_SECRET").expect("JWT_SECRET not set"),
            supabase_project_url: std::env::var("SUPABASE_PROJECT_URL")
                .expect("SUPABASE_PROJECT_URL not set"),
            supabase_annon_key: std::env::var("SUPABASE_ANNON_KEY")
                .expect("SUPABASE_ANNON_KEY not set"),
            supabase_jwt_token: std::env::var("SUPABASE_JWT_TOKEN")
                .expect("SUPABASE_JWT_TOKEN not set"),
        }
    }
}
