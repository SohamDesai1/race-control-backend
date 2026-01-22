use std::sync::Arc;

use crate::{models::news::NewsCache, utils::state::AppState};
use axum::{extract::State, response::IntoResponse, Json};
use http::StatusCode;
use serde_json::{json, Value};

pub async fn get_news(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let ttl_hours = 6;

    // Check cache first
    let cached = sqlx::query_as::<_, NewsCache>(
        r#"SELECT * FROM "NewsCache" ORDER BY created_at DESC LIMIT 10"#,
    )
    .fetch_all(&state.db_pool)
    .await;

    if let Ok(cached_news) = cached {
        if let Some(latest) = cached_news.first() {
            let age = chrono::Utc::now().signed_duration_since(latest.created_at);
            if age < chrono::Duration::hours(ttl_hours) {
                return (
                    StatusCode::OK,
                    Json(json!({ "source": "cache", "articles": cached_news })),
                )
                    .into_response();
            }
        }
    }

    // Fetch fresh news
    let days_ago = chrono::Utc::now() - chrono::Duration::days(14);
    let since = days_ago.format("%Y-%m-%d").to_string();

    let mut collected = Vec::new();

    // Fetch from NewsAPI
    match state
        .http_client
        .get(format!(
            "https://newsapi.org/v2/everything?q=F1 OR Formula1&language=en&from={}",
            since
        ))
        .header("User-Agent", "F1FantasyApp/1.0")
        .header("X-Api-Key", std::env::var("NEWS1").unwrap_or_default())
        .send()
        .await
    {
        Ok(res) => {
            if let Ok(text) = res.text().await {
                if let Ok(json1) = serde_json::from_str::<Value>(&text) {
                    if let Some(article) = json1["articles"].get(0) {
                        collected.push(json!({
                            "source": "newsapi",
                            "title": article["title"],
                            "description": article["description"],
                            "url": article["url"],
                            "image": article["urlToImage"],
                            "published_at": article["publishedAt"]
                        }));
                    }
                }
            }
        }
        Err(e) => tracing::warn!("Failed to fetch from NewsAPI: {:?}", e),
    }

    // Fetch from WorldNewsAPI
    match state
        .http_client
        .get(format!(
            "https://api.worldnewsapi.com/search-news?text=Formula1&language=en&earliest-publish-date={}",
            since
        ))
        .header("x-api-key", std::env::var("NEWS2").unwrap_or_default())
        .send()
        .await
    {
        Ok(res) => {
            if let Ok(text) = res.text().await {
                if let Ok(json2) = serde_json::from_str::<Value>(&text) {
                    if let Some(article) = json2["news"].get(0) {
                        collected.push(json!({
                            "source": "worldnewsapi",
                            "title": article["title"],
                            "description": article["summary"],
                            "url": article["url"],
                            "image": article["image"],
                            "published_at": article["publish_date"]
                        }));
                    }
                }
            }
        }
        Err(e) => tracing::warn!("Failed to fetch from WorldNewsAPI: {:?}", e),
    }

    // Cache the results
    for item in &collected {
        let _ = sqlx::query(
            r#"
            INSERT INTO "NewsCache" (source, title, description, url, image, published_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            "#,
        )
        .bind(item["source"].as_str().unwrap_or(""))
        .bind(item["title"].as_str().unwrap_or(""))
        .bind(item["description"].as_str())
        .bind(item["url"].as_str().unwrap_or(""))
        .bind(item["image"].as_str())
        .bind(item["published_at"].as_str())
        .execute(&state.db_pool)
        .await;
    }

    (
        StatusCode::OK,
        Json(json!({ "source": "api", "articles": collected })),
    )
        .into_response()
}
