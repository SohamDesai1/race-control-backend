use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct CreateUser {
    pub name: String,
    pub username: String,
}
