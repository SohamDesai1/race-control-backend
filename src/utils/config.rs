#[derive(Debug, Clone)]
pub struct Config {
    pub db_url: String,
    pub jwt_secret: String,
}

impl Config {
    pub fn init() -> Self {
        let db_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| {
                eprintln!("ERROR: DATABASE_URL environment variable is not set");
                std::process::exit(1);
            });
        
        let jwt_secret = std::env::var("JWT_SECRET")
            .unwrap_or_else(|_| {
                eprintln!("ERROR: JWT_SECRET environment variable is not set");
                std::process::exit(1);
            });

        Config {
            db_url,
            jwt_secret,
        }
    }
}
