// main.rs

#![allow(non_snake_case)]

use rdev::display_size;
use slint::{ComponentHandle, LogicalPosition, LogicalSize};
use std::cell::RefCell;
use std::rc::Rc;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::Mutex;
use bcrypt::{hash, verify, DEFAULT_COST};

slint::include_modules!();

static USERS: Lazy<Mutex<HashMap<String, String>>> = Lazy::new(|| {
    Mutex::new(HashMap::new())
});

fn handle_signup(nickname: String, password: String) -> bool {
    // FUTURE: This function will make an HTTP POST request to a /signup endpoint.
    // For now, it simulates direct user creation.
    let mut users_map = USERS.lock().unwrap();

    if users_map.contains_key(&nickname) {
        println!("User {} already exists.", nickname);
        return false;
    }

    match hash(password, DEFAULT_COST) {
        Ok(hashed_password) => {
            users_map.insert(nickname.clone(), hashed_password);
            println!("User {} registered successfully.", nickname);
            true
        }
        Err(e) => {
            println!("Error hashing password for user {}: {:?}", nickname, e);
            false
        }
    }
}

fn handle_signin(nickname: String, password: String) -> bool {
    // FUTURE: This function will make an HTTP POST request to a /signin endpoint.
    // For now, it simulates direct credential check.
    let users_map = USERS.lock().unwrap();

    match users_map.get(&nickname) {
        Some(stored_hashed_password) => {
            match verify(password, stored_hashed_password) {
                Ok(is_valid) => {
                    if is_valid {
                        println!("User {} signed in successfully.", nickname);
                        true
                    } else {
                        println!("Invalid password for user {}.", nickname);
                        false
                    }
                }
                Err(e) => {
                    println!("Error verifying password for user {}: {:?}", nickname, e);
                    false
                }
            }
        }
        None => {
            println!("User {} not found.", nickname);
            false
        }
    }
}

fn main()
{
    let authenticationWindow = authentication::new().unwrap();
    let mainAppWindowHandle: Rc<RefCell<Option<mainApp>>> = Rc::new(RefCell::new(None));

    // Weak reference for callbacks
    let weakAuthentication = authenticationWindow.as_weak();

    // Clone for on_authenticate
    let mainAppWindowHandleClone = mainAppWindowHandle.clone();
    let auth_weak_for_auth = weakAuthentication.clone(); // Clone weak ref

    authenticationWindow.on_authenticate(move |nickName, password| {
        let nickName_str: String = nickName.into();
        let password_str: String = password.into();
        if handle_signin(nickName_str.clone(), password_str) {
            if let Some(app_auth) = auth_weak_for_auth.upgrade() { // Use the cloned weak ref
                app_auth.global::<status>().set_auth_status_message("".into());

                let mainAppWindow = mainApp::new().unwrap();
                mainAppWindow.set_nickName(nickName.into()); // Use original SharedString or new String

                let weakMainApp = mainAppWindow.as_weak();
                mainAppWindow.on_exit(move || {
                    if let Some(app_main) = weakMainApp.upgrade() {
                        app_main.hide().unwrap();
                    }
                });

                let (screenWidth, screenHeight) = display_size().unwrap();
                let (screenWidth_f32, screenHeight_f32) = (screenWidth as f32, screenHeight as f32);
                let (width, height) = (1280.0, 720.0);

                mainAppWindow.window().set_size(LogicalSize::new(width, height));
                mainAppWindow.window().set_position(LogicalPosition::new((screenWidth_f32 - width) / 2.0, (screenHeight_f32 - height) / 2.0));

                mainAppWindow.show().unwrap();
                app_auth.hide().unwrap(); // use app_auth here
                *mainAppWindowHandleClone.borrow_mut() = Some(mainAppWindow);
            }
        } else {
            if let Some(app_auth) = auth_weak_for_auth.upgrade() {
                app_auth.global::<status>().set_auth_status_message("Login failed. Check nickname or password.".into());
            }
            println!("Authentication failed for nickname: {}", nickName); // Keep console log
        }
    });

    // Clone weak ref for on_register
    let auth_weak_for_register = weakAuthentication.clone();

    authenticationWindow.on_register(move |nickName, password| {
        let nickName_str: String = nickName.into();
        let password_str: String = password.into();
        if handle_signup(nickName_str.clone(), password_str) {
            if let Some(auth_app) = auth_weak_for_register.upgrade() {
                auth_app.global::<status>().set_auth_status_message("Registration successful! Please log in.".into());
                auth_app.global::<status>().set_currentView(view::authorization);
            }
            println!("Registration successful for nickname: {}. Please log in.", nickName_str); // Keep console log
        } else {
            if let Some(auth_app) = auth_weak_for_register.upgrade() {
                auth_app.global::<status>().set_auth_status_message("Registration failed. User might already exist.".into());
            }
            println!("Registration failed for nickname: {}", nickName_str); // Keep console log
        }
    });

    let weakAuthenticationExit = authenticationWindow.as_weak(); // This can reuse weakAuthentication or be a new clone

    authenticationWindow.on_exit(move ||
    {
        if let Some(app) = weakAuthenticationExit.upgrade() // Ensure this weak ref is valid for this closure
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
}

#[cfg(test)]
mod tests {
    use super::*; // To import USERS, handle_signup, handle_signin

    // Test suite for handle_signup
    // #[test]
    // fn test_signup_new_user_successful() {
    //     // 1. Clear USERS or use a fresh instance for testing if possible
    //     // 2. Call handle_signup with new credentials
    //     // 3. Assert that it returns true
    //     // 4. Assert that the user is added to USERS (and password is hashed)
    // }

    // #[test]
    // fn test_signup_existing_user_fails() {
    //     // 1. Signup a user
    //     // 2. Attempt to signup the same user again
    //     // 3. Assert that the second call returns false
    // }

    // Add more placeholder comments for other signup scenarios:
    // - Password hashing integrity (conceptual: ensure it's not plaintext)
    // - (Future) Invalid inputs (empty nickname, short password)

    // Test suite for handle_signin
    // #[test]
    // fn test_signin_correct_credentials_successful() {
    //     // 1. Signup a user
    //     // 2. Call handle_signin with correct credentials
    //     // 3. Assert that it returns true
    // }

    // #[test]
    // fn test_signin_non_existent_user_fails() {
    //     // 1. Call handle_signin with credentials for a user that hasn't been signed up
    //     // 2. Assert that it returns false
    // }

    // #[test]
    // fn test_signin_incorrect_password_fails() {
    //     // 1. Signup a user
    //     // 2. Call handle_signin with the correct nickname but incorrect password
    //     // 3. Assert that it returns false
    // }

    // Add more placeholder comments for other signin scenarios:
    // - Signin after multiple users registered

    // Test suite for bcrypt password verification (direct bcrypt usage)
    // #[test]
    // fn test_bcrypt_verify_logic() {
    //     // let password = "test_password";
    //     // let hashed_password = bcrypt::hash(password, DEFAULT_COST).unwrap();
    //     // assert!(bcrypt::verify(password, &hashed_password).unwrap());
    //     // assert!(!bcrypt::verify("wrong_password", &hashed_password).unwrap());
    // }

    // Note: For actual tests involving the static USERS map,
    // careful state management between tests would be needed (e.g., clearing the map,
    // or running tests that depend on shared state in a controlled sequence, though the latter is not ideal).
    // Using a dependency-injected user store would be better for testability in a larger app.
}