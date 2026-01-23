#[derive(Debug, Clone)]
pub struct Config {
    pub db_url: String,
    pub jwt_secret: String,
}

impl Config {
    pub fn init() -> Self {
        Config {
            db_url: std::env::var("DATABASE_URL").expect("DB_URL not set"),
            jwt_secret: std::env::var("JWT_SECRET").expect("JWT_SECRET not set"),
        }
    }
}
