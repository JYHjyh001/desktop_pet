use std::process::Command;

use serde::Serialize;
use tauri::AppHandle;

const GITHUB_OWNER: &str = "JYHjyh001";
const GITHUB_REPO: &str = "desktop_pet";
const GITHUB_RELEASES_URL: &str = "https://github.com/JYHjyh001/desktop_pet/releases/latest";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCheckResult {
    pub current_version: String,
    pub latest_version: Option<String>,
    pub update_url: Option<String>,
    pub release_url: Option<String>,
    pub asset_name: Option<String>,
    pub status: String,
    pub message: String,
}

#[derive(Debug)]
struct GitHubRelease {
    tag_name: String,
    html_url: String,
    assets: Vec<GitHubReleaseAsset>,
}

#[derive(Debug)]
struct GitHubReleaseAsset {
    name: String,
    browser_download_url: String,
}

pub fn check_for_update(app: &AppHandle) -> UpdateCheckResult {
    let current_version = app.package_info().version.to_string();

    let release = match fetch_latest_github_release() {
        Ok(release) => release,
        Err(err) => {
            return UpdateCheckResult {
                current_version,
                latest_version: None,
                update_url: Some(GITHUB_RELEASES_URL.to_string()),
                release_url: Some(GITHUB_RELEASES_URL.to_string()),
                asset_name: None,
                status: "error".to_string(),
                message: format!("检查 GitHub 更新失败：{err}。可以打开发布页手动查看。"),
            };
        }
    };

    let latest_version = normalize_version(&release.tag_name);
    let selected_asset = select_exe_asset(&release.assets);
    let update_url = selected_asset
        .map(|asset| asset.browser_download_url.clone())
        .or_else(|| Some(release.html_url.clone()));
    let asset_name = selected_asset.map(|asset| asset.name.clone());

    if is_newer_version(&latest_version, &current_version) {
        let message = if let Some(asset_name) = asset_name.as_deref() {
            format!("发现新版本 v{latest_version}，已找到 EXE 下载文件：{asset_name}。")
        } else {
            format!(
                "发现新版本 v{latest_version}，但该 Release 中没有 .exe 附件，请打开发布页查看。"
            )
        };

        return UpdateCheckResult {
            current_version,
            latest_version: Some(latest_version),
            update_url,
            release_url: Some(release.html_url),
            asset_name,
            status: "available".to_string(),
            message,
        };
    }

    if is_newer_version(&current_version, &latest_version) {
        return UpdateCheckResult {
            current_version: current_version.clone(),
            latest_version: Some(latest_version.clone()),
            update_url: Some(release.html_url.clone()),
            release_url: Some(release.html_url),
            asset_name,
            status: "latest".to_string(),
            message: format!(
                "当前版本 v{current_version} 高于已发布版本 v{latest_version}，无需更新。"
            ),
        };
    }

    UpdateCheckResult {
        current_version,
        latest_version: Some(latest_version.clone()),
        update_url: Some(release.html_url.clone()),
        release_url: Some(release.html_url),
        asset_name,
        status: "latest".to_string(),
        message: format!("当前已是最新版本 v{latest_version}。"),
    }
}

pub fn open_update_page(url: Option<String>) -> Result<(), String> {
    let update_url = match url.as_deref() {
        Some(value) => validate_update_url(value)?.to_string(),
        None => GITHUB_RELEASES_URL.to_string(),
    };

    open_url(&update_url).map_err(|err| format!("打开更新页面失败：{err}"))?;
    Ok(())
}

fn fetch_latest_github_release() -> Result<GitHubRelease, String> {
    let release_url = fetch_latest_release_url(GITHUB_RELEASES_URL)?;
    let tag_name = release_tag_from_url(&release_url)?;
    let latest_version = normalize_version(&tag_name);
    let asset_name = setup_asset_name(&latest_version);
    let browser_download_url = format!(
        "https://github.com/{GITHUB_OWNER}/{GITHUB_REPO}/releases/download/{tag_name}/{asset_name}"
    );

    Ok(GitHubRelease {
        html_url: format!(
            "https://github.com/{GITHUB_OWNER}/{GITHUB_REPO}/releases/tag/{tag_name}"
        ),
        tag_name,
        assets: vec![GitHubReleaseAsset {
            name: asset_name,
            browser_download_url,
        }],
    })
}

