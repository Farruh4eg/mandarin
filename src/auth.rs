use axum::{
    async_trait,
    extract::{FromRequestParts},
    http::{request::Parts},
    response::{IntoResponse, Response},
};
use axum_extra::headers::{authorization::Bearer, Authorization};
use axum_extra::TypedHeader;
use bcrypt::{hash, verify, DEFAULT_COST};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use rand::RngCore;
use sqlx::PgPool;
use std::env;

use crate::models::{AuthResponse, Claims, User};
use crate::errors::AppError;
use axum::http::StatusCode;

// --- Константы для времени жизни токенов ---
const ACCESS_TOKEN_EXPIRATION_MINUTES: i64 = 15;
const REFRESH_TOKEN_EXPIRATION_DAYS: i64 = 30;

/// Хеширует пароль с использованием bcrypt.
pub fn hash_password(password: &str) -> Result<String, AppError> {
    hash(password, DEFAULT_COST).map_err(|_| {
        AppError::new(StatusCode::INTERNAL_SERVER_ERROR, "Не удалось хешировать пароль")
    })
}

/// Проверяет пароль на соответствие хешу.
pub fn verify_password(password: &str, hash: &str) -> Result<bool, AppError> {
    verify(password, hash).map_err(|_| {
        AppError::new(StatusCode::INTERNAL_SERVER_ERROR, "Ошибка при проверке пароля")
    })
}

/// Генерирует пару access и refresh токенов.
pub async fn generate_tokens(user_id: &i32, pool: &PgPool) -> Result<AuthResponse, AppError> {
    // Получаем пользователя целиком, чтобы иметь доступ к роли.
    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_one(pool)
        .await?;

    // 1. Создание Access Token
    let now = Utc::now();
    let access_token_exp = (now + Duration::minutes(ACCESS_TOKEN_EXPIRATION_MINUTES)).timestamp();
    let access_claims = Claims {
        exp: access_token_exp as usize,
        iat: now.timestamp() as usize,
        user_id: *user_id,
        role: user.role,
    };
    let jwt_secret = env::var("JWT_SECRET").expect("JWT_SECRET должен быть установлен");
    let access_token = encode(
        &Header::default(),
        &access_claims,
        &EncodingKey::from_secret(jwt_secret.as_ref()),
    )?;

    // 2. Создание Refresh Token
    let mut refresh_token_bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut refresh_token_bytes);
    let refresh_token = hex::encode(refresh_token_bytes);
    let refresh_token_exp = now + Duration::days(REFRESH_TOKEN_EXPIRATION_DAYS);

    // 3. Сохранение Refresh Token в БД
    sqlx::query("INSERT INTO refresh_sessions (user_id, refresh_token, expires_at) VALUES ($1, $2, $3)")
        .bind(user_id)
        .bind(&refresh_token)
        .bind(refresh_token_exp)
        .execute(pool)
        .await?;

    Ok(AuthResponse { access_token, refresh_token })
}

/// Обновляет access token, используя refresh token (без транзакции).
pub async fn refresh_access_token(refresh_token: &str, pool: &PgPool) -> Result<AuthResponse, AppError> {
    // 1. Найти сессию по refresh token в БД
    let session: (i32, chrono::DateTime<Utc>) = sqlx::query_as(
        "SELECT user_id, expires_at FROM refresh_sessions WHERE refresh_token = $1",
    )
        .bind(refresh_token)
        .fetch_optional(pool) // Используем пул напрямую
        .await?
        .ok_or_else(|| AppError::new(StatusCode::UNAUTHORIZED, "Невалидный refresh токен"))?;

    let (user_id, expires_at) = session;

    // 2. Проверить, не истек ли срок действия
    if Utc::now() > expires_at {
        // Удаляем просроченный токен из БД
        sqlx::query("DELETE FROM refresh_sessions WHERE refresh_token = $1").bind(refresh_token).execute(pool).await?;
        return Err(AppError::new(StatusCode::UNAUTHORIZED, "Сессия истекла"));
    }

    // 3. Удалить старый refresh token (рискованная часть, но так было запрошено)
    sqlx::query("DELETE FROM refresh_sessions WHERE refresh_token = $1")
        .bind(refresh_token)
        .execute(pool) // Используем пул напрямую
        .await?;

    // 4. Сгенерировать новую пару токенов (ротация)
    let tokens = generate_tokens(&user_id, pool).await?;

    Ok(tokens)
}

// Реализация экстрактора для получения claims из токена в защищенных хендлерах
#[async_trait]
impl<S> FromRequestParts<S> for Claims
where
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let TypedHeader(Authorization(bearer)) =
            TypedHeader::<Authorization<Bearer>>::from_request_parts(parts, _state)
                .await
                .map_err(|_| AppError::new(StatusCode::UNAUTHORIZED, "Требуется токен авторизации").into_response())?;

        let jwt_secret = env::var("JWT_SECRET").expect("JWT_SECRET должен быть установлен");
        let token_data = decode::<Claims>(
            bearer.token(),
            &DecodingKey::from_secret(jwt_secret.as_ref()),
            &Validation::default(),
        )
            .map_err(|e| {
                let error_message = format!("Невалидный токен: {}", e);
                AppError::new(StatusCode::UNAUTHORIZED, &error_message).into_response()
            })?;

        Ok(token_data.claims)
    }
}