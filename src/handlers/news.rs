use std::sync::Arc;

use crate::utils::state::AppState;
use axum::{extract::State, response::IntoResponse, Json};
use http::StatusCode;
use serde_json::{json, Value};

pub async fn get_news(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let ttl_hours = 6;

    let cached = state
        .supabase
        .from("NewsCache")
        .select("*")
        .order("created_at.desc")
        .limit(10)
        .execute()
        .await;

    if let Ok(res) = cached {
        let body = res.text().await.unwrap();
        let cached_news: Vec<Value> = serde_json::from_str(&body).unwrap();

        if let Some(latest) = cached_news.first() {
            if let Some(created_at) = latest["created_at"].as_str() {
                let created = chrono::DateTime::parse_from_rfc3339(created_at)
                    .unwrap()
                    .with_timezone(&chrono::Utc);

                if chrono::Utc::now() - created < chrono::Duration::hours(ttl_hours) {
                    return (
                        StatusCode::OK,
                        Json(json!({ "source": "cache", "articles": cached_news })),
                    )
                        .into_response();
                }
            }
        }
    }

    let days_ago = chrono::Utc::now() - chrono::Duration::days(14);
    let rfc_date = days_ago.to_rfc3339();
    let since = rfc_date.split("T").next().unwrap_or("");

    let mut collected = Vec::new();

    let res1 = state
        .http_client
        .get(format!(
            "https://newsapi.org/v2/everything?q=F1 OR Formula1&language=en&from={}",
            since
        ))
        .header("User-Agent", "F1FantasyApp/1.0")
        .header("X-Api-Key", std::env::var("NEWS1").expect("NEWS1 not set"))
        .send()
        .await
        .unwrap();

    let json1: Value = serde_json::from_str(&res1.text().await.unwrap()).unwrap();

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

    let res2 = state
        .http_client
        .get(format!(
            "https://api.worldnewsapi.com/search-news?text=Formula1&language=en&earliest-publish-date={}",
            since
        ))
        .header("x-api-key", std::env::var("NEWS2").expect("NEWS2 not set"))
        .send()
        .await
        .unwrap();

    let json2: Value = serde_json::from_str(&res2.text().await.unwrap()).unwrap();

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

    for item in &collected {
        let _ = state
            .supabase
            .from("NewsCache")
            .insert(item.to_string())
            .execute()
            .await;
    }

    (
        StatusCode::OK,
        Json(json!({ "source": "api", "articles": collected })),
    )
        .into_response()
}
