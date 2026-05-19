use tauri::{AppHandle, Manager, PhysicalPosition, Position};

use crate::app_data;

pub fn restore_pet_position(app: &AppHandle) {
    let Ok(config) = app_data::read_config(app) else {
        return;
    };

    let (Some(x), Some(y)) = (config.pet.x, config.pet.y) else {
        return;
    };

    if let Some(window) = app.get_webview_window("pet") {
        let _ = window.set_position(Position::Physical(PhysicalPosition { x, y }));
    }
}

pub fn apply_window_preferences(app: &AppHandle) {
    let Ok(config) = app_data::read_config(app) else {
        return;
    };

    let _ = set_pet_always_on_top(app, config.pet.always_on_top);
    let _ = set_drawer_always_on_top(app, config.drawer.always_on_top);
}

pub fn save_pet_position(app: &AppHandle, x: i32, y: i32) -> Result<(), String> {
    let mut config = app_data::read_config(app)?;
    config.pet.x = Some(x);
    config.pet.y = Some(y);
    app_data::write_config(app, &config)
}

pub fn toggle_drawer(app: &AppHandle) -> Result<(), String> {
    let drawer = app
        .get_webview_window("drawer")
        .ok_or_else(|| "未找到抽屉窗口".to_string())?;

    if drawer.is_visible().map_err(|err| err.to_string())? {
        drawer.hide().map_err(|err| err.to_string())?;
        return Ok(());
    }

    show_drawer(app)
}

pub fn show_drawer(app: &AppHandle) -> Result<(), String> {
    let drawer = app
        .get_webview_window("drawer")
        .ok_or_else(|| "未找到抽屉窗口".to_string())?;

    position_drawer(app)?;
    drawer.show().map_err(|err| err.to_string())?;
    drawer.set_focus().map_err(|err| err.to_string())?;

    Ok(())
}

pub fn hide_drawer(app: &AppHandle) -> Result<(), String> {
    let drawer = app
        .get_webview_window("drawer")
        .ok_or_else(|| "未找到抽屉窗口".to_string())?;

    drawer.hide().map_err(|err| err.to_string())
}

pub fn show_pet_menu(app: &AppHandle, cursor_x: i32, cursor_y: i32) -> Result<(), String> {
    let pet = app
        .get_webview_window("pet")
        .ok_or_else(|| "未找到宠物窗口".to_string())?;
    let menu = app
        .get_webview_window("pet-menu")
        .ok_or_else(|| "未找到宠物菜单窗口".to_string())?;

    let pet_pos = pet.outer_position().map_err(|err| err.to_string())?;
    let menu_size = menu.outer_size().map_err(|err| err.to_string())?;
    let margin = 8;
    let mut x = pet_pos.x + cursor_x + margin;
    let mut y = pet_pos.y + cursor_y + margin;

    if let Ok(Some(monitor)) = pet.current_monitor() {
        let monitor_pos = monitor.position();
        let monitor_size = monitor.size();
        let max_x = monitor_pos.x + monitor_size.width as i32 - menu_size.width as i32 - margin;
        let max_y = monitor_pos.y + monitor_size.height as i32 - menu_size.height as i32 - margin;

        x = x.max(monitor_pos.x + margin).min(max_x.max(monitor_pos.x + margin));
        y = y.max(monitor_pos.y + margin).min(max_y.max(monitor_pos.y + margin));
    }

    menu.set_position(Position::Physical(PhysicalPosition { x, y }))
        .map_err(|err| err.to_string())?;
    menu.show().map_err(|err| err.to_string())?;
    menu.set_focus().map_err(|err| err.to_string())
}

pub fn hide_pet_menu(app: &AppHandle) -> Result<(), String> {
    let menu = app
        .get_webview_window("pet-menu")
        .ok_or_else(|| "未找到宠物菜单窗口".to_string())?;

    menu.hide().map_err(|err| err.to_string())
}

pub fn show_pet(app: &AppHandle) -> Result<(), String> {
    let pet = app
        .get_webview_window("pet")
        .ok_or_else(|| "未找到宠物窗口".to_string())?;

    pet.show().map_err(|err| err.to_string())?;
    pet.set_focus().map_err(|err| err.to_string())
}

pub fn set_pet_always_on_top(app: &AppHandle, always_on_top: bool) -> Result<(), String> {
    let pet = app
        .get_webview_window("pet")
        .ok_or_else(|| "未找到宠物窗口".to_string())?;

    pet.set_always_on_top(always_on_top)
        .map_err(|err| err.to_string())
}

pub fn set_drawer_always_on_top(app: &AppHandle, always_on_top: bool) -> Result<(), String> {
    let drawer = app
        .get_webview_window("drawer")
        .ok_or_else(|| "未找到抽屉窗口".to_string())?;

    drawer
        .set_always_on_top(always_on_top)
        .map_err(|err| err.to_string())
}

pub fn hide_pet(app: &AppHandle) -> Result<(), String> {
    let pet = app
        .get_webview_window("pet")
        .ok_or_else(|| "未找到宠物窗口".to_string())?;

    pet.hide().map_err(|err| err.to_string())
}

fn position_drawer(app: &AppHandle) -> Result<(), String> {
    let pet = app
        .get_webview_window("pet")
        .ok_or_else(|| "未找到宠物窗口".to_string())?;
    let drawer = app
        .get_webview_window("drawer")
        .ok_or_else(|| "未找到抽屉窗口".to_string())?;

    let pet_pos = pet.outer_position().map_err(|err| err.to_string())?;
    let pet_size = pet.outer_size().map_err(|err| err.to_string())?;
    let drawer_size = drawer.outer_size().map_err(|err| err.to_string())?;
    let margin = 12;

    let mut x = pet_pos.x + pet_size.width as i32 + margin;
    let mut y = pet_pos.y;

    if let Ok(Some(monitor)) = pet.current_monitor() {
        let monitor_pos = monitor.position();
        let monitor_size = monitor.size();
        let max_x = monitor_pos.x + monitor_size.width as i32 - drawer_size.width as i32 - margin;
        let max_y =
            monitor_pos.y + monitor_size.height as i32 - drawer_size.height as i32 - margin;

        if x > max_x {
            x = pet_pos.x - drawer_size.width as i32 - margin;
        }

        x = x.max(monitor_pos.x + margin).min(max_x.max(monitor_pos.x + margin));
        y = y.max(monitor_pos.y + margin).min(max_y.max(monitor_pos.y + margin));
    }

    drawer
        .set_position(Position::Physical(PhysicalPosition { x, y }))
        .map_err(|err| err.to_string())
}
