// main.rs

#![allow(non_snake_case)]

mod models;
mod handlers;

use axum::{
    routing::{post, Router},
    Extension,
};
use dotenvy::dotenv;
use rdev::display_size;
use slint::{ComponentHandle, LogicalPosition, LogicalSize, SharedString};
use sqlx::postgres::PgPoolOptions;
use std::cell::RefCell;
use reqwest::Client;
use crate::models::{LoginPayload, RegisterPayload, AuthResponse}; // Assuming these are public
use serde_json::Value; // For parsing generic error messages
use std::net::SocketAddr;
use std::rc::Rc;
use tokio::net::TcpListener;
use std::sync::Arc;

// Assuming AppState is in models.rs and handlers are in handlers.rs
// If not, these paths might need adjustment.
use crate::models::AppState;
use crate::handlers::{login_handler, register_handler};


slint::include_modules!();

async fn run_axum_server() {
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let pool = match PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
    {
        Ok(pool) => {
            println!("Successfully connected to the database");
            pool
        }
        Err(err) => {
            eprintln!("Failed to connect to the database: {:?}", err);
            std::process::exit(1);
        }
    };

    let app_state = Arc::new(AppState { db_pool: pool });

    let router = Router::new() // Renamed app to router for clarity with axum::serve call
        .route("/register", post(register_handler))
        .route("/login", post(login_handler))
        .layer(Extension(app_state));

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));

    let listener = match TcpListener::bind(addr).await {
        Ok(listener) => {
            println!("Axum server listening on {}", addr);
            listener
        }
        Err(e) => {
            eprintln!("Failed to bind TCP listener on {}: {}", addr, e);
            // Consider exiting or a more robust error handling strategy if the server can't start
            return;
        }
    };

    if let Err(e) = axum::serve(listener, router).await { // Use the renamed router variable
        eprintln!("Axum server error: {}", e);
    }
}

