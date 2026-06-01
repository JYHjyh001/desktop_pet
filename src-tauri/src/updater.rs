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
    let update_url = url
        .as_deref()
        .map(str::trim)
        .filter(|value| is_supported_url(value))
        .unwrap_or(GITHUB_RELEASES_URL);
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
    let tag = url
        .split("/releases/tag/")
        .nth(1)
        .map(|value| {
            value
                .split(|character| character == '?' || character == '#')
                .next()
                .unwrap_or(value)
                .trim_matches('/')
                .to_string()
        })
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| format!("无法从 GitHub 发布地址中解析版本号：{url}"))?;

    Ok(tag)
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

fn is_supported_url(value: &str) -> bool {
    let is_github_url = value.starts_with("https://github.com/")
        || value.starts_with("https://objects.githubusercontent.com/")
        || value.starts_with("https://api.github.com/");
    let is_project_url = value.contains(GITHUB_OWNER) && value.contains(GITHUB_REPO);

    is_github_url && is_project_url
}

#[cfg(target_os = "windows")]
fn open_url(url: &str) -> Result<(), String> {
    use std::os::windows::process::CommandExt;

    const CREATE_NO_WINDOW: u32 = 0x08000000;

    Command::new("cmd")
        .args(["/C", "start", "", url])
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .map(|_| ())
        .map_err(|err| err.to_string())
}

#[cfg(target_os = "macos")]
fn open_url(url: &str) -> Result<(), String> {
    Command::new("open")
        .arg(url)
        .spawn()
        .map(|_| ())
        .map_err(|err| err.to_string())
}

#[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
fn open_url(url: &str) -> Result<(), String> {
    Command::new("xdg-open")
        .arg(url)
        .spawn()
        .map(|_| ())
        .map_err(|err| err.to_string())
}
