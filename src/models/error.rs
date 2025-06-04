use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::response::Response;
use axum::Json;
use serde_json::json;
use serde_json::Value;


#[derive(Debug)]
pub struct Error {
    pub code: StatusCode,
    pub body: Json<Value>,
}

impl Error {
    pub fn new(code: StatusCode, message: &str) -> Self {
        Self {
            code,
            body: Json(json!({"message": message})),
        }
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        (self.code, self.body).into_response()
    }
}

impl From<(StatusCode, &str)> for Error {
    fn from((code, msg): (StatusCode, &str)) -> Self {
        Self::new(code, msg)
    }
}

impl From<jsonwebtoken::errors::Error> for Error {
    fn from(error: jsonwebtoken::errors::Error) -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, &error.to_string())
    }
}

impl From<argon2::password_hash::errors::Error> for Error {
    fn from(error: argon2::password_hash::errors::Error) -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, &error.to_string())
    }
}