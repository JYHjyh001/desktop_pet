//! Lazy creation uses the existing window config as its single source of truth.
//! Called on blocking workers, never from a synchronous UI event handler.
use std::{
    collections::HashSet,
    sync::{Condvar, Mutex},
    time::{Duration, Instant},
};
use tauri::{AppHandle, Manager, WebviewWindow, WebviewWindowBuilder};

#[derive(Default)]
pub struct WindowLifecycle {
    creation: Mutex<()>,
    ready: Mutex<HashSet<String>>,
    changed: Condvar,
}

pub fn mark_ready(app: &AppHandle, label: &str) -> Result<(), String> {
    let state = app.state::<WindowLifecycle>();
    state
        .ready
        .lock()
        .map_err(|_| "窗口状态不可用")?
        .insert(label.into());
    state.changed.notify_all();
    Ok(())
}

pub fn ensure(app: &AppHandle, label: &str) -> Result<WebviewWindow, String> {
    let state = app.state::<WindowLifecycle>();
    let window = {
        let _creation = state.creation.lock().map_err(|_| "窗口创建锁不可用")?;
        if let Some(window) = app.get_webview_window(label) {
            window
        } else {
            let config = app
                .config()
                .app
                .windows
                .iter()
                .find(|config| config.label == label)
                .ok_or("窗口配置不存在")?;
            state
                .ready
                .lock()
                .map_err(|_| "窗口状态不可用")?
                .remove(label);
            let window = WebviewWindowBuilder::from_config(app, config)
                .map_err(|_| "窗口配置无效")?
                .build()
                .map_err(|_| "窗口创建失败")?;
            let close_window = window.clone();
            window.on_window_event(move |event| {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = close_window.hide();
                }
            });
            window
        }
    };
    // Do not hold the creation lock while the frontend invokes its ready command.
    let deadline = Instant::now() + Duration::from_secs(20);
    let mut ready = state.ready.lock().map_err(|_| "窗口状态不可用")?;
    while !ready.contains(label) {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err("窗口正在初始化，请稍后重试。".into());
        }
        ready = state
            .changed
            .wait_timeout(ready, remaining)
            .map_err(|_| "窗口状态不可用")?
            .0;
    }
    Ok(window)
}

#[cfg(test)]
mod tests {
    #[test]
    fn only_pet_is_eager_and_all_window_configuration_is_preserved() {
        let config: serde_json::Value =
            serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
        let windows = config["app"]["windows"].as_array().unwrap();
        assert_eq!(windows.len(), 8);
        for window in windows {
            assert_eq!(window["create"] != false, window["label"] == "pet");
        }
    }
}
