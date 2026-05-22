#[cfg(target_os = "windows")]
const RUN_KEY: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run";
#[cfg(target_os = "windows")]
const RUN_VALUE_NAME: &str = "PetDrawer";

#[cfg(target_os = "windows")]
pub fn is_start_on_boot_enabled() -> Result<bool, String> {
    let output = reg_command()
        .args(["query", RUN_KEY, "/v", RUN_VALUE_NAME])
        .output()
        .map_err(|err| format!("读取开机自启状态失败：{err}"))?;

    if !output.status.success() {
        return Ok(false);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout
        .lines()
        .any(|line| line.trim_start().starts_with(RUN_VALUE_NAME)))
}

#[cfg(target_os = "windows")]
pub fn set_start_on_boot(enabled: bool) -> Result<(), String> {
    if enabled {
        let exe_path = std::env::current_exe()
            .map_err(|err| format!("无法获取当前程序路径：{err}"))?
            .to_string_lossy()
            .to_string();
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

    if !is_start_on_boot_enabled()? {
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
