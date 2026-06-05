use std::{io, path::Path, process::Command};

use tauri::AppHandle;

use crate::app_data::{self, PetApp};

pub fn launch_app(app: &AppHandle, app_id: &str) -> Result<PetApp, String> {
    let mut apps = app_data::read_apps(app)?;
    let auto_favorite_enabled = app_data::read_config(app)
        .map(|config| config.system.auto_favorite_enabled)
        .unwrap_or(true);
    let index = apps
        .iter()
        .position(|item| item.id == app_id)
        .ok_or_else(|| "未找到该快捷入口".to_string())?;

    match apps[index].item_kind.as_str() {
        "folder" => launch_folder(Path::new(&apps[index].path))?,
        "website" => launch_website(&apps[index].path)?,
        "file" => launch_file(Path::new(&apps[index].path))?,
        _ => {
            let app_path = Path::new(&apps[index].path);
            if !app_path.exists() {
                return Err("软件路径不存在".to_string());
            }

            launch_process(app_path, apps[index].run_as_admin)?;
        }
    }

    let launched_at = app_data::now_seconds();
    apps[index].launch_count = apps[index].launch_count.saturating_add(1);
    apps[index].last_launch_at = Some(launched_at.clone());
    app_data::record_launch(&mut apps[index], launched_at, auto_favorite_enabled);
    let updated = apps[index].clone();
    app_data::write_apps(app, &apps)?;

    Ok(updated)
}

fn launch_folder(folder_path: &Path) -> Result<(), String> {
    if !folder_path.exists() {
        return Err("文件夹不存在".to_string());
    }

    if !folder_path.is_dir() {
        return Err("该路径不是文件夹".to_string());
    }

    open_folder(folder_path)
}

fn launch_website(url: &str) -> Result<(), String> {
    let normalized = normalize_website_url(url)?;
    open_website(&normalized)
}

fn launch_file(file_path: &Path) -> Result<(), String> {
    if !file_path.exists() {
        return Err("文件不存在".to_string());
    }

    if !file_path.is_file() {
        return Err("该路径不是文件".to_string());
    }

    open_file(file_path)
}

fn launch_process(app_path: &Path, run_as_admin: bool) -> Result<(), String> {
    if run_as_admin {
        return launch_process_as_admin(app_path);
    }

    match Command::new(app_path).spawn() {
        Ok(_) => Ok(()),
        Err(err) if is_elevation_required(&err) => Err(
            "启动失败：该软件需要管理员权限，请编辑软件并勾选“以管理员身份启动”后重试".to_string(),
        ),
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

#[cfg(target_os = "windows")]
fn open_folder(folder_path: &Path) -> Result<(), String> {
    Command::new("explorer.exe")
        .arg(folder_path)
        .spawn()
        .map(|_| ())
        .map_err(|err| format!("打开文件夹失败：{err}"))
}

#[cfg(not(target_os = "windows"))]
fn open_folder(folder_path: &Path) -> Result<(), String> {
    Command::new("open")
        .arg(folder_path)
        .spawn()
        .map(|_| ())
        .map_err(|err| format!("打开文件夹失败：{err}"))
}

#[cfg(target_os = "windows")]
fn open_website(url: &str) -> Result<(), String> {
    use std::ptr;
    use windows_sys::Win32::UI::Shell::ShellExecuteW;
    use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

    let operation = to_wide_null("open");
    let file = to_wide_null(url);

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
        Err(format!("打开网站失败：系统返回错误码 {result}"))
    }
}

#[cfg(not(target_os = "windows"))]
fn open_website(url: &str) -> Result<(), String> {
    Command::new("open")
        .arg(url)
        .spawn()
        .map(|_| ())
        .map_err(|err| format!("打开网站失败：{err}"))
}

#[cfg(target_os = "windows")]
fn open_file(file_path: &Path) -> Result<(), String> {
    use std::{os::windows::ffi::OsStrExt, ptr};
    use windows_sys::Win32::UI::Shell::ShellExecuteW;
    use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

    let operation = to_wide_null("open");
    let file = file_path
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
        Err(format!("打开文件失败：系统返回错误码 {result}"))
    }
}

#[cfg(not(target_os = "windows"))]
fn open_file(file_path: &Path) -> Result<(), String> {
    Command::new("open")
        .arg(file_path)
        .spawn()
        .map(|_| ())
        .map_err(|err| format!("打开文件失败：{err}"))
}

#[cfg(target_os = "macos")]
fn launch_process_as_admin(app_path: &Path) -> Result<(), String> {
    Command::new("/usr/bin/osascript")
        .args([
            "-e",
            &format!(
                "do shell script \"open -a '{}'\" with administrator privileges",
                app_path.to_string_lossy().replace('"', "\\\"")
            ),
        ])
        .spawn()
        .map(|_| ())
        .map_err(|err| format!("管理员启动失败：{err}"))
}

#[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
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
        .ok_or_else(|| "未找到该快捷入口".to_string())?;

    let path = Path::new(&target.path);
    if !path.exists() {
        return Err(match target.item_kind.as_str() {
            "folder" => "文件夹不存在".to_string(),
            "file" => "文件不存在".to_string(),
            _ => "软件路径不存在".to_string(),
        });
    }

    #[cfg(target_os = "windows")]
    {
        if target.item_kind == "folder" {
            open_folder(path)?;
        } else {
            let selected = format!("/select,{}", path.display());
            Command::new("explorer.exe")
                .arg(selected)
                .spawn()
                .map_err(|err| format!("打开目录失败：{err}"))?;
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        if target.item_kind == "folder" {
            open_folder(path)?;
        } else {
            let parent = path
                .parent()
                .ok_or_else(|| "无法获取所在目录".to_string())?;
            Command::new("open")
                .arg(parent)
                .spawn()
                .map_err(|err| format!("打开目录失败：{err}"))?;
        }
    }

    Ok(())
}

fn normalize_website_url(url: &str) -> Result<String, String> {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        return Err("网址不能为空".to_string());
    }

    if trimmed.chars().any(char::is_whitespace) {
        return Err("网址不能包含空白字符".to_string());
    }

    let lower = trimmed.to_ascii_lowercase();
    let normalized = if lower.starts_with("http://") || lower.starts_with("https://") {
        trimmed.to_string()
    } else {
        format!("https://{trimmed}")
    };

    let lower_normalized = normalized.to_ascii_lowercase();
    let without_scheme = lower_normalized
        .trim_start_matches("http://")
        .trim_start_matches("https://");

    let host = without_scheme
        .split(|value| matches!(value, '/' | ':' | '?' | '#'))
        .next()
        .unwrap_or_default();

    if !has_valid_website_host(host) {
        return Err("请输入有效的网址".to_string());
    }

    Ok(normalized)
}

fn has_valid_website_host(host: &str) -> bool {
    if host == "localhost" {
        return true;
    }

    if host.contains('.') && !host.starts_with('.') && !host.ends_with('.') {
        return true;
    }

    let parts = host.split('.').collect::<Vec<_>>();
    parts.len() == 4
        && parts.iter().all(|part| {
            !part.is_empty()
                && part.len() <= 3
                && part.parse::<u8>().is_ok()
                && (*part == "0" || !part.starts_with('0'))
        })
}
