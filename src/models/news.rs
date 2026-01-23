use serde::{Deserialize, Serialize};
use sqlx::prelude::FromRow;

#[derive(FromRow, Debug, Clone, Serialize, Deserialize)]
pub struct NewsCache {
    pub id: i32,
    pub source: String,
    pub title: String,
    pub description: Option<String>,
    pub url: String,
    pub image: Option<String>,
    pub published_at: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}