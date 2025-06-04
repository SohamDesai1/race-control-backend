use axum::{
    extract::{Request, State},
    middleware::Next,
    response::IntoResponse,
};
use http::{header, StatusCode};
use jsonwebtoken::{decode, DecodingKey, Validation};
use serde_json::Value;
use std::sync::Arc;

use crate::{
    models::{error::Error, jwt::Claims, user::User},
    utils::state::AppState,
};

pub async fn auth_middleware(
    State(state): State<Arc<AppState>>,
    mut req: Request,
    next: Next,
) -> Result<impl IntoResponse, Error> {
    let token = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .ok_or((StatusCode::UNAUTHORIZED, "Missing Bearer token"))?;

    let secret = std::env::var("JWT_SECRET")
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "JWT_SECRET not set"))?;

    let decoded = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )?;

    let email = decoded.claims.sub;

    let response = state
        .supabase
        .from("Users")
        .eq("email", &email)
        .select("*")
        .execute()
        .await
        .map_err(|e| Error::new(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))?;

    let body = response.text().await.map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to read Supabase response",
        )
    })?;

    let users: Vec<Value> = serde_json::from_str(&body).map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Invalid JSON from Supabase",
        )
    })?;

    if users.is_empty() {
        return Err((StatusCode::UNAUTHORIZED, "No matching user").into());
    }

    let user: User = serde_json::from_value(users[0].clone())
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Cannot deserialize user"))?;

    req.extensions_mut().insert(user);

    Ok(next.run(req).await)
}
