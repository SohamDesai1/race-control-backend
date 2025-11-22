use axum::{
    extract::{Request, State},
    middleware::Next,
    response::IntoResponse,
};
use http::{header, StatusCode};
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
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
        .ok_or((StatusCode::UNAUTHORIZED, "Missing Bearer token"))?;

    let jwk_x = state.config.jwk_x.clone();
    let jwk_y = state.config.jwk_y.clone();

    let decoding_key = DecodingKey::from_ec_components(&jwk_x, &jwk_y).map_err(|e| {
        Error::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("Invalid JWK: {}", e),
        )
    })?;

    let mut validation = Validation::new(Algorithm::ES256);
    validation.set_audience(&["authenticated"]);

    let decoded = decode::<Claims>(token, &decoding_key, &validation).map_err(|e| {
        Error::new(
            StatusCode::UNAUTHORIZED,
            &format!("Token validation failed: {}", e),
        )
    })?;

    req.extensions_mut().insert(decoded.claims);

    Ok(next.run(req).await)
}
