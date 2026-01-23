use axum::{
    extract::{Request, State},
    middleware::Next,
    response::IntoResponse,
};
use http::{header, StatusCode};
use jsonwebtoken::{decode, DecodingKey, Validation};
use std::sync::Arc;

use crate::{
    models::{error::Error, jwt::Claims},
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
        .ok_or_else(|| Error::new(StatusCode::UNAUTHORIZED, "Missing authorization header"))?;

    let secret = &state.config.jwt_secret;

    let claims = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )
    .map(|data| data.claims)
    .map_err(|e| Error::new(StatusCode::UNAUTHORIZED, &format!("Invalid token: {}", e)))?;

    req.extensions_mut().insert(claims);

    Ok(next.run(req).await)
}
