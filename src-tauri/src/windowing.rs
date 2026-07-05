use serde::{Deserialize, Serialize};
use tauri::{
    AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, Position, Size, WebviewWindow,
};

use crate::app_data;

const SCREEN_MARGIN: i32 = 8;
const PET_BUBBLE_BASE_GAP: i32 = 6;
const PET_BUBBLE_MAX_GAP: i32 = 18;
const PET_BUBBLE_TAIL_MARGIN: i32 = 22;
const PET_BUBBLE_MIN_WIDTH: u32 = 168;
const PET_BUBBLE_MAX_WIDTH: u32 = 340;
const PET_BUBBLE_EXPANDED_MAX_WIDTH: u32 = 420;
const PET_BUBBLE_EXPANDED_MAX_HEIGHT: u32 = 376;
const PET_BUBBLE_TEXT_CHROME_WIDTH: u32 = 58;
const PET_BUBBLE_VERTICAL_CHROME_HEIGHT: u32 = 28;
const PET_BUBBLE_STACK_GAP: u32 = 8;
const PET_BUBBLE_SINGLE_LINE_HEIGHT: u32 = 68;
const PET_BUBBLE_TWO_LINE_HEIGHT: u32 = 92;
const PET_BUBBLE_THREE_LINE_HEIGHT: u32 = 116;
const PET_BADGE_WIDTH: u32 = 144;
const PET_BADGE_HEIGHT: u32 = 44;
const PET_COMPLETION_WIDTH: u32 = 156;
const PET_COMPLETION_HEIGHT: u32 = 52;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PetBubbleItemPayload {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub channel: String,
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub state: String,
    #[serde(default)]
    pub message: String,
    #[serde(default)]
    pub expanded: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PetBubblePayload {
    pub kind: String,
    pub state: String,
    pub message: String,
    pub theme: String,
    #[serde(default)]
    pub items: Vec<PetBubbleItemPayload>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct PetBubbleRenderPayload {
    kind: String,
    state: String,
    message: String,
    theme: String,
    items: Vec<PetBubbleItemPayload>,
    placement: String,
    tail_x: i32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct PetBubblePlacementPayload {
    placement: String,
    tail_x: i32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct WindowOpenAnimationPayload {
    label: String,
    variant: String,
}

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

    let _ = set_pet_size(app, config.pet.size);
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
        prepare_window_open_animation(&drawer);
        return Ok(());
    }

    show_drawer(app)
}

pub fn show_drawer(app: &AppHandle) -> Result<(), String> {
    let drawer = app
        .get_webview_window("drawer")
        .ok_or_else(|| "未找到抽屉窗口".to_string())?;

    position_drawer(app)?;
    show_window_with_open_animation(&drawer, "drawer", true)
}

pub fn hide_drawer(app: &AppHandle) -> Result<(), String> {
    let drawer = app
        .get_webview_window("drawer")
        .ok_or_else(|| "未找到抽屉窗口".to_string())?;

    drawer.hide().map_err(|err| err.to_string())?;
    prepare_window_open_animation(&drawer);
    Ok(())
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
    show_window_with_open_animation(&menu, "menu", true)
}

pub fn hide_pet_menu(app: &AppHandle) -> Result<(), String> {
    let menu = app
        .get_webview_window("pet-menu")
        .ok_or_else(|| "未找到宠物菜单窗口".to_string())?;

    menu.hide().map_err(|err| err.to_string())?;
    prepare_window_open_animation(&menu);
    Ok(())
}

pub fn show_pet_chat(app: &AppHandle) -> Result<(), String> {
    let chat = app
        .get_webview_window("pet-chat")
        .ok_or_else(|| "未找到宠物对话窗口".to_string())?;

    position_pet_chat(app)?;
    show_window_with_open_animation(&chat, "panel", true)?;
    app.emit("pet-chat-opened", ())
        .map_err(|err| err.to_string())
}

pub fn hide_pet_chat(app: &AppHandle) -> Result<(), String> {
    let chat = app
        .get_webview_window("pet-chat")
        .ok_or_else(|| "未找到宠物对话窗口".to_string())?;

    chat.hide().map_err(|err| err.to_string())?;
    prepare_window_open_animation(&chat);
    Ok(())
}

pub fn show_story(app: &AppHandle) -> Result<(), String> {
    let story = app
        .get_webview_window("story")
        .ok_or_else(|| "未找到故事模式窗口".to_string())?;

    position_story(app)?;
    show_window_with_open_animation(&story, "panel", true)?;
    app.emit("story-opened", ()).map_err(|err| err.to_string())
}

pub fn hide_story(app: &AppHandle) -> Result<(), String> {
    let story = app
        .get_webview_window("story")
        .ok_or_else(|| "未找到故事模式窗口".to_string())?;

    story.hide().map_err(|err| err.to_string())?;
    prepare_window_open_animation(&story);
    Ok(())
}

pub fn show_translator(app: &AppHandle) -> Result<(), String> {
    let translator = app
        .get_webview_window("translator")
        .ok_or_else(|| "未找到翻译窗口".to_string())?;

    position_translator(app)?;
    show_window_with_open_animation(&translator, "panel", true)?;
    app.emit("translator-opened", ())
        .map_err(|err| err.to_string())
}

pub fn hide_translator(app: &AppHandle) -> Result<(), String> {
    let translator = app
        .get_webview_window("translator")
        .ok_or_else(|| "未找到翻译窗口".to_string())?;

    translator.hide().map_err(|err| err.to_string())?;
    prepare_window_open_animation(&translator);
    Ok(())
}

pub fn show_music_player(app: &AppHandle) -> Result<(), String> {
    let music = app
        .get_webview_window("music")
        .ok_or_else(|| "未找到音乐播放窗口".to_string())?;

    position_music_player(app)?;
    show_window_with_open_animation(&music, "panel", true)
}

pub fn hide_music_player(app: &AppHandle) -> Result<(), String> {
    let music = app
        .get_webview_window("music")
        .ok_or_else(|| "未找到音乐播放窗口".to_string())?;

    music.hide().map_err(|err| err.to_string())?;
    prepare_window_open_animation(&music);
    Ok(())
}

pub fn show_pet(app: &AppHandle) -> Result<(), String> {
    let pet = app
        .get_webview_window("pet")
        .ok_or_else(|| "未找到宠物窗口".to_string())?;

    restore_pet_position(app);
    show_window_with_open_animation(&pet, "pet", true)
}

pub fn set_pet_always_on_top(app: &AppHandle, always_on_top: bool) -> Result<(), String> {
    let pet = app
        .get_webview_window("pet")
        .ok_or_else(|| "未找到宠物窗口".to_string())?;

    pet.set_always_on_top(always_on_top)
        .map_err(|err| err.to_string())?;

    if let Some(bubble) = app.get_webview_window("pet-bubble") {
        bubble
            .set_always_on_top(always_on_top)
            .map_err(|err| err.to_string())?;
    }

    Ok(())
}

pub fn set_pet_size(app: &AppHandle, size: u32) -> Result<(), String> {
    let pet = app
        .get_webview_window("pet")
        .ok_or_else(|| "未找到宠物窗口".to_string())?;
    let size = app_data::normalize_pet_size(size);

    pet.set_size(Size::Physical(PhysicalSize {
        width: size,
        height: size,
    }))
    .map_err(|err| err.to_string())?;
    restore_pet_position(app);
    let _ = reposition_pet_bubble(app);

    Ok(())
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

    let _ = hide_pet_bubble(app);
    pet.hide().map_err(|err| err.to_string())?;
    prepare_window_open_animation(&pet);
    Ok(())
}

pub fn show_pet_bubble(app: &AppHandle, payload: PetBubblePayload) -> Result<(), String> {
    let bubble = app
        .get_webview_window("pet-bubble")
        .ok_or_else(|| "未找到宠物气泡窗口".to_string())?;
    let payload = normalize_pet_bubble_payload(payload);
    let scale_factor = window_scale_factor(&bubble);
    let (logical_width, logical_height) = pet_bubble_window_size(&payload);
    let width = logical_to_physical_size(logical_width, scale_factor);
    let height = logical_to_physical_size(logical_height, scale_factor);
    let keep_cursor_inside = payload.items.iter().any(|item| item.expanded);

    bubble
        .set_size(Size::Physical(PhysicalSize { width, height }))
        .map_err(|err| err.to_string())?;
    let placement = position_pet_bubble_window(
        app,
        &bubble,
        &payload.kind,
        width,
        height,
        scale_factor,
        keep_cursor_inside,
    )?;
    bubble
        .emit(
            "pet-bubble-updated",
            PetBubbleRenderPayload {
                kind: payload.kind,
                state: payload.state,
                message: payload.message,
                theme: payload.theme,
                items: payload.items,
                placement: placement.placement,
                tail_x: placement.tail_x,
            },
        )
        .map_err(|err| err.to_string())?;
    show_window_with_open_animation(&bubble, "bubble", false)?;
    raise_pet_bubble_window(app, &bubble)
}

pub fn hide_pet_bubble(app: &AppHandle) -> Result<(), String> {
    let bubble = app
        .get_webview_window("pet-bubble")
        .ok_or_else(|| "未找到宠物气泡窗口".to_string())?;

    bubble.hide().map_err(|err| err.to_string())?;
    prepare_window_open_animation(&bubble);
    Ok(())
}

pub fn reposition_pet_bubble(app: &AppHandle) -> Result<(), String> {
    let bubble = app
        .get_webview_window("pet-bubble")
        .ok_or_else(|| "未找到宠物气泡窗口".to_string())?;

    if !bubble.is_visible().map_err(|err| err.to_string())? {
        return Ok(());
    }

    let size = bubble.outer_size().map_err(|err| err.to_string())?;
    let scale_factor = window_scale_factor(&bubble);
    let kind = pet_bubble_kind_for_size(size.width, size.height, scale_factor);
    let placement = position_pet_bubble_window(
        app,
        &bubble,
        kind,
        size.width,
        size.height,
        scale_factor,
        false,
    )?;
    bubble
        .emit("pet-bubble-placement-updated", placement)
        .map_err(|err| err.to_string())?;
    raise_pet_bubble_window(app, &bubble)
}

fn normalize_pet_bubble_payload(payload: PetBubblePayload) -> PetBubblePayload {
    let kind = normalize_pet_bubble_kind(&payload.kind);
    let theme = if payload.theme == "animal-island" {
        "animal-island".to_string()
    } else {
        "light".to_string()
    };
    let state = normalize_pet_bubble_state(&payload.state);
    let message = payload
        .message
        .trim()
        .chars()
        .take(if kind == "badge" || kind == "completion" {
            24
        } else {
            140
        })
        .collect::<String>();
    let mut items = payload
        .items
        .into_iter()
        .map(normalize_pet_bubble_item)
        .filter(|item| !item.message.is_empty())
        .take(4)
        .collect::<Vec<_>>();

    if kind != "bubble" && items.len() > 1 {
        items.truncate(1);
    }

    if items.is_empty() && !message.is_empty() {
        items.push(PetBubbleItemPayload {
            id: "codex".to_string(),
            channel: "codex".to_string(),
            kind: kind.clone(),
            state: state.clone(),
            message: message.clone(),
            expanded: false,
        });
    }

    PetBubblePayload {
        kind,
        state,
        message,
        theme,
        items,
    }
}

fn normalize_pet_bubble_item(item: PetBubbleItemPayload) -> PetBubbleItemPayload {
    let kind = normalize_pet_bubble_kind(&item.kind);
    let channel = if item.channel == "translation" {
        "translation".to_string()
    } else {
        "codex".to_string()
    };
    let expanded = kind == "bubble" && channel == "translation" && item.expanded;
    let message = item
        .message
        .trim()
        .chars()
        .take(if expanded {
            8000
        } else if kind == "badge" || kind == "completion" {
            24
        } else {
            140
        })
        .collect::<String>();
    let id = item.id.trim();
    let id = if id.is_empty() {
        channel.clone()
    } else {
        id.chars().take(32).collect::<String>()
    };

    PetBubbleItemPayload {
        id,
        channel,
        kind,
        state: normalize_pet_bubble_state(&item.state),
        message,
        expanded,
    }
}

fn normalize_pet_bubble_kind(kind: &str) -> String {
    match kind {
        "badge" | "completion" => kind.to_string(),
        _ => "bubble".to_string(),
    }
}

fn normalize_pet_bubble_state(state: &str) -> String {
    match state {
        "disconnected" | "starting" | "connected" | "running" | "waiting" | "review"
        | "completed" | "failed" => state.to_string(),
        _ => "connected".to_string(),
    }
}

fn pet_bubble_window_size(payload: &PetBubblePayload) -> (u32, u32) {
    match payload.kind.as_str() {
        "badge" => (PET_BADGE_WIDTH, PET_BADGE_HEIGHT),
        "completion" => (PET_COMPLETION_WIDTH, PET_COMPLETION_HEIGHT),
        _ => pet_bubble_stack_window_size(payload),
    }
}

fn pet_bubble_stack_window_size(payload: &PetBubblePayload) -> (u32, u32) {
    let items = if payload.items.is_empty() {
        return pet_bubble_dynamic_window_size(&payload.message);
    } else {
        &payload.items
    };
    let width = items
        .iter()
        .map(pet_bubble_item_window_size)
        .map(|size| size.0)
        .max()
        .unwrap_or(PET_BUBBLE_MIN_WIDTH);
    let cards_height = items.iter().map(pet_bubble_item_card_height).sum::<u32>();
    let gaps = items.len().saturating_sub(1).try_into().unwrap_or(0u32) * PET_BUBBLE_STACK_GAP;
    let height = PET_BUBBLE_VERTICAL_CHROME_HEIGHT + cards_height + gaps;

    (width, height)
}

fn pet_bubble_item_card_height(item: &PetBubbleItemPayload) -> u32 {
    pet_bubble_item_window_size(item)
        .1
        .saturating_sub(PET_BUBBLE_VERTICAL_CHROME_HEIGHT)
        .max(40)
}

fn pet_bubble_item_window_size(item: &PetBubbleItemPayload) -> (u32, u32) {
    if item.expanded {
        return pet_bubble_expanded_window_size(&item.message);
    }

    pet_bubble_dynamic_window_size(&item.message)
}

fn pet_bubble_dynamic_window_size(message: &str) -> (u32, u32) {
    let display_units = message_display_units(message).max(4);
    let desired_width = display_units
        .saturating_mul(6)
        .saturating_add(PET_BUBBLE_TEXT_CHROME_WIDTH);
    let width = desired_width.clamp(PET_BUBBLE_MIN_WIDTH, PET_BUBBLE_MAX_WIDTH);
    let text_width = width.saturating_sub(PET_BUBBLE_TEXT_CHROME_WIDTH).max(96);
    let estimated_text_px = display_units.saturating_mul(6);
    let line_count = ((estimated_text_px + text_width - 1) / text_width).clamp(1, 3);
    let height = match line_count {
        1 => PET_BUBBLE_SINGLE_LINE_HEIGHT,
        2 => PET_BUBBLE_TWO_LINE_HEIGHT,
        _ => PET_BUBBLE_THREE_LINE_HEIGHT,
    };

    (width, height)
}

fn pet_bubble_expanded_window_size(message: &str) -> (u32, u32) {
    let display_units = message_display_units(message).max(12);
    let desired_width = display_units
        .saturating_mul(5)
        .saturating_add(PET_BUBBLE_TEXT_CHROME_WIDTH);
    let width = desired_width.clamp(PET_BUBBLE_MIN_WIDTH, PET_BUBBLE_EXPANDED_MAX_WIDTH);
    let text_width = width.saturating_sub(PET_BUBBLE_TEXT_CHROME_WIDTH).max(120);
    let estimated_text_px = display_units.saturating_mul(6);
    let line_count = ((estimated_text_px + text_width - 1) / text_width).clamp(2, 18);
    let text_height = line_count.saturating_mul(17).saturating_add(22);
    let height = PET_BUBBLE_VERTICAL_CHROME_HEIGHT
        .saturating_add(text_height)
        .clamp(PET_BUBBLE_TWO_LINE_HEIGHT, PET_BUBBLE_EXPANDED_MAX_HEIGHT);

    (width, height)
}

fn message_display_units(message: &str) -> u32 {
    message
        .chars()
        .map(|ch| if ch.is_ascii() { 1 } else { 2 })
        .sum()
}

fn window_scale_factor(window: &WebviewWindow) -> f64 {
    window.scale_factor().unwrap_or(1.0).clamp(1.0, 4.0)
}

fn logical_to_physical_size(value: u32, scale_factor: f64) -> u32 {
    ((value as f64) * scale_factor).ceil() as u32
}

fn logical_to_physical_i32(value: i32, scale_factor: f64) -> i32 {
    ((value as f64) * scale_factor).ceil() as i32
}

fn physical_to_logical_i32(value: i32, scale_factor: f64) -> i32 {
    ((value as f64) / scale_factor).round() as i32
}

fn pet_bubble_always_on_top(app: &AppHandle) -> bool {
    app_data::read_config(app)
        .map(|config| config.pet.always_on_top)
        .unwrap_or(true)
}

fn raise_pet_bubble_window(app: &AppHandle, bubble: &WebviewWindow) -> Result<(), String> {
    let always_on_top = pet_bubble_always_on_top(app);
    let _ = bubble.set_always_on_top(!always_on_top);
    bubble
        .set_always_on_top(always_on_top)
        .map_err(|err| err.to_string())?;
    raise_native_pet_bubble_window(bubble, always_on_top);
    Ok(())
}

fn show_window_with_open_animation(
    window: &WebviewWindow,
    variant: &str,
    focus: bool,
) -> Result<(), String> {
    let was_visible = window.is_visible().map_err(|err| err.to_string())?;
    if !was_visible {
        trigger_window_open_animation(window, variant);
    }

    window.show().map_err(|err| err.to_string())?;
    if focus {
        window.set_focus().map_err(|err| err.to_string())?;
    }

    Ok(())
}

fn prepare_window_open_animation(window: &WebviewWindow) {
    let _ = window.emit(
        "window-open-prepare",
        WindowOpenAnimationPayload {
            label: window.label().to_string(),
            variant: String::new(),
        },
    );
}

fn trigger_window_open_animation(window: &WebviewWindow, variant: &str) {
    let _ = window.emit(
        "window-open-animation",
        WindowOpenAnimationPayload {
            label: window.label().to_string(),
            variant: variant.to_string(),
        },
    );
}

#[cfg(windows)]
fn raise_native_pet_bubble_window(bubble: &WebviewWindow, always_on_top: bool) {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        SetWindowPos, HWND_TOP, HWND_TOPMOST, SWP_ASYNCWINDOWPOS, SWP_NOACTIVATE, SWP_NOMOVE,
        SWP_NOSIZE, SWP_SHOWWINDOW,
    };

    let Ok(hwnd) = bubble.hwnd() else {
        return;
    };
    let insert_after = if always_on_top {
        HWND_TOPMOST
    } else {
        HWND_TOP
    };
    unsafe {
        let _ = SetWindowPos(
            hwnd.0 as _,
            insert_after,
            0,
            0,
            0,
            0,
            SWP_ASYNCWINDOWPOS | SWP_NOACTIVATE | SWP_NOMOVE | SWP_NOSIZE | SWP_SHOWWINDOW,
        );
    }
}

#[cfg(not(windows))]
fn raise_native_pet_bubble_window(_bubble: &WebviewWindow, _always_on_top: bool) {}

fn pet_bubble_kind_for_size(width: u32, height: u32, scale_factor: f64) -> &'static str {
    if physical_size_matches_logical_size(
        width,
        height,
        PET_COMPLETION_WIDTH,
        PET_COMPLETION_HEIGHT,
        scale_factor,
    ) {
        "completion"
    } else if physical_size_matches_logical_size(
        width,
        height,
        PET_BADGE_WIDTH,
        PET_BADGE_HEIGHT,
        scale_factor,
    ) {
        "badge"
    } else {
        "bubble"
    }
}

