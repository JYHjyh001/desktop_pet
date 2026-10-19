//! Opt-in test driver, omitted from ordinary builds. Only synthetic isolated data.
use crate::{app_data, windowing};
use serde_json::{json, Value};
use std::{
    fs,
    sync::{Arc, Condvar, Mutex},
    time::Duration,
};
use tauri::{AppHandle, Emitter, Listener, Manager};

pub fn start(app: &AppHandle) -> Result<(), String> {
    if std::env::var("PETDRAWER_RESOURCE_VALIDATION")
        .ok()
        .as_deref()
        != Some("cycles")
    {
        return Ok(());
    }
    if app.config().identifier != "com.petdrawer.petoptvalidation" {
        return Err("验证模式只允许隔离应用标识".into());
    }
    let config = app_data::read_config(app)?;
    if config.ai.enabled
        || config.codex_app_server.enabled
        || config.wechat_clawbot.enabled
        || config.wechat_clawbot.bridge_enabled
    {
        return Err("验证配置包含外部连接开关，已拒绝启动".into());
    }
    let app = app.clone();
    std::thread::spawn(move || {
        let result = run(&app);
        let report = match result {
            Ok(value) => value,
            Err(error) => json!({"ok":false,"error":error}),
        };
        if let Ok(dir) = app_data::default_data_dir(&app) {
            let _ = fs::write(
                dir.join("resource-validation-report.json"),
                serde_json::to_vec_pretty(&report).unwrap_or_default(),
            );
        }
    });
    Ok(())
}

fn probe(app: &AppHandle, label: &str) -> Result<Value, String> {
    let state = Arc::new((Mutex::new(None), Condvar::new()));
    let receiver = state.clone();
    let listener = app.listen("pet-opt-validation-probe", move |event| {
        if let Ok(value) = serde_json::from_str::<Value>(event.payload()) {
            if let Ok(mut payload) = receiver.0.lock() {
                *payload = Some(value);
                receiver.1.notify_all();
            }
        }
    });
    let result = (|| {
        let window = app.get_webview_window(label).ok_or("验证窗口不存在")?;
        window.eval(r#"window.__TAURI_INTERNALS__.invoke('plugin:event|emit', {event:'pet-opt-validation-probe',payload:{
            label:window.__TAURI_INTERNALS__.metadata.currentWindow.label,
            audioTime:document.querySelector('audio')?.currentTime ?? null,
            paused:document.querySelector('audio')?.paused ?? null,
            theme:document.querySelector('main')?.className ?? '',
            bodyOverflow:document.documentElement.scrollWidth > window.innerWidth
        }})"#).map_err(|_| "验证脚本执行失败")?;
        let lock = state.0.lock().map_err(|_| "验证状态锁不可用")?;
        let (mut payload, _) = state
            .1
            .wait_timeout_while(lock, Duration::from_secs(5), |payload| payload.is_none())
            .map_err(|_| "验证等待失败")?;
        payload.take().ok_or("验证页面未响应".into())
    })();
    app.unlisten(listener);
    result
}

fn seed_music(app: &AppHandle) -> Result<(), String> {
    let dir = app_data::default_data_dir(app)?;
    let path = dir.join("validation-silence.wav");
    let count = 8000u32 * 60;
    let length = count * 2;
    let mut bytes = Vec::with_capacity(length as usize + 44);
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(length + 36).to_le_bytes());
    bytes.extend_from_slice(b"WAVEfmt ");
    bytes.extend_from_slice(&16u32.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&8000u32.to_le_bytes());
    bytes.extend_from_slice(&16000u32.to_le_bytes());
    bytes.extend_from_slice(&2u16.to_le_bytes());
    bytes.extend_from_slice(&16u16.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&length.to_le_bytes());
    bytes.resize(length as usize + 44, 0);
    fs::write(&path, bytes).map_err(|_| "合成音频创建失败")?;
    app_data::write_music_library_state(
        app,
        &json!({"version":1,"settings":{"volume":0,"repeatMode":"one"},"tracks":[{"id":"validation-silence","title":"Validation silence","path":path.to_string_lossy(),"tags":[]}],"playlists":[],"savedAt":null}),
    )
}

fn run(app: &AppHandle) -> Result<Value, String> {
    std::thread::sleep(Duration::from_secs(3));
    seed_music(app)?;
    let mut cycles = Vec::new();
    for theme in ["light", "animal-island"] {
        let mut config = app_data::read_config(app)?;
        config.drawer.theme = theme.into();
        app_data::write_config(app, &config)?;
        app.emit("ui-theme-changed", theme)
            .map_err(|_| "验证主题通知失败")?;
        for cycle in 0..20 {
            windowing::show_drawer(app)?;
            windowing::hide_drawer(app)?;
            windowing::show_pet_chat(app)?;
            windowing::hide_pet_chat(app)?;
            windowing::show_story(app)?;
            windowing::hide_story(app)?;
            windowing::show_translator(app)?;
            windowing::hide_translator(app)?;
            windowing::show_pet_menu(app, 0, 0)?;
            windowing::hide_pet_menu(app)?;
            windowing::show_pet_bubble(
                app,
                windowing::PetBubblePayload {
                    kind: "bubble".into(),
                    state: "completed".into(),
                    message: "Synthetic validation".into(),
                    theme: theme.into(),
                    items: Vec::new(),
                },
            )?;
            windowing::hide_pet_bubble(app)?;
            windowing::show_music_player(app)?;
            if cycle == 0 {
                cycles.push(probe(app, "music")?);
            }
            windowing::hide_music_player(app)?;
            cycles
                .push(json!({"theme":theme,"cycle":cycle+1,"windows":app.webview_windows().len()}));
            std::thread::sleep(Duration::from_millis(150));
        }
    }
    windowing::show_music_player(app)?;
    app.emit(
        "music-action-requested",
        json!({"type":"music_action","action":"resume"}),
    )
    .map_err(|_| "验证播放通知失败")?;
    std::thread::sleep(Duration::from_secs(3));
    let before = probe(app, "music")?;
    windowing::hide_music_player(app)?;
    std::thread::sleep(Duration::from_secs(5));
    let after = probe(app, "music")?;
    let playing = before["paused"] == false
        && after["paused"] == false
        && after["audioTime"].as_f64() > before["audioTime"].as_f64();
    Ok(
        json!({"ok":playing,"cyclesPerTheme":20,"cycles":cycles,"audioBeforeHide":before,"audioAfterHide":after,"hiddenMusicPlaying":playing}),
    )
}
