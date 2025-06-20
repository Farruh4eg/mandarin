#[cfg(test)]
mod tests {
    use crate::app;
    use crate::auth;
    use crate::models::{RegisterPayload, LoginPayload, AuthResponse, CreateHieroglyphPayload};
    use crate::AppState;
    use axum::{
        body::Body,
        http::{Request, StatusCode, Method},
    };
    use http_body_util::BodyExt;
    use sqlx::{postgres::PgPoolOptions, PgPool};
    use std::env;
    use tower::ServiceExt;

    /// Вспомогательная функция для создания пула соединений к БД из `.env`.
    async fn setup_test_pool() -> PgPool {
        dotenv::dotenv().ok();
        let db_url = env::var("DATABASE_URL").expect("DATABASE_URL должен быть установлен для тестов");
        PgPoolOptions::new()
            .connect(&db_url)
            .await
            .expect("Не удалось подключиться к тестовой базе данных")
    }

    #[tokio::test]
    async fn test_register_and_login() {
        let pool = setup_test_pool().await;
        let app_state = AppState { db_pool: pool.clone() };
        let app = app(app_state);
        let nickname = "testuser123".to_string();

        // 1. Тест успешной регистрации
        let register_payload = RegisterPayload {
            nickname: nickname.clone(),
            password: "testpassword".to_string(),
        };

        let request = Request::builder()
            .method(Method::POST)
            .uri("/api/register")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&register_payload).unwrap()))
            .unwrap();

        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);

        // 2. Тест регистрации с существующим никнеймом
        let request = Request::builder()
            .method(Method::POST)
            .uri("/api/register")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&register_payload).unwrap()))
            .unwrap();

        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::CONFLICT);


        // 3. Тест успешного логина
        let login_payload = LoginPayload {
            nickname: nickname.clone(),
            password: "testpassword".to_string(),
        };

        let request = Request::builder()
            .method(Method::POST)
            .uri("/api/login")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&login_payload).unwrap()))
            .unwrap();

        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // Проверяем, что в ответе есть токены
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let tokens: AuthResponse = serde_json::from_slice(&body).unwrap();
        assert!(!tokens.access_token.is_empty());
        assert!(!tokens.refresh_token.is_empty());

        // Очистка
        sqlx::query("DELETE FROM users WHERE nickname = $1").bind(nickname).execute(&pool).await.unwrap();
    }

    #[tokio::test]
    async fn test_protected_route() {
        let pool = setup_test_pool().await;
        let app_state = AppState { db_pool: pool.clone() };
        let app = app(app_state);
        let nickname = "test_prot_user".to_string();

        // Создаем пользователя и логинимся, чтобы получить токен
        sqlx::query("INSERT INTO users (nickname, password_hash, role) VALUES ($1, $2, 'user')")
            .bind(nickname.clone())
            .bind(auth::hash_password("password").unwrap())
            .execute(&pool)
            .await
            .unwrap();

        let login_payload = LoginPayload {
            nickname: nickname.clone(),
            password: "password".to_string(),
        };
        let request = Request::builder()
            .method(Method::POST)
            .uri("/api/login")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&login_payload).unwrap()))
            .unwrap();
        let response = app.clone().oneshot(request).await.unwrap();
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let tokens: AuthResponse = serde_json::from_slice(&body).unwrap();


        // 1. Тест доступа к защищенной ручке с валидным токеном
        let request = Request::builder()
            .method(Method::GET)
            .uri("/api/protected")
            .header("Authorization", format!("Bearer {}", tokens.access_token))
            .body(Body::empty())
            .unwrap();

        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // 2. Тест доступа без токена
        let request = Request::builder()
            .method(Method::GET)
            .uri("/api/protected")
            .body(Body::empty())
            .unwrap();

        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

        // Очистка
        sqlx::query("DELETE FROM users WHERE nickname = $1").bind(nickname).execute(&pool).await.unwrap();
    }

    #[tokio::test]
    async fn test_create_hieroglyph_permission() {
        let pool = setup_test_pool().await;
        let app_state = AppState { db_pool: pool.clone() };
        let app = app(app_state);
        let admin_nick = "admin_test_h".to_string();
        let user_nick = "user_test_h".to_string();

        // Создаем админа и обычного пользователя
        sqlx::query("INSERT INTO users (nickname, password_hash, role) VALUES ($1, $2, 'admin'), ($3, $4, 'user')")
            .bind(admin_nick.clone())
            .bind(auth::hash_password("password").unwrap())
            .bind(user_nick.clone())
            .bind(auth::hash_password("password").unwrap())
            .execute(&pool)
            .await
            .unwrap();

        // Получаем токен для админа
        let admin_tokens: AuthResponse = serde_json::from_slice(
            &app.clone().oneshot(Request::builder()
                .method(Method::POST)
                .uri("/api/login")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&LoginPayload { nickname: admin_nick.clone(), password: "password".to_string() }).unwrap()))
                .unwrap()
            ).await.unwrap().into_body().collect().await.unwrap().to_bytes()
        ).unwrap();

        // Получаем токен для юзера
        let user_tokens: AuthResponse = serde_json::from_slice(
            &app.clone().oneshot(Request::builder()
                .method(Method::POST)
                .uri("/api/login")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&LoginPayload { nickname: user_nick.clone(), password: "password".to_string() }).unwrap()))
                .unwrap()
            ).await.unwrap().into_body().collect().await.unwrap().to_bytes()
        ).unwrap();

        let hieroglyph_payload = CreateHieroglyphPayload {
            character: "测".to_string(),
            pinyin: "cè".to_string(),
            translation: "тест".to_string(),
            example: Some("这是一个测试".to_string()),
        };

        // 1. Тест создания иероглифа админом (успех)
        let request = Request::builder()
            .method(Method::POST)
            .uri("/api/hieroglyphs")
            .header("Authorization", format!("Bearer {}", admin_tokens.access_token))
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&hieroglyph_payload).unwrap()))
            .unwrap();

        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);

        // 2. Тест создания иероглифа юзером (провал)
        let request = Request::builder()
            .method(Method::POST)
            .uri("/api/hieroglyphs")
            .header("Authorization", format!("Bearer {}", user_tokens.access_token))
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&hieroglyph_payload).unwrap()))
            .unwrap();

        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);

        // Очистка
        sqlx::query("DELETE FROM users WHERE nickname = $1 OR nickname = $2")
            .bind(admin_nick)
            .bind(user_nick)
            .execute(&pool).await.unwrap();
        sqlx::query("DELETE FROM hieroglyphs WHERE character = '测'").execute(&pool).await.unwrap();
    }
}