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
    State(state): State<Arc<AppState>>, // No longer used
    mut req: Request,
    next: Next,
) -> Result<impl IntoResponse, Error> {
    let token = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .ok_or((StatusCode::UNAUTHORIZED, "Missing Bearer token"))?;

    let secret = state.config.jwt_secret.clone();

    let mut validation = Validation::new(jsonwebtoken::Algorithm::HS256);
    validation.set_audience(&["authenticated"]);

    let decoded = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_ref()),
        &validation,
    )?;

    req.extensions_mut().insert(decoded.claims);

    Ok(next.run(req).await)
}
