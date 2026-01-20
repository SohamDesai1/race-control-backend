use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub iat: usize,
    pub exp: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RefreshClaims {
    pub sub: String,
    pub iat: usize,
    pub exp: usize,
}
