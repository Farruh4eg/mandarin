use axum::{
    routing::{get, post},
    Router,
};
use sqlx::postgres::PgPoolOptions;
use std::env;
use std::net::SocketAddr;
use dotenv::dotenv;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

// Подключаем наши модули
mod auth;
mod handlers;
mod models;
mod errors;

// Подключаем тестовый модуль, только когда запускаем `cargo test`
#[cfg(test)]
mod tests;

// Структура для хранения состояния приложения (например, пула подключений к БД)
#[derive(Clone)]
pub struct AppState {
    db_pool: sqlx::PgPool,
}

// Логика создания роутера вынесена в отдельную функцию для тестируемости
pub fn app(app_state: AppState) -> Router {
    Router::new()
        // --- Роуты аутентификации ---
        .route("/api/register", post(handlers::register_handler))
        .route("/api/login", post(handlers::login_handler))
        .route("/api/refresh", post(handlers::refresh_handler))
        .route("/api/logout", post(handlers::logout_handler))
        .route("/api/protected", get(handlers::protected_handler))

        // --- Роуты для иероглифов ---
        .route("/api/hieroglyphs", get(handlers::get_hieroglyphs_handler))
        .route("/api/hieroglyphs", post(handlers::create_hieroglyph_handler))
        .route("/api/hieroglyphs/:id", get(handlers::get_hieroglyph_by_id_handler))

        // --- Роуты для прогресса пользователя ---
        .route("/api/progress/me", get(handlers::get_my_progress_handler))
        .route("/api/progress/learn", post(handlers::mark_learned_handler))

        // --- Роуты для достижений ---
        .route("/api/achievements", get(handlers::get_all_achievements_handler))
        .route("/api/achievements/me", get(handlers::get_my_achievements_handler))

        // --- Роуты для тестов ---
        .route("/api/tests", get(handlers::get_all_tests_handler))
        .route("/api/tests/:id", get(handlers::get_test_details_handler))
        .route("/api/tests/:id/submit", post(handlers::submit_test_handler))

        .with_state(app_state)
}