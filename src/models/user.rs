use chrono::DateTime;
use serde::{Deserialize, Serialize};
use sqlx::prelude::FromRow;

#[derive(FromRow,Deserialize, Serialize, Clone, Debug)]
pub struct User {
    pub email: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<DateTime<chrono::Utc>>,
    //option for none serde
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dob: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gender: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hashed_password: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth_provider: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_profile_complete: Option<bool>,
}