fn physical_size_matches_logical_size(
    width: u32,
    height: u32,
    logical_width: u32,
    logical_height: u32,
    scale_factor: f64,
) -> bool {
    let expected_width = logical_to_physical_size(logical_width, scale_factor);
    let expected_height = logical_to_physical_size(logical_height, scale_factor);
    width.abs_diff(expected_width) <= 2 && height.abs_diff(expected_height) <= 2
}

fn position_pet_bubble_window(
    app: &AppHandle,
    bubble: &WebviewWindow,
    kind: &str,
    width: u32,
    height: u32,
    scale_factor: f64,
    keep_cursor_inside: bool,
) -> Result<PetBubblePlacementPayload, String> {
    let pet = app
        .get_webview_window("pet")
        .ok_or_else(|| "未找到宠物窗口".to_string())?;
    let pet_pos = pet.outer_position().map_err(|err| err.to_string())?;
    let pet_size = pet.outer_size().map_err(|err| err.to_string())?;
    let width = width as i32;
    let height = height as i32;
    let pet_center_x = pet_pos.x + pet_size.width as i32 / 2;
    if kind == "completion" {
        return position_pet_completion_window(
            &pet,
            bubble,
            pet_pos,
            pet_size,
            width,
            height,
            scale_factor,
        );
    }

    let preferred_x = pet_center_x - width / 2;
    let bubble_gap = pet_bubble_gap_for_size(pet_size);
    let top_y = pet_pos.y - height - bubble_gap;
    let bottom_y = pet_pos.y + pet_size.height as i32 + bubble_gap;
    let mut x = preferred_x;
    let mut y = top_y;
    let mut placement = "top".to_string();

    if let Some((left, top, right, bottom)) = monitor_bounds_for_window(&pet) {
        let min_x = left + SCREEN_MARGIN;
        let max_x = (right - width - SCREEN_MARGIN).max(min_x);
        let min_y = top + SCREEN_MARGIN;
        let max_y = (bottom - height - SCREEN_MARGIN).max(min_y);
        let top_available = (pet_pos.y - bubble_gap - min_y).max(0);
        let bottom_available =
            (max_y + height - pet_pos.y - pet_size.height as i32 - bubble_gap).max(0);
        let can_fit_top = height <= top_available;
        let can_fit_bottom = height <= bottom_available;

        if !can_fit_top && (can_fit_bottom || bottom_available > top_available) {
            y = bottom_y;
            placement = "bottom".to_string();
        }

        x = x.max(min_x).min(max_x);
        y = y.max(min_y).min(max_y);

        if keep_cursor_inside {
            if let Some(cursor) = current_cursor_position() {
                let cursor_padding = logical_to_physical_i32(14, scale_factor);
                if cursor.x < x + cursor_padding {
                    x = cursor.x - cursor_padding;
                } else if cursor.x > x + width - cursor_padding {
                    x = cursor.x - width + cursor_padding;
                }
                if cursor.y < y + cursor_padding {
                    y = cursor.y - cursor_padding;
                } else if cursor.y > y + height - cursor_padding {
                    y = cursor.y - height + cursor_padding;
                }

                x = x.max(min_x).min(max_x);
                y = y.max(min_y).min(max_y);
            }
        }
    }

    bubble
        .set_position(Position::Physical(PhysicalPosition { x, y }))
        .map_err(|err| err.to_string())?;

    let tail_margin = logical_to_physical_i32(PET_BUBBLE_TAIL_MARGIN, scale_factor);
    let max_tail_x = (width - tail_margin).max(tail_margin);
    let tail_x = physical_to_logical_i32(
        (pet_center_x - x).max(tail_margin).min(max_tail_x),
        scale_factor,
    );

    Ok(PetBubblePlacementPayload { placement, tail_x })
}

