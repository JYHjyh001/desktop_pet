use std::{path::Path, process::Command};

use tauri::AppHandle;

use crate::app_data::{self, PetApp};

pub fn launch_app(app: &AppHandle, app_id: &str) -> Result<PetApp, String> {
    let mut apps = app_data::read_apps(app)?;
    let index = apps
        .iter()
        .position(|item| item.id == app_id)
        .ok_or_else(|| "未找到该软件".to_string())?;

    let app_path = Path::new(&apps[index].path);
    if !app_path.exists() {
        return Err("软件路径不存在".to_string());
    }

    Command::new(app_path)
        .spawn()
        .map_err(|err| format!("启动失败：{err}"))?;

    let launched_at = app_data::now_seconds();
    apps[index].launch_count = apps[index].launch_count.saturating_add(1);
    apps[index].last_launch_at = Some(launched_at.clone());
    app_data::record_launch(&mut apps[index], launched_at);
    let updated = apps[index].clone();
    app_data::write_apps(app, &apps)?;

    Ok(updated)
}

pub fn open_app_dir(app: &AppHandle, app_id: &str) -> Result<(), String> {
    let apps = app_data::read_apps(app)?;
    let target = apps
        .iter()
        .find(|item| item.id == app_id)
        .ok_or_else(|| "未找到该软件".to_string())?;

    let path = Path::new(&target.path);
    if !path.exists() {
        return Err("软件路径不存在".to_string());
    }

    #[cfg(target_os = "windows")]
    {
        let selected = format!("/select,{}", path.display());
        Command::new("explorer.exe")
            .arg(selected)
            .spawn()
            .map_err(|err| format!("打开目录失败：{err}"))?;
    }

    #[cfg(not(target_os = "windows"))]
    {
        let parent = path.parent().ok_or_else(|| "无法获取软件目录".to_string())?;
        Command::new("open")
            .arg(parent)
            .spawn()
            .map_err(|err| format!("打开目录失败：{err}"))?;
    }

    Ok(())
}