#[tokio::main]
async fn main() {
    dotenv().ok(); // Load .env file

    // Spawn the Axum server in a separate Tokio task
    tokio::spawn(run_axum_server());

    let authenticationWindow = authentication::new().unwrap();
    let mainAppWindowHandle: Rc<RefCell<Option<mainApp>>> = Rc::new(RefCell::new(None));
    let weakAuthentication = authenticationWindow.as_weak();
    let mainAppWindowHandleClone = mainAppWindowHandle.clone();

    authenticationWindow.on_authenticate(move |nickname, password| {
        let weak_auth_clone = weakAuthentication.clone();
        let main_app_handle_clone = mainAppWindowHandleClone.clone();
        let nickname_str: String = nickname.to_string(); // Convert SharedString to String for async block
        let password_str: String = password.to_string(); // Convert SharedString to String for async block

        tokio::spawn(async move {
            let client = Client::new();
            let payload = LoginPayload {
                nickname: nickname_str.clone(), // Clone for logging purposes if needed later
                password: password_str,
            };

            match client
                .post("http://127.0.0.1:3000/login")
                .json(&payload)
                .send()
                .await
            {
                Ok(response) => {
                    if response.status().is_success() {
                        match response.json::<AuthResponse>().await {
                            Ok(auth_response) => {
                                println!(
                                    "Login successful! Access Token: {}, Refresh Token: {}",
                                    auth_response.access_token, auth_response.refresh_token
                                );
                                if let Some(app) = weak_auth_clone.upgrade() {
                                    // Execute UI transition
                                    let main_app_window = mainApp::new().unwrap();
                                    main_app_window.set_nickName(nickname.clone()); // Use original SharedString here

                                    let weak_main_app = main_app_window.as_weak();
                                    main_app_window.on_exit(move || {
                                        if let Some(main_app) = weak_main_app.upgrade() {
                                            main_app.hide().unwrap();
                                        }
                                    });

                                    let (screenWidth, screenHeight) = display_size().unwrap();
                                    let (screenWidth, screenHeight) = (screenWidth as f32, screenHeight as f32);
                                    let (width, height) = (1280.0, 720.0);

                                    main_app_window.window().set_size(LogicalSize::new(width, height));
                                    main_app_window.window().set_position(LogicalPosition::new((screenWidth - width) / 2.0, (screenHeight - height) / 2.0));

                                    main_app_window.show().unwrap();
                                    app.hide().unwrap();
                                    *main_app_handle_clone.borrow_mut() = Some(main_app_window);
                                    app.set_status_message("".into()); // Clear message on success
                                }
                            }
                            Err(e) => {
                                eprintln!("Failed to parse login response: {:?}", e);
                                if let Some(app) = weak_auth_clone.upgrade() {
                                    app.set_status_message("Login failed: Invalid server response.".into());
                                }
                            }
                        }
                    } else {
                        let status = response.status();
                        let error_body = response.text().await.unwrap_or_else(|_| "Unknown error, could not retrieve error body".to_string());
                        eprintln!("Login failed with status: {}. Body: {}", status, error_body);
                        if let Some(app) = weak_auth_clone.upgrade() {
                            let error_message = if let Ok(json_error) = serde_json::from_str::<Value>(&error_body) {
                                json_error.get("error").and_then(Value::as_str)
                                    .map(|msg| format!("Login failed: {}", msg))
                                    .unwrap_or_else(|| format!("Login failed: HTTP {}", status))
                            } else {
                                format!("Login failed: HTTP {} - {}", status, error_body)
                            };
                            app.set_status_message(error_message.into());
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Login request failed: {:?}", e);
                    if let Some(app) = weak_auth_clone.upgrade() {
                        app.set_status_message("Login request failed. Check connection.".into());
                    }
                }
            }
        });
    });

    authenticationWindow.on_register(move |nickname, password| {
        let weak_auth_clone = weakAuthentication.clone();
        // Convert SharedString to String for async block if they are to be stored or passed around more
        let nickname_str: String = nickname.to_string();
        let password_str: String = password.to_string();

        tokio::spawn(async move {
            let client = Client::new();
            let payload = RegisterPayload {
                nickname: nickname_str.clone(), // Clone for logging
                password: password_str,
            };

            match client
                .post("http://127.0.0.1:3000/register")
                .json(&payload)
                .send()
                .await
            {
                Ok(response) => {
                    if response.status().is_success() { // Typically 201 Created for registration
                        println!("Registration successful for user: {}", nickname_str);
                        if let Some(app) = weak_auth_clone.upgrade() {
                            app.set_status_message("Registration successful! Please login.".into());
                            // Attempt to switch view to login. This assumes `status` is a global accessible via `app.global()`
                            // and `view::authorization` is the correct enum path.
                            // This line might need adjustment based on actual Slint global structure.
                            app.global::<status>().set_currentView(crate::view::Authorization);
                            println!("UI: Registration successful, requested switch to login view.");
                        }
                    } else {
                        let status = response.status();
                        let error_body = response.text().await.unwrap_or_else(|_| "Unknown error, could not retrieve error body".to_string());
                        eprintln!("Registration failed with status: {}. Body: {}", status, error_body);
                        if let Some(app) = weak_auth_clone.upgrade() {
                            let error_message = if let Ok(json_error) = serde_json::from_str::<Value>(&error_body) {
                                json_error.get("error").and_then(Value::as_str)
                                    .map(|msg| format!("Registration failed: {}", msg))
                                    .unwrap_or_else(|| format!("Registration failed: HTTP {}", status))
                            } else {
                                format!("Registration failed: HTTP {} - {}", status, error_body)
                            };
                            app.set_status_message(error_message.into());
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Registration request failed: {:?}", e);
                    if let Some(app) = weak_auth_clone.upgrade() {
                        app.set_status_message("Registration request failed. Check connection.".into());
                    }
                }
            }
        });
    });

    let weakAuthenticationExit = weakAuthentication.clone(); // Use the existing weak pointer

    authenticationWindow.on_exit(move ||
    {
        if let Some(app) = weakAuthenticationExit.upgrade()
        {
            app.hide().unwrap();
        }
    });

    let (screenWidth, screenHeight) = display_size().unwrap();
    let (sw, sh) = (screenWidth as f32, screenHeight as f32);
    let (w, h) = (380.0, 650.0);

    authenticationWindow.window().set_size(LogicalSize::new(w, h));
    authenticationWindow.window().set_position(LogicalPosition::new((sw - w) / 2.0, (sh - h) / 2.0));
    authenticationWindow.show().unwrap();

    slint::run_event_loop().unwrap();

    // It's important to ensure that JWT_SECRET and DATABASE_URL environment variables are set
    // for the application to function correctly.
    // JWT_SECRET is used for token generation and validation.
    // DATABASE_URL is used to connect to the PostgreSQL database.
}