#[cfg(target_os = "windows")]
const RUN_KEY: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run";
#[cfg(target_os = "windows")]
const RUN_VALUE_NAME: &str = "PetDrawer";

#[cfg(target_os = "windows")]
pub fn is_start_on_boot_enabled() -> Result<bool, String> {
    let Some(run_value) = read_run_value()? else {
        return Ok(false);
    };
    let Some(run_path) = run_value_exe_path(&run_value) else {
        return Ok(false);
    };
    let current_path =
        std::env::current_exe().map_err(|err| format!("无法获取当前程序路径：{err}"))?;
    let current_path = current_path.to_string_lossy().to_string();
    if same_windows_path(&run_path, &current_path) {
        return Ok(true);
    }

    let startup_path = match startup_exe_path() {
        Ok(path) => path,
        Err(_) => return Ok(false),
    };

    Ok(same_windows_path(&run_path, &startup_path))
}

#[cfg(target_os = "windows")]
pub fn set_start_on_boot(enabled: bool) -> Result<(), String> {
    if enabled {
        let exe_path = startup_exe_path()?;
        let quoted_path = format!("\"{}\"", exe_path.replace('"', ""));
        let output = reg_command()
            .args([
                "add",
                RUN_KEY,
                "/v",
                RUN_VALUE_NAME,
                "/t",
                "REG_SZ",
                "/d",
                &quoted_path,
                "/f",
            ])
            .output()
            .map_err(|err| format!("设置开机自启失败：{err}"))?;

        return command_result(output, "设置开机自启失败");
    }

    if read_run_value()?.is_none() {
        return Ok(());
    }

    let output = reg_command()
        .args(["delete", RUN_KEY, "/v", RUN_VALUE_NAME, "/f"])
        .output()
        .map_err(|err| format!("关闭开机自启失败：{err}"))?;

    command_result(output, "关闭开机自启失败")
}

#[cfg(target_os = "windows")]
fn reg_command() -> std::process::Command {
    use std::os::windows::process::CommandExt;

    const CREATE_NO_WINDOW: u32 = 0x08000000;

    let mut command = std::process::Command::new("reg.exe");
    command.creation_flags(CREATE_NO_WINDOW);
    command
}

#[cfg(target_os = "windows")]
fn read_run_value() -> Result<Option<String>, String> {
    let output = reg_command()
        .args(["query", RUN_KEY, "/v", RUN_VALUE_NAME])
        .output()
        .map_err(|err| format!("读取开机自启状态失败：{err}"))?;

    if !output.status.success() {
        return Ok(None);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(parse_run_value(&stdout))
}

#[cfg(target_os = "windows")]
fn parse_run_value(output: &str) -> Option<String> {
    for line in output.lines() {
        let line = line.trim_start();
        let Some(rest) = line.strip_prefix(RUN_VALUE_NAME) else {
            continue;
        };
        let rest = rest.trim_start();
        if !rest.starts_with("REG_") {
            continue;
        }
        let Some(data_start) = rest.find(char::is_whitespace) else {
            continue;
        };
        let data = rest[data_start..].trim();
        if !data.is_empty() {
            return Some(data.to_string());
        }
    }

    None
}

#[cfg(target_os = "windows")]
fn run_value_exe_path(value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }

    if let Some(unquoted) = value.strip_prefix('"') {
        let end = unquoted.find('"')?;
        let path = unquoted[..end].trim();
        return (!path.is_empty()).then(|| path.to_string());
    }

    value
        .split_whitespace()
        .next()
        .filter(|path| !path.is_empty())
        .map(ToString::to_string)
}

#[cfg(target_os = "windows")]
fn startup_exe_path() -> Result<String, String> {
    let current_path =
        std::env::current_exe().map_err(|err| format!("无法获取当前程序路径：{err}"))?;

    if cfg!(debug_assertions) {
        return release_exe_path_for_debug(&current_path)
            .map(|path| path.to_string_lossy().to_string());
    }

    Ok(current_path.to_string_lossy().to_string())
}

#[cfg(target_os = "windows")]
fn release_exe_path_for_debug(
    current_path: &std::path::Path,
) -> Result<std::path::PathBuf, String> {
    let file_name = current_path
        .file_name()
        .ok_or_else(|| "无法解析当前程序文件名".to_string())?;
    let debug_dir = current_path
        .parent()
        .ok_or_else(|| "无法解析当前程序目录".to_string())?;
    let target_dir = debug_dir
        .parent()
        .ok_or_else(|| "无法解析 target 目录".to_string())?;
    let release_path = target_dir.join("release").join(file_name);

    if !release_path.exists() {
        return Err(format!(
            "未找到 release 版可执行文件：{}。请先运行 cargo build --release 或 npm run tauri:build。",
            release_path.display()
        ));
    }

    Ok(release_path)
}

#[cfg(target_os = "windows")]
fn same_windows_path(left: &str, right: &str) -> bool {
    normalized_windows_path(left) == normalized_windows_path(right)
}

#[cfg(target_os = "windows")]
fn normalized_windows_path(value: &str) -> String {
    let trimmed = value.trim().trim_matches('"');
    std::path::PathBuf::from(trimmed)
        .canonicalize()
        .unwrap_or_else(|_| std::path::PathBuf::from(trimmed))
        .to_string_lossy()
        .replace('/', "\\")
        .to_lowercase()
}

#[cfg(target_os = "windows")]
fn command_result(output: std::process::Output, context: &str) -> Result<(), String> {
    if output.status.success() {
        return Ok(());
    }

    let detail = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let detail = if detail.is_empty() {
        String::from_utf8_lossy(&output.stdout).trim().to_string()
    } else {
        detail
    };

    if detail.is_empty() {
        Err(context.to_string())
    } else {
        Err(format!("{context}：{detail}"))
    }
}

#[cfg(not(target_os = "windows"))]
pub fn is_start_on_boot_enabled() -> Result<bool, String> {
    Ok(false)
}

#[cfg(not(target_os = "windows"))]
pub fn set_start_on_boot(enabled: bool) -> Result<(), String> {
    if enabled {
        Err("开机自启当前仅支持 Windows 版本".to_string())
    } else {
        Ok(())
    }
}

#[cfg(all(test, target_os = "windows"))]
mod tests {
    use super::*;

    #[test]
    fn parse_run_value_extracts_data() {
        let output = r#"
HKEY_CURRENT_USER\Software\Microsoft\Windows\CurrentVersion\Run
    PetDrawer    REG_SZ    "C:\Program Files\PetDrawer\PetDrawer.exe"
"#;

        assert_eq!(
            parse_run_value(output),
            Some(r#""C:\Program Files\PetDrawer\PetDrawer.exe""#.to_string())
        );
    }

    #[test]
    fn run_value_exe_path_handles_arguments() {
        assert_eq!(
            run_value_exe_path(r#""C:\Program Files\PetDrawer\PetDrawer.exe" --hidden"#),
            Some(r#"C:\Program Files\PetDrawer\PetDrawer.exe"#.to_string())
        );
    }
}
