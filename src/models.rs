use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt;
use chrono::{DateTime, Utc};

// --- Модели для базы данных ---

/// Rust-эквивалент для `content_type_enum` из PostgreSQL.
#[derive(Debug, Clone, sqlx::Type, Serialize, Deserialize, PartialEq)]
#[sqlx(type_name = "content_type_enum", rename_all = "snake_case")]
pub enum ContentType {
    Hieroglyph,
    Word,
    Phrase,
    GrammarRule,
    Lesson,
}

/// Rust-эквивалент для `user_role_enum` из PostgreSQL.
#[derive(Debug, Clone, sqlx::Type, Serialize, Deserialize, PartialEq)]
#[sqlx(type_name = "user_role_enum", rename_all = "lowercase")]
pub enum UserRole {
    User,
    Admin,
}

// Реализуем Display для удобного вывода роли в текстовом виде.
impl fmt::Display for UserRole {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            UserRole::User => write!(f, "user"),
            UserRole::Admin => write!(f, "admin"),
        }
    }
}


#[derive(sqlx::FromRow, Debug, Serialize, Deserialize)]
pub struct User {
    pub id: i32,
    pub nickname: String,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub role: UserRole,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Hieroglyph {
    pub id: i32,
    pub character: String,
    pub pinyin: String,
    pub translation: String,
    pub example: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct UserProgress {
    pub id: i32,
    pub user_id: i32,
    pub content_type: ContentType,
    pub content_id: i32,
    pub is_learned: bool,
    pub learned_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Achievement {
    pub id: i32,
    pub name: String,
    pub description: Option<String>,
    pub criteria: Value, // JSONB
    pub icon: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct UserAchievementDetails {
    pub id: i32,
    pub name: String,
    pub description: Option<String>,
    pub icon: Option<String>,
    pub achieved_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Test {
    pub id: i32,
    pub name: String,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct TestItem {
    pub id: i32,
    pub test_id: i32,
    pub question: String,
    pub options: Option<Value>, // JSONB
}

// --- Структуры для request/response ---

#[derive(Debug, Serialize, Deserialize)]
pub struct TestDetails {
    pub id: i32,
    pub name: String,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub questions: Vec<TestItem>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AnswerPayload {
    pub question_id: i32,
    pub answer: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TestSubmissionPayload {
    pub answers: Vec<AnswerPayload>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TestResultResponse {
    pub score: usize,
    pub total_questions: usize,
}


/// Полезная нагрузка для регистрации.
#[derive(Debug, Deserialize, Serialize)]
pub struct RegisterPayload {
    pub nickname: String,
    pub password: String,
}

/// Полезная нагрузка для логина.
#[derive(Debug, Deserialize, Serialize)]
pub struct LoginPayload {
    pub nickname: String,
    pub password: String,
}

/// Полезная нагрузка для обновления токена.
#[derive(Debug, Deserialize, Serialize)]
pub struct RefreshPayload {
    pub refresh_token: String,
}

/// Полезная нагрузка для создания иероглифа
#[derive(Debug, Deserialize, Serialize)]
pub struct CreateHieroglyphPayload {
    pub character: String,
    pub pinyin: String,
    pub translation: String,
    pub example: Option<String>,
}

/// Полезная нагрузка для отметки контента как выученного.
#[derive(Debug, Deserialize, Serialize)]
pub struct MarkLearnedPayload {
    pub content_type: ContentType,
    pub content_id: i32,
}


/// Ответ с токенами.
#[derive(Debug, Serialize, Deserialize)]
pub struct AuthResponse {
    pub access_token: String,
    pub refresh_token: String,
}

/// Структура "claims" для JWT.
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub exp: usize,
    pub iat: usize,
    pub user_id: i32,
    pub role: UserRole,
}

// --- Application State ---

/// Global application state shared across handlers.
#[derive(Debug, Clone)]
pub struct AppState {
    pub db_pool: sqlx::PgPool,
}