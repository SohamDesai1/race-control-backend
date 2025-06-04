use chrono::Utc;
use jsonwebtoken::{encode, EncodingKey, Header};

use crate::models::jwt::Claims;

pub fn jwt_encode(sub:String,exp:Option<usize>,secret:&[u8]) -> String {
    let now = Utc::now();
                let iat = now.timestamp() as usize;
                let claims = Claims {
                    sub: sub,
                    iat: iat,
                    exp: exp,
                };
                let token = encode(
                    &Header::default(),
                    &claims,
                    &EncodingKey::from_secret(secret),
                )
                .unwrap();
            token
}