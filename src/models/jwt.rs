use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Claims {
    pub iss: String,
    pub sub: String,
    // pub aud: String,
    pub exp: usize,
    pub iat: usize,
    pub email: String,
    pub phone: Option<String>,
    pub app_metadata: AppMetadata,
    pub user_metadata: UserMetadata,
    pub role: String,
    pub aal: String,
    pub amr: Vec<AuthMethod>,
    pub session_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AppMetadata {
    pub provider: String,
    pub providers: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UserMetadata {
    pub email: String,
    pub email_verified: bool,
    pub phone_verified: bool,
    pub sub: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuthMethod {
    pub method: String,
    pub timestamp: usize,
}