#[cfg(target_os = "windows")]
fn fetch_latest_release_url(url: &str) -> Result<String, String> {
    use std::os::windows::process::CommandExt;

    const CREATE_NO_WINDOW: u32 = 0x08000000;
    const UPDATE_REQUEST_URL_ENV: &str = "PETDRAWER_UPDATE_REQUEST_URL";
    const SCRIPT: &str = r#"
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
$ProgressPreference = 'SilentlyContinue'
$LatestUrl = $env:PETDRAWER_UPDATE_REQUEST_URL
if ([string]::IsNullOrWhiteSpace($LatestUrl)) {
  throw '更新检查地址为空'
}

[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
$request = [System.Net.HttpWebRequest]::Create($LatestUrl)
$request.Method = 'GET'
$request.AllowAutoRedirect = $false
$request.UserAgent = 'PetDrawer-Updater'
$request.Accept = 'text/html,application/xhtml+xml'

$response = $null
try {
  $response = $request.GetResponse()
} catch [System.Net.WebException] {
  if ($_.Exception.Response -eq $null) {
    throw
  }
  $response = $_.Exception.Response
}

try {
  $location = $response.Headers['Location']
  if ([string]::IsNullOrWhiteSpace($location)) {
    $location = $response.ResponseUri.AbsoluteUri
  }
  if ([string]::IsNullOrWhiteSpace($location)) {
    throw 'GitHub 最新发布地址为空'
  }

  if ($location.StartsWith('/')) {
    $uri = [Uri]$LatestUrl
    $location = $uri.Scheme + '://' + $uri.Host + $location
  }

  [Console]::Out.Write($location)
} finally {
  if ($response -ne $null) {
    $response.Close()
  }
}
"#;

    let output = Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            SCRIPT,
        ])
        .env(UPDATE_REQUEST_URL_ENV, url)
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|err| format!("无法启动 PowerShell 检查更新：{err}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        if stderr.is_empty() {
            return Err("获取 GitHub 最新发布地址失败".to_string());
        }

        return Err(stderr);
    }

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if stdout.is_empty() {
        return Err("GitHub 最新发布地址为空".to_string());
    }

    Ok(stdout)
}

#[cfg(not(target_os = "windows"))]
fn fetch_latest_release_url(url: &str) -> Result<String, String> {
    let output = std::process::Command::new("curl")
        .args([
            "-sIL",
            "--connect-timeout",
            "5",
            "--max-time",
            "10",
            "-o",
            "/dev/null",
            "-w",
            "%{url_effective}",
            url,
        ])
        .output()
        .map_err(|err| format!("无法启动 curl 检查更新：{err}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        if stderr.is_empty() {
            return Err("获取 GitHub 最新发布地址失败".to_string());
        }
        return Err(stderr);
    }

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if stdout.is_empty() {
        return Err("GitHub 最新发布地址为空".to_string());
    }

    Ok(stdout)
}

fn release_tag_from_url(url: &str) -> Result<String, String> {
    let parsed = validate_update_url(url)?;
    let segments = parsed
        .path_segments()
        .map(|segments| segments.collect::<Vec<_>>())
        .unwrap_or_default();

    match segments.as_slice() {
        [owner, repo, "releases", "tag", tag] if *owner == GITHUB_OWNER && *repo == GITHUB_REPO => {
            Ok((*tag).to_string())
        }
        _ => Err(format!("无法从 GitHub 发布地址中解析版本号：{url}")),
    }
}

#[cfg(target_os = "windows")]
fn setup_asset_name(version: &str) -> String {
    format!("PetDrawer_{version}_x64-setup.exe")
}

#[cfg(target_os = "macos")]
fn setup_asset_name(version: &str) -> String {
    format!("PetDrawer_{version}_aarch64.dmg")
}

#[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
fn setup_asset_name(version: &str) -> String {
    format!("PetDrawer_{version}_amd64.AppImage")
}

fn select_exe_asset(assets: &[GitHubReleaseAsset]) -> Option<&GitHubReleaseAsset> {
    #[cfg(target_os = "windows")]
    let ext = "exe";
    #[cfg(target_os = "macos")]
    let ext = "dmg";
    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    let ext = "AppImage";

    assets.iter().find(|asset| {
        let name = asset.name.to_lowercase();
        name.ends_with(ext)
    })
}

fn normalize_version(value: &str) -> String {
    value
        .trim()
        .trim_start_matches('v')
        .trim_start_matches('V')
        .to_string()
}

fn is_newer_version(latest: &str, current: &str) -> bool {
    let latest_parts = numeric_version_parts(latest);
    let current_parts = numeric_version_parts(current);
    let max_len = latest_parts.len().max(current_parts.len());

    for index in 0..max_len {
        let latest_part = *latest_parts.get(index).unwrap_or(&0);
        let current_part = *current_parts.get(index).unwrap_or(&0);

        if latest_part > current_part {
            return true;
        }

        if latest_part < current_part {
            return false;
        }
    }

    false
}

fn numeric_version_parts(value: &str) -> Vec<u64> {
    normalize_version(value)
        .split(|character: char| !character.is_ascii_digit())
        .filter(|part| !part.is_empty())
        .filter_map(|part| part.parse::<u64>().ok())
        .collect()
}

