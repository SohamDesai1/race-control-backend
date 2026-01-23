use jsonwebtoken::{EncodingKey, Header};

use crate::models::jwt::{Claims, RefreshClaims};

pub fn jwt_encode(email: String, secret: &str) -> String {
    let now = chrono::Utc::now().timestamp() as usize;
    let claims = Claims {
        sub: email,
        iat: now,
        exp: now + 15 * 60,
    };

    jsonwebtoken::encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .unwrap()
}

pub fn refresh_token_encode(email: String, secret: &str) -> String {
    let now = chrono::Utc::now().timestamp() as usize;
    let claims = RefreshClaims {
        sub: email,
        iat: now,
        exp: now + 1 * 24 * 60 * 60,
    };

    jsonwebtoken::encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .unwrap()
}