#[cfg(windows)]
fn current_cursor_position() -> Option<PhysicalPosition<i32>> {
    use windows_sys::Win32::{Foundation::POINT, UI::WindowsAndMessaging::GetCursorPos};

    let mut point = POINT { x: 0, y: 0 };
    let ok = unsafe { GetCursorPos(&mut point) };
    if ok == 0 {
        None
    } else {
        Some(PhysicalPosition {
            x: point.x,
            y: point.y,
        })
    }
}

#[cfg(not(windows))]
fn current_cursor_position() -> Option<PhysicalPosition<i32>> {
    None
}

fn pet_bubble_gap_for_size(pet_size: PhysicalSize<u32>) -> i32 {
    let pet_extent = pet_size.width.max(pet_size.height) as i32;
    let scaled_gap = pet_extent / 20;
    scaled_gap.clamp(PET_BUBBLE_BASE_GAP, PET_BUBBLE_MAX_GAP)
}

fn position_pet_completion_window(
    pet: &WebviewWindow,
    bubble: &WebviewWindow,
    pet_pos: PhysicalPosition<i32>,
    pet_size: PhysicalSize<u32>,
    width: i32,
    height: i32,
    scale_factor: f64,
) -> Result<PetBubblePlacementPayload, String> {
    let mut x = pet_pos.x + pet_size.width as i32 - 18;
    let mut y = pet_pos.y + pet_size.height as i32 - height - 8;

    if let Some((left, top, right, bottom)) = monitor_bounds_for_window(pet) {
        let min_x = left + SCREEN_MARGIN;
        let max_x = (right - width - SCREEN_MARGIN).max(min_x);
        let min_y = top + SCREEN_MARGIN;
        let max_y = (bottom - height - SCREEN_MARGIN).max(min_y);

        x = x.max(min_x).min(max_x);
        y = y.max(min_y).min(max_y);
    }

    bubble
        .set_position(Position::Physical(PhysicalPosition { x, y }))
        .map_err(|err| err.to_string())?;

    Ok(PetBubblePlacementPayload {
        placement: "side".to_string(),
        tail_x: physical_to_logical_i32(width / 2, scale_factor),
    })
}

