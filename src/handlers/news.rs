use std::sync::Arc;

use crate::utils::state::AppState;
use axum::{extract::State, response::IntoResponse, Json};
use http::StatusCode;
use serde_json::{from_str, json, Value};

pub async fn get_news(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let five_days_ago = chrono::Utc::now() - chrono::Duration::days(14);
    let rfc_date = five_days_ago.to_rfc3339();
    let date_str = rfc_date.split("T").next().unwrap_or("");
    let response1 = state
        .http_client
        .get(format!(
            "https://newsapi.org/v2/everything?q=F1 OR Formula1&apiKey={}&language=en&from={}",
            std::env::var("NEWS1").expect("NEWS1 not set"),
            date_str
        ))
        .header("User-Agent", "F1FantasyApp/1.0")
        .send()
        .await
        .unwrap();
    let body1 = response1.text().await.unwrap();
    let json1: Value = from_str(&body1).unwrap();
    let title1 = &json1["articles"][0]["title"];
    let desc1 = &json1["articles"][0]["description"];
    let url1 = &json1["articles"][0]["url"];
    let image1 = &json1["articles"][0]["urlToImage"];
    let response2 = state
        .http_client
        .get(format!(
            "https://api.worldnewsapi.com/search-news?text=Formula1&language=en&earliest-publish-date={}",
            date_str

        )).header("x-api-key", std::env::var("NEWS2").expect("NEWS2 not set"))
        .send()
        .await
        .unwrap();
    let body2 = response2.text().await.unwrap();
    let json2: Value = from_str(&body2).unwrap();
    let title2 = &json2["news"][0]["title"];
    let desc2 = &json2["news"][0]["text"];
    let url2 = &json2["news"][0]["url"];
    let image2 = &json2["news"][0]["image"];
    let combined = json!({
        "articles": [
            {
                "title": title2,
                "description": desc2,
                "url": url2,
                "image": image2
            }, 
            {
                "title": title1,
                "description": desc1,
                "url": url1,
                "image": image1
            },
        ]
    });
    (StatusCode::OK, Json(combined)).into_response()
}
