use axum::{extract::{State, Path}, http::StatusCode, Json, response::IntoResponse};

use crate::auth;
use crate::models::{
    RegisterPayload, LoginPayload, AuthResponse, RefreshPayload, Claims, User,
    Hieroglyph, CreateHieroglyphPayload, UserRole, UserProgress, MarkLearnedPayload,
    Achievement, UserAchievementDetails, Test, TestItem, TestDetails, TestSubmissionPayload, TestResultResponse
};
use crate::errors::AppError;
use crate::AppState;


// --- Обработчики аутентификации ---

/// Обработчик регистрации нового пользователя.
#[axum::debug_handler]
pub async fn register_handler(
    State(state): State<AppState>,
    Json(payload): Json<RegisterPayload>,
) -> Result<impl IntoResponse, AppError> {
    // Проверяем, существует ли пользователь с таким никнеймом
    let existing_user = sqlx::query("SELECT id FROM users WHERE nickname = $1")
        .bind(&payload.nickname)
        .fetch_optional(&state.db_pool)
        .await?;

    if existing_user.is_some() {
        return Err(AppError::new(StatusCode::CONFLICT, "Пользователь с таким никнеймом уже существует"));
    }

    // Хешируем пароль
    let hashed_password = auth::hash_password(&payload.password)?;

    // Сохраняем нового пользователя в БД
    sqlx::query("INSERT INTO users (nickname, password_hash) VALUES ($1, $2)")
        .bind(&payload.nickname)
        .bind(&hashed_password)
        .execute(&state.db_pool)
        .await?;

    Ok((StatusCode::CREATED, "Пользователь успешно зарегистрирован"))
}

/// Обработчик входа пользователя.
#[axum::debug_handler]
pub async fn login_handler(
    State(state): State<AppState>,
    Json(payload): Json<LoginPayload>,
) -> Result<Json<AuthResponse>, AppError> {
    // Ищем пользователя по никнейму
    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE nickname = $1")
        .bind(&payload.nickname)
        .fetch_optional(&state.db_pool)
        .await?
        .ok_or_else(|| AppError::new(StatusCode::UNAUTHORIZED, "Неверный никнейм или пароль"))?;

    // Проверяем пароль
    if !auth::verify_password(&payload.password, &user.password_hash)? {
        return Err(AppError::new(StatusCode::UNAUTHORIZED, "Неверный никнейм или пароль"));
    }

    // Генерируем access и refresh токены, используя пул соединений
    let tokens = auth::generate_tokens(&user.id, &state.db_pool).await?;

    Ok(Json(tokens))
}

/// Обработчик обновления токенов.
pub async fn refresh_handler(
    State(state): State<AppState>,
    Json(payload): Json<RefreshPayload>,
) -> Result<Json<AuthResponse>, AppError> {
    let tokens = auth::refresh_access_token(&payload.refresh_token, &state.db_pool).await?;
    Ok(Json(tokens))
}

/// Обработчик выхода из системы.
pub async fn logout_handler(
    State(state): State<AppState>,
    Json(payload): Json<RefreshPayload>,
) -> Result<impl IntoResponse, AppError> {
    // Удаляем refresh токен из базы
    sqlx::query("DELETE FROM refresh_sessions WHERE refresh_token = $1")
        .bind(&payload.refresh_token)
        .execute(&state.db_pool)
        .await?;

    Ok((StatusCode::OK, "Вы успешно вышли из системы"))
}

/// Пример защищенного обработчика.
pub async fn protected_handler(claims: Claims) -> String {
    format!("Привет, user_id: {}. Твоя роль: {}. Это защищенный ресурс.", claims.user_id, claims.role)
}

// --- Обработчики для иероглифов ---

/// Создание нового иероглифа (только для админов).
pub async fn create_hieroglyph_handler(
    State(state): State<AppState>,
    claims: Claims, // Экстрактор для проверки аутентификации и роли
    Json(payload): Json<CreateHieroglyphPayload>,
) -> Result<impl IntoResponse, AppError> {
    // Проверяем, что у пользователя роль админа
    if claims.role != UserRole::Admin {
        return Err(AppError::new(StatusCode::FORBIDDEN, "Доступ запрещен"));
    }

    // Вставляем новый иероглиф в базу данных
    let hieroglyph = sqlx::query_as::<_, Hieroglyph>(
        "INSERT INTO hieroglyphs (character, pinyin, translation, example) VALUES ($1, $2, $3, $4) RETURNING *",
    )
        .bind(payload.character)
        .bind(payload.pinyin)
        .bind(payload.translation)
        .bind(payload.example)
        .fetch_one(&state.db_pool)
        .await?;

    Ok((StatusCode::CREATED, Json(hieroglyph)))
}

/// Получение списка всех иероглифов.
pub async fn get_hieroglyphs_handler(
    State(state): State<AppState>,
) -> Result<Json<Vec<Hieroglyph>>, AppError> {
    let hieroglyphs = sqlx::query_as::<_, Hieroglyph>("SELECT * FROM hieroglyphs")
        .fetch_all(&state.db_pool)
        .await?;

    Ok(Json(hieroglyphs))
}

