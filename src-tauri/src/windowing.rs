use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, Position, WebviewWindow};

use crate::app_data;

const SCREEN_MARGIN: i32 = 8;

pub fn restore_pet_position(app: &AppHandle) {
    let Ok(mut config) = app_data::read_config(app) else {
        return;
    };

    if let Some(window) = app.get_webview_window("pet") {
        let requested = match (config.pet.x, config.pet.y) {
            (Some(x), Some(y)) => PhysicalPosition { x, y },
            _ => default_pet_position(&window),
        };
        let position = visible_pet_position(&window, requested.x, requested.y);

        if config.pet.x != Some(position.x) || config.pet.y != Some(position.y) {
            config.pet.x = Some(position.x);
            config.pet.y = Some(position.y);
            let _ = app_data::write_config(app, &config);
        }

        let _ = window.set_position(Position::Physical(position));
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

        x = x
            .max(monitor_pos.x + margin)
            .min(max_x.max(monitor_pos.x + margin));
        y = y
            .max(monitor_pos.y + margin)
            .min(max_y.max(monitor_pos.y + margin));
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

pub fn show_pet_chat(app: &AppHandle) -> Result<(), String> {
    let chat = app
        .get_webview_window("pet-chat")
        .ok_or_else(|| "未找到宠物对话窗口".to_string())?;

    position_pet_chat(app)?;
    chat.show().map_err(|err| err.to_string())?;
    chat.set_focus().map_err(|err| err.to_string())?;
    app.emit("pet-chat-opened", ())
        .map_err(|err| err.to_string())
}

pub fn hide_pet_chat(app: &AppHandle) -> Result<(), String> {
    let chat = app
        .get_webview_window("pet-chat")
        .ok_or_else(|| "未找到宠物对话窗口".to_string())?;

    chat.hide().map_err(|err| err.to_string())
}

pub fn show_pet(app: &AppHandle) -> Result<(), String> {
    let pet = app
        .get_webview_window("pet")
        .ok_or_else(|| "未找到宠物窗口".to_string())?;

    restore_pet_position(app);
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
        let max_y = monitor_pos.y + monitor_size.height as i32 - drawer_size.height as i32 - margin;

        if x > max_x {
            x = pet_pos.x - drawer_size.width as i32 - margin;
        }

        x = x
            .max(monitor_pos.x + margin)
            .min(max_x.max(monitor_pos.x + margin));
        y = y
            .max(monitor_pos.y + margin)
            .min(max_y.max(monitor_pos.y + margin));
    }

    drawer
        .set_position(Position::Physical(PhysicalPosition { x, y }))
        .map_err(|err| err.to_string())
}

fn position_pet_chat(app: &AppHandle) -> Result<(), String> {
    let pet = app
        .get_webview_window("pet")
        .ok_or_else(|| "未找到宠物窗口".to_string())?;
    let chat = app
        .get_webview_window("pet-chat")
        .ok_or_else(|| "未找到宠物对话窗口".to_string())?;

    let pet_pos = pet.outer_position().map_err(|err| err.to_string())?;
    let pet_size = pet.outer_size().map_err(|err| err.to_string())?;
    let chat_size = chat.outer_size().map_err(|err| err.to_string())?;
    let margin = 12;

    let mut x = pet_pos.x + pet_size.width as i32 + margin;
    let mut y = pet_pos.y - 32;

    if let Ok(Some(monitor)) = pet.current_monitor() {
        let monitor_pos = monitor.position();
        let monitor_size = monitor.size();
        let max_x = monitor_pos.x + monitor_size.width as i32 - chat_size.width as i32 - margin;
        let max_y = monitor_pos.y + monitor_size.height as i32 - chat_size.height as i32 - margin;

        if x > max_x {
            x = pet_pos.x - chat_size.width as i32 - margin;
        }

        x = x
            .max(monitor_pos.x + margin)
            .min(max_x.max(monitor_pos.x + margin));
        y = y
            .max(monitor_pos.y + margin)
            .min(max_y.max(monitor_pos.y + margin));
    }

    chat.set_position(Position::Physical(PhysicalPosition { x, y }))
        .map_err(|err| err.to_string())
}

fn visible_pet_position(window: &WebviewWindow, x: i32, y: i32) -> PhysicalPosition<i32> {
    let Ok(window_size) = window.outer_size() else {
        return PhysicalPosition { x, y };
    };

    let width = window_size.width as i32;
    let height = window_size.height as i32;

    if let Ok(monitors) = window.available_monitors() {
        for monitor in &monitors {
            let monitor_pos = monitor.position();
            let monitor_size = monitor.size();
            let left = monitor_pos.x;
            let top = monitor_pos.y;
            let right = left + monitor_size.width as i32;
            let bottom = top + monitor_size.height as i32;

            let overlaps = x < right && x + width > left && y < bottom && y + height > top;
            if overlaps {
                return clamp_to_screen(x, y, width, height, left, top, right, bottom);
            }
        }

        if let Some(monitor) = monitors.first() {
            let monitor_pos = monitor.position();
            let monitor_size = monitor.size();
            return clamp_to_screen(
                x,
                y,
                width,
                height,
                monitor_pos.x,
                monitor_pos.y,
                monitor_pos.x + monitor_size.width as i32,
                monitor_pos.y + monitor_size.height as i32,
            );
        }
    }

    if let Ok(Some(monitor)) = window.primary_monitor() {
        let monitor_pos = monitor.position();
        let monitor_size = monitor.size();
        return clamp_to_screen(
            x,
            y,
            width,
            height,
            monitor_pos.x,
            monitor_pos.y,
            monitor_pos.x + monitor_size.width as i32,
            monitor_pos.y + monitor_size.height as i32,
        );
    }

    PhysicalPosition { x, y }
}

fn default_pet_position(window: &WebviewWindow) -> PhysicalPosition<i32> {
    let Ok(window_size) = window.outer_size() else {
        return PhysicalPosition {
            x: SCREEN_MARGIN,
            y: SCREEN_MARGIN,
        };
    };

    let width = window_size.width as i32;
    let height = window_size.height as i32;
    let monitor = window
        .current_monitor()
        .ok()
        .flatten()
        .or_else(|| window.primary_monitor().ok().flatten())
        .or_else(|| {
            window
                .available_monitors()
                .ok()
                .and_then(|monitors| monitors.into_iter().next())
        });

    if let Some(monitor) = monitor {
        let monitor_pos = monitor.position();
        let monitor_size = monitor.size();
        let left = monitor_pos.x;
        let top = monitor_pos.y;
        let right = left + monitor_size.width as i32;
        let bottom = top + monitor_size.height as i32;
        let x = right - width - 32;
        let y = bottom - height - 96;

        return clamp_to_screen(x, y, width, height, left, top, right, bottom);
    }

    PhysicalPosition {
        x: SCREEN_MARGIN,
        y: SCREEN_MARGIN,
    }
}

fn clamp_to_screen(
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
) -> PhysicalPosition<i32> {
    let min_x = left + SCREEN_MARGIN;
    let min_y = top + SCREEN_MARGIN;
    let max_x = (right - width - SCREEN_MARGIN).max(min_x);
    let max_y = (bottom - height - SCREEN_MARGIN).max(min_y);

    PhysicalPosition {
        x: x.max(min_x).min(max_x),
        y: y.max(min_y).min(max_y),
    }
}