fn monitor_bounds_for_window(window: &WebviewWindow) -> Option<(i32, i32, i32, i32)> {
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
        })?;
    let monitor_pos = monitor.position();
    let monitor_size = monitor.size();
    Some((
        monitor_pos.x,
        monitor_pos.y,
        monitor_pos.x + monitor_size.width as i32,
        monitor_pos.y + monitor_size.height as i32,
    ))
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

fn position_story(app: &AppHandle) -> Result<(), String> {
    let pet = app
        .get_webview_window("pet")
        .ok_or_else(|| "未找到宠物窗口".to_string())?;
    let story = app
        .get_webview_window("story")
        .ok_or_else(|| "未找到故事模式窗口".to_string())?;

    let pet_pos = pet.outer_position().map_err(|err| err.to_string())?;
    let pet_size = pet.outer_size().map_err(|err| err.to_string())?;
    let story_size = story.outer_size().map_err(|err| err.to_string())?;
    let margin = 12;

    let mut x = pet_pos.x + pet_size.width as i32 + margin;
    let mut y = pet_pos.y - 48;

    if let Ok(Some(monitor)) = pet.current_monitor() {
        let monitor_pos = monitor.position();
        let monitor_size = monitor.size();
        let max_x = monitor_pos.x + monitor_size.width as i32 - story_size.width as i32 - margin;
        let max_y = monitor_pos.y + monitor_size.height as i32 - story_size.height as i32 - margin;

        if x > max_x {
            x = pet_pos.x - story_size.width as i32 - margin;
        }

        x = x
            .max(monitor_pos.x + margin)
            .min(max_x.max(monitor_pos.x + margin));
        y = y
            .max(monitor_pos.y + margin)
            .min(max_y.max(monitor_pos.y + margin));
    }

    story
        .set_position(Position::Physical(PhysicalPosition { x, y }))
        .map_err(|err| err.to_string())
}