fn validate_update_url(value: &str) -> Result<reqwest::Url, String> {
    if value.is_empty() || value.chars().any(is_unsafe_url_character) {
        return Err("更新地址包含不允许的字符".to_string());
    }

    let parsed = reqwest::Url::parse(value).map_err(|_| "更新地址格式无效".to_string())?;
    if parsed.scheme() != "https"
        || parsed.host_str() != Some("github.com")
        || parsed.port().is_some()
        || !parsed.username().is_empty()
        || parsed.password().is_some()
        || parsed.query().is_some()
        || parsed.fragment().is_some()
    {
        return Err("更新地址必须是项目的 GitHub HTTPS Release 地址".to_string());
    }

    let segments = parsed
        .path_segments()
        .map(|segments| segments.collect::<Vec<_>>())
        .ok_or_else(|| "更新地址路径无效".to_string())?;
    let is_supported_path = match segments.as_slice() {
        [owner, repo, "releases", "latest"] => *owner == GITHUB_OWNER && *repo == GITHUB_REPO,
        [owner, repo, "releases", "tag", tag] => {
            *owner == GITHUB_OWNER && *repo == GITHUB_REPO && is_safe_path_segment(tag)
        }
        [owner, repo, "releases", "download", tag, asset_name] => {
            *owner == GITHUB_OWNER
                && *repo == GITHUB_REPO
                && is_safe_path_segment(tag)
                && is_safe_path_segment(asset_name)
        }
        _ => false,
    };

    if !is_supported_path {
        return Err("更新地址不属于当前项目允许的 GitHub Release 路径".to_string());
    }

    Ok(parsed)
}

fn is_unsafe_url_character(character: char) -> bool {
    character.is_whitespace()
        || character.is_control()
        || matches!(character, '&' | '|' | '^' | '<' | '>' | '"' | '\'' | '`')
}

fn is_safe_path_segment(value: &str) -> bool {
    !value.is_empty()
        && value.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-' | '+')
        })
}

#[cfg(target_os = "windows")]
fn open_url(url: &str) -> Result<(), String> {
    use std::ptr;
    use windows_sys::Win32::UI::Shell::ShellExecuteW;
    use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

    let operation = "open".encode_utf16().chain(Some(0)).collect::<Vec<_>>();
    let target = url.encode_utf16().chain(Some(0)).collect::<Vec<_>>();
    let result = unsafe {
        ShellExecuteW(
            ptr::null_mut(),
            operation.as_ptr(),
            target.as_ptr(),
            ptr::null(),
            ptr::null(),
            SW_SHOWNORMAL,
        )
    } as isize;

    if result > 32 {
        Ok(())
    } else {
        Err(format!("系统返回错误码 {result}"))
    }
}

#[cfg(target_os = "macos")]
fn open_url(url: &str) -> Result<(), String> {
    Command::new("open")
        .arg(url)
        .spawn()
        .map(|_| ())
        .map_err(|err| err.to_string())
}

#[cfg(test)]
mod tests {
    use super::{release_tag_from_url, validate_update_url};

    #[test]
    fn accepts_project_release_urls() {
        let urls = [
            "https://github.com/JYHjyh001/desktop_pet/releases/latest",
            "https://github.com/JYHjyh001/desktop_pet/releases/tag/v0.3.2",
            "https://github.com/JYHjyh001/desktop_pet/releases/download/v0.3.2/PetDrawer_0.3.2_x64-setup.exe",
        ];

        for url in urls {
            assert!(validate_update_url(url).is_ok(), "应接受合法地址：{url}");
        }
    }

    #[test]
    fn rejects_shell_metacharacters() {
        let url = "https://github.com/JYHjyh001/desktop_pet/releases/tag/v0.3.2&calc.exe";

        assert!(validate_update_url(url).is_err());
    }

    #[test]
    fn rejects_wrong_owner_or_repo() {
        let urls = [
            "https://github.com/attacker/desktop_pet/releases/tag/v0.3.2",
            "https://github.com/JYHjyh001/other_repo/releases/tag/v0.3.2",
        ];

        for url in urls {
            assert!(validate_update_url(url).is_err(), "应拒绝其他仓库：{url}");
        }
    }

    #[test]
    fn rejects_userinfo_fragment_and_abnormal_host() {
        let urls = [
            "https://attacker@github.com/JYHjyh001/desktop_pet/releases/tag/v0.3.2",
            "https://github.com/JYHjyh001/desktop_pet/releases/tag/v0.3.2#download",
            "https://github.com:8443/JYHjyh001/desktop_pet/releases/tag/v0.3.2",
            "https://github.com.evil.example/JYHjyh001/desktop_pet/releases/tag/v0.3.2",
        ];

        for url in urls {
            assert!(validate_update_url(url).is_err(), "应拒绝异常地址：{url}");
        }
    }

    #[test]
    fn extracts_tag_only_from_valid_project_tag_url() {
        assert_eq!(
            release_tag_from_url("https://github.com/JYHjyh001/desktop_pet/releases/tag/v0.3.2")
                .as_deref(),
            Ok("v0.3.2")
        );
        assert!(release_tag_from_url(
            "https://github.com/JYHjyh001/desktop_pet/releases/download/v0.3.2/setup.exe"
        )
        .is_err());
    }
}

#[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
fn open_url(url: &str) -> Result<(), String> {
    Command::new("xdg-open")
        .arg(url)
        .spawn()
        .map(|_| ())
        .map_err(|err| err.to_string())
}
