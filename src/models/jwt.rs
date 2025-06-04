use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub iat: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exp: Option<usize>,
}