fn position_translator(app: &AppHandle) -> Result<(), String> {
    let pet = app
        .get_webview_window("pet")
        .ok_or_else(|| "未找到宠物窗口".to_string())?;
    let translator = app
        .get_webview_window("translator")
        .ok_or_else(|| "未找到翻译窗口".to_string())?;

    let pet_pos = pet.outer_position().map_err(|err| err.to_string())?;
    let pet_size = pet.outer_size().map_err(|err| err.to_string())?;
    let translator_size = translator.outer_size().map_err(|err| err.to_string())?;
    let margin = 12;

    let mut x = pet_pos.x + pet_size.width as i32 + margin;
    let mut y = pet_pos.y - 36;

    if let Ok(Some(monitor)) = pet.current_monitor() {
        let monitor_pos = monitor.position();
        let monitor_size = monitor.size();
        let max_x =
            monitor_pos.x + monitor_size.width as i32 - translator_size.width as i32 - margin;
        let max_y =
            monitor_pos.y + monitor_size.height as i32 - translator_size.height as i32 - margin;

        if x > max_x {
            x = pet_pos.x - translator_size.width as i32 - margin;
        }

        x = x
            .max(monitor_pos.x + margin)
            .min(max_x.max(monitor_pos.x + margin));
        y = y
            .max(monitor_pos.y + margin)
            .min(max_y.max(monitor_pos.y + margin));
    }

    translator
        .set_position(Position::Physical(PhysicalPosition { x, y }))
        .map_err(|err| err.to_string())
}

