use std::{io, path::Path, process::Command};

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

    launch_process(app_path)?;

    let launched_at = app_data::now_seconds();
    apps[index].launch_count = apps[index].launch_count.saturating_add(1);
    apps[index].last_launch_at = Some(launched_at.clone());
    app_data::record_launch(&mut apps[index], launched_at);
    let updated = apps[index].clone();
    app_data::write_apps(app, &apps)?;

    Ok(updated)
}

fn launch_process(app_path: &Path) -> Result<(), String> {
    match Command::new(app_path).spawn() {
        Ok(_) => Ok(()),
        Err(err) if is_elevation_required(&err) => launch_process_as_admin(app_path),
        Err(err) => Err(format!("启动失败：{err}")),
    }
}

#[cfg(target_os = "windows")]
fn is_elevation_required(err: &io::Error) -> bool {
    err.raw_os_error() == Some(740)
}

#[cfg(not(target_os = "windows"))]
fn is_elevation_required(_: &io::Error) -> bool {
    false
}

#[cfg(target_os = "windows")]
fn launch_process_as_admin(app_path: &Path) -> Result<(), String> {
    use std::{os::windows::ffi::OsStrExt, ptr};
    use windows_sys::Win32::UI::Shell::ShellExecuteW;
    use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

    let operation = to_wide_null("runas");
    let file = app_path
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect::<Vec<_>>();

    let result = unsafe {
        ShellExecuteW(
            ptr::null_mut(),
            operation.as_ptr(),
            file.as_ptr(),
            ptr::null(),
            ptr::null(),
            SW_SHOWNORMAL,
        )
    } as isize;

    if result > 32 {
        Ok(())
    } else {
        Err(format!(
            "启动失败：需要管理员权限，但权限确认窗口打开失败，错误码 {result}"
        ))
    }
}

#[cfg(not(target_os = "windows"))]
fn launch_process_as_admin(app_path: &Path) -> Result<(), String> {
    Command::new(app_path)
        .spawn()
        .map(|_| ())
        .map_err(|err| format!("启动失败：{err}"))
}

#[cfg(target_os = "windows")]
fn to_wide_null(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(Some(0)).collect()
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
        let parent = path
            .parent()
            .ok_or_else(|| "无法获取软件目录".to_string())?;
        Command::new("open")
            .arg(parent)
            .spawn()
            .map_err(|err| format!("打开目录失败：{err}"))?;
    }

    Ok(())
}
