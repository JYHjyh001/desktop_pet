use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle,
};

use crate::windowing;

pub fn create_tray(app: &AppHandle) -> tauri::Result<()> {
    let show_pet = MenuItem::with_id(app, "show_pet", "显示宠物", true, None::<&str>)?;
    let hide_pet = MenuItem::with_id(app, "hide_pet", "隐藏宠物", true, None::<&str>)?;
    let open_drawer = MenuItem::with_id(app, "open_drawer", "打开抽屉", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "退出程序", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show_pet, &hide_pet, &open_drawer, &quit])?;

    let mut builder = TrayIconBuilder::new()
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show_pet" => {
                let _ = windowing::show_pet(app);
            }
            "hide_pet" => {
                let _ = windowing::hide_pet(app);
            }
            "open_drawer" => {
                let _ = windowing::show_drawer(app);
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                let _ = windowing::show_pet(app);
                let _ = windowing::toggle_drawer(app);
            }
        });

    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }

    builder.build(app)?;
    Ok(())
}