fn position_music_player(app: &AppHandle) -> Result<(), String> {
    let pet = app
        .get_webview_window("pet")
        .ok_or_else(|| "未找到宠物窗口".to_string())?;
    let music = app
        .get_webview_window("music")
        .ok_or_else(|| "未找到音乐播放窗口".to_string())?;

    let pet_pos = pet.outer_position().map_err(|err| err.to_string())?;
    let pet_size = pet.outer_size().map_err(|err| err.to_string())?;
    let music_size = music.outer_size().map_err(|err| err.to_string())?;
    let margin = 12;

    let mut x = pet_pos.x + pet_size.width as i32 + margin;
    let mut y = pet_pos.y - 24;

    if let Ok(Some(monitor)) = pet.current_monitor() {
        let monitor_pos = monitor.position();
        let monitor_size = monitor.size();
        let max_x = monitor_pos.x + monitor_size.width as i32 - music_size.width as i32 - margin;
        let max_y = monitor_pos.y + monitor_size.height as i32 - music_size.height as i32 - margin;

        if x > max_x {
            x = pet_pos.x - music_size.width as i32 - margin;
        }

        x = x
            .max(monitor_pos.x + margin)
            .min(max_x.max(monitor_pos.x + margin));
        y = y
            .max(monitor_pos.y + margin)
            .min(max_y.max(monitor_pos.y + margin));
    }

    music
        .set_position(Position::Physical(PhysicalPosition { x, y }))
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
