use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

/// Наша кастомная структура ошибок.
#[derive(Debug)]
pub struct AppError {
    status_code: StatusCode,
    message: String,
}

impl AppError {
    pub fn new(status_code: StatusCode, message: &str) -> Self {
        Self {
            status_code,
            message: message.to_string(),
        }
    }
}

/// Преобразуем нашу ошибку в HTTP ответ.
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        (
            self.status_code,
            Json(json!({ "error": self.message })),
        )
            .into_response()
    }
}

/// Позволяем использовать `?` для ошибок `sqlx`.
impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        tracing::error!("Ошибка базы данных: {:?}", err);
        AppError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Произошла ошибка на сервере",
        )
    }
}

/// Позволяем использовать `?` для ошибок `jsonwebtoken`.
impl From<jsonwebtoken::errors::Error> for AppError {
    fn from(err: jsonwebtoken::errors::Error) -> Self {
        tracing::error!("Ошибка JWT: {:?}", err);
        AppError::new(StatusCode::INTERNAL_SERVER_ERROR, "Ошибка JWT")
    }
}

/// Позволяем использовать `?` для ошибок `bcrypt`.
impl From<bcrypt::BcryptError> for AppError {
    fn from(err: bcrypt::BcryptError) -> Self {
        tracing::error!("Ошибка Bcrypt: {:?}", err);
        AppError::new(StatusCode::INTERNAL_SERVER_ERROR, "Ошибка хеширования")
    }
}