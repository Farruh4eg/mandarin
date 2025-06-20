// main.rs

#![allow(non_snake_case)]

use rdev::display_size;
use slint::{ComponentHandle, LogicalPosition, LogicalSize};
use std::cell::RefCell;
use std::rc::Rc;

slint::include_modules!();

fn main()
{
    let authenticationWindow = authentication::new().unwrap();
    let mainAppWindowHandle: Rc<RefCell<Option<mainApp>>> = Rc::new(RefCell::new(None));
    let weakAuthentication = authenticationWindow.as_weak();
    let mainAppWindowHandleClone = mainAppWindowHandle.clone();

    authenticationWindow.on_authenticate(move |nickName|
    {
        if let Some(app) = weakAuthentication.upgrade()
        {
            let mainAppWindow = mainApp::new().unwrap();

            mainAppWindow.set_nickName(nickName);

            let weakMainApp = mainAppWindow.as_weak();

            mainAppWindow.on_exit(move ||
            {
                if let Some(app) = weakMainApp.upgrade()
                {
                    app.hide().unwrap();
                }
            });

            let (screenWidth, screenHeight) = display_size().unwrap();
            let (screenWidth, screenHeight) = (screenWidth as f32, screenHeight as f32);
            let (width, height) = (1280.0, 720.0);

            mainAppWindow.window().set_size(LogicalSize::new(width, height));
            mainAppWindow.window().set_position(LogicalPosition::new((screenWidth - width) / 2.0, (screenHeight - height) / 2.0));

            mainAppWindow.show().unwrap();
            app.hide().unwrap();

            *mainAppWindowHandleClone.borrow_mut() = Some(mainAppWindow);
        }
    });

    let weakAuthenticationExit = authenticationWindow.as_weak();

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
}