/// Получение одного иероглифа по ID.
pub async fn get_hieroglyph_by_id_handler(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<Hieroglyph>, AppError> {
    let hieroglyph = sqlx::query_as::<_, Hieroglyph>("SELECT * FROM hieroglyphs WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.db_pool)
        .await?
        .ok_or_else(|| AppError::new(StatusCode::NOT_FOUND, "Иероглиф не найден"))?;

    Ok(Json(hieroglyph))
}

// --- Обработчики прогресса пользователя ---

/// Отметить элемент контента как выученный.
pub async fn mark_learned_handler(
    State(state): State<AppState>,
    claims: Claims,
    Json(payload): Json<MarkLearnedPayload>,
) -> Result<impl IntoResponse, AppError> {
    // Используем INSERT ... ON CONFLICT DO UPDATE для атомарного добавления/обновления прогресса
    // Это гарантирует, что не будет дубликатов, и триггер сработает корректно
    let query = "
        INSERT INTO user_progress (user_id, content_type, content_id, is_learned, learned_at)
        VALUES ($1, $2, $3, TRUE, NOW())
        ON CONFLICT (user_id, content_type, content_id) DO UPDATE
        SET is_learned = TRUE, learned_at = NOW()
    ";

    sqlx::query(query)
        .bind(claims.user_id)
        .bind(payload.content_type)
        .bind(payload.content_id)
        .execute(&state.db_pool)
        .await?;

    Ok(StatusCode::OK)
}

/// Получить прогресс текущего пользователя.
pub async fn get_my_progress_handler(
    State(state): State<AppState>,
    claims: Claims,
) -> Result<Json<Vec<UserProgress>>, AppError> {
    let progress = sqlx::query_as::<_, UserProgress>("SELECT * FROM user_progress WHERE user_id = $1")
        .bind(claims.user_id)
        .fetch_all(&state.db_pool)
        .await?;

    Ok(Json(progress))
}

// --- Обработчики достижений ---

/// Получить список всех возможных достижений
pub async fn get_all_achievements_handler(
    State(state): State<AppState>,
) -> Result<Json<Vec<Achievement>>, AppError> {
    let achievements = sqlx::query_as::<_, Achievement>("SELECT * FROM achievements")
        .fetch_all(&state.db_pool)
        .await?;

    Ok(Json(achievements))
}

/// Получить список достижений текущего пользователя
pub async fn get_my_achievements_handler(
    State(state): State<AppState>,
    claims: Claims,
) -> Result<Json<Vec<UserAchievementDetails>>, AppError> {
    let my_achievements = sqlx::query_as::<_, UserAchievementDetails>(
        "SELECT a.id, a.name, a.description, a.icon, ua.achieved_at
         FROM achievements a
         JOIN user_achievements ua ON a.id = ua.achievement_id
         WHERE ua.user_id = $1"
    )
        .bind(claims.user_id)
        .fetch_all(&state.db_pool)
        .await?;

    Ok(Json(my_achievements))
}

// --- Обработчики тестов ---

/// Получить список всех тестов
pub async fn get_all_tests_handler(
    State(state): State<AppState>,
) -> Result<Json<Vec<Test>>, AppError> {
    let tests = sqlx::query_as::<_, Test>("SELECT * FROM tests")
        .fetch_all(&state.db_pool)
        .await?;
    Ok(Json(tests))
}

/// Получить детальную информацию о тесте, включая вопросы
pub async fn get_test_details_handler(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<TestDetails>, AppError> {
    // Получаем основную информацию о тесте
    let test = sqlx::query_as::<_, Test>("SELECT * FROM tests WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.db_pool)
        .await?
        .ok_or_else(|| AppError::new(StatusCode::NOT_FOUND, "Тест не найден"))?;

    // Получаем вопросы к этому тесту
    // Важно: не отдаем `correct_answer` клиенту
    let questions = sqlx::query_as::<_, TestItem>(
        "SELECT id, test_id, question, options FROM test_items WHERE test_id = $1",
    )
        .bind(id)
        .fetch_all(&state.db_pool)
        .await?;

    let test_details = TestDetails {
        id: test.id,
        name: test.name,
        description: test.description,
        created_at: test.created_at,
        questions,
    };

    Ok(Json(test_details))
}

/// Принять ответы на тест, проверить и сохранить результат
pub async fn submit_test_handler(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    claims: Claims,
    Json(payload): Json<TestSubmissionPayload>,
) -> Result<Json<TestResultResponse>, AppError> {
    // Получаем правильные ответы из БД
    let correct_answers = sqlx::query_as::<_, (i32, String)>(
        "SELECT id, correct_answer FROM test_items WHERE test_id = $1"
    )
        .bind(id)
        .fetch_all(&state.db_pool)
        .await?;

    let total_questions = correct_answers.len();
    if total_questions == 0 {
        return Err(AppError::new(StatusCode::NOT_FOUND, "Тест не найден или не содержит вопросов"));
    }

    // Считаем правильные ответы
    let mut score = 0;
    for (question_id, correct_answer) in correct_answers {
        if let Some(user_answer) = payload.answers.iter().find(|a| a.question_id == question_id) {
            if user_answer.answer == correct_answer {
                score += 1;
            }
        }
    }

    // Сохраняем результат в БД
    sqlx::query("INSERT INTO test_results (user_id, test_id, score) VALUES ($1, $2, $3)")
        .bind(claims.user_id)
        .bind(id)
        .bind(score as i32)
        .execute(&state.db_pool)
        .await?;

    let response = TestResultResponse {
        score,
        total_questions,
    };

    Ok(Json(response))
}