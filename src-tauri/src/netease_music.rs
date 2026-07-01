use std::{
    collections::BTreeMap,
    fs,
    path::PathBuf,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use aes::Aes128;
use base64::{engine::general_purpose, Engine as _};
use cipher::{block_padding::Pkcs7, BlockEncryptMut, KeyInit};
use ecb::Encryptor;
use md5::{Digest, Md5};
use qrcode::{render::svg, QrCode};
use reqwest::{
    blocking::Client,
    header::{HeaderMap, COOKIE, SET_COOKIE},
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use tauri::AppHandle;

use crate::app_data;

const NETEASE_API_BASE: &str = "https://music.163.com";
const NETEASE_EAPI_BASE: &str = "https://interface.music.163.com";
const QR_KEY_ENDPOINT: &str = "https://music.163.com/api/login/qrcode/unikey";
const QR_CHECK_ENDPOINT: &str = "https://music.163.com/api/login/qrcode/client/login";
const ACCOUNT_STATUS_ENDPOINT: &str = "https://music.163.com/api/w/nuser/account/get";
const USER_PLAYLIST_ENDPOINT: &str = "https://music.163.com/api/user/playlist";
const PLAYLIST_DETAIL_ENDPOINT: &str = "https://music.163.com/api/v6/playlist/detail";
const SONG_SEARCH_ENDPOINT: &str = "https://music.163.com/api/search/get/web";
const SONG_DETAIL_ENDPOINT: &str = "https://music.163.com/api/song/detail";
const SONG_LYRIC_ENDPOINT: &str = "https://music.163.com/api/song/lyric";
const SONG_PLAYBACK_URL_ENDPOINT: &str = "https://music.163.com/api/song/enhance/player/url/v1";
const NETEASE_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0 Safari/537.36";
const NETEASE_EAPI_USER_AGENT: &str = "NeteaseMusic 9.0.90/5038 (iPhone; iOS 16.2; zh_CN)";
const NETEASE_EAPI_KEY: &str = "e82ckenh8dichen8";
const QR_EXPIRE_SECONDS: u64 = 180;
const PLAYLIST_PAGE_LIMIT: usize = 100;
const MAX_NETEASE_PLAYLISTS: usize = 500;
const MAX_NETEASE_PLAYLIST_TRACK_PAGE_LIMIT: u64 = 300;
const MAX_NETEASE_SEARCH_LIMIT: u64 = 50;
const MAX_NETEASE_LYRIC_CHARS: usize = 80_000;
const CREDENTIAL_DIR_NAME: &str = "music-platforms";
const CREDENTIAL_FILE_NAME: &str = "netease-cloud.json";
const PENDING_FILE_NAME: &str = "netease-cloud-pending.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NeteaseMembershipInfo {
    pub active: bool,
    pub status_label: String,
    pub type_label: Option<String>,
    pub level_label: Option<String>,
    pub expire_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NeteaseLoginProfile {
    pub user_id: u64,
    pub nickname: String,
    pub avatar_url: Option<String>,
    pub membership: Option<NeteaseMembershipInfo>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NeteaseLoginStatus {
    pub logged_in: bool,
    pub profile: Option<NeteaseLoginProfile>,
    pub saved_at: Option<String>,
    pub checked_at: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NeteaseQrLogin {
    pub key: String,
    pub qr_url: String,
    pub qr_image: String,
    pub expires_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NeteaseQrCheckResult {
    pub code: i64,
    pub status: String,
    pub message: String,
    pub logged_in: bool,
    pub profile: Option<NeteaseLoginProfile>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NeteasePlaylistSummary {
    pub id: u64,
    pub name: String,
    pub track_count: u64,
    pub play_count: u64,
    pub subscribed_count: u64,
    pub cover_img_url: Option<String>,
    pub creator_nickname: Option<String>,
    pub subscribed: bool,
    pub update_time: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NeteasePlaylistTrack {
    pub id: u64,
    pub name: String,
    pub artists: Vec<String>,
    pub album: Option<String>,
    pub duration_ms: Option<u64>,
    pub cover_img_url: Option<String>,
    pub fee: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NeteasePlaylistDetail {
    pub playlist: NeteasePlaylistSummary,
    pub tracks: Vec<NeteasePlaylistTrack>,
    pub total_track_count: u64,
    pub truncated: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NeteaseSearchResult {
    pub keyword: String,
    pub tracks: Vec<NeteasePlaylistTrack>,
    pub total: u64,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NeteaseLyricsResult {
    pub song_id: u64,
    pub content: String,
    pub lrc_content: Option<String>,
    pub yrc_content: Option<String>,
    pub klyric_content: Option<String>,
    pub translated_content: Option<String>,
    pub source: String,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NeteasePlaybackUrl {
    pub song_id: u64,
    pub url: String,
    pub level: String,
    pub bitrate: Option<u64>,
    pub duration_ms: Option<u64>,
    pub file_type: Option<String>,
    pub size: Option<u64>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct NeteaseCredential {
    cookies: BTreeMap<String, String>,
    saved_at: String,
    profile: Option<NeteaseLoginProfile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct NeteasePendingLogin {
    key: String,
    cookies: BTreeMap<String, String>,
    created_at: String,
    expires_at: String,
}

struct NeteaseResponse {
    body: Value,
    set_cookies: Vec<String>,
}

pub fn create_qr_login(app: &AppHandle) -> Result<NeteaseQrLogin, String> {
    let mut cookies = BTreeMap::new();
    let response = post_eapi(
        "/api/login/qrcode/unikey",
        json!({ "type": 3 }),
        &mut cookies,
    )
    .or_else(|_| {
        let qr_key_url = timestamp_url(QR_KEY_ENDPOINT);
        post_form(&qr_key_url, &[("type", "3")], None)
    })?;
    let key = response
        .body
        .get("unikey")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "网易云二维码 key 返回为空".to_string())?
        .to_string();
    merge_set_cookies(&mut cookies, &response.set_cookies);
    let expires_at = (current_epoch_seconds() + QR_EXPIRE_SECONDS).to_string();
    write_pending_login(
        app,
        &NeteasePendingLogin {
            key: key.clone(),
            cookies: cookies.clone(),
            created_at: app_data::now_seconds(),
            expires_at: expires_at.clone(),
        },
    )?;

    let qr_url = format!(
        "{NETEASE_API_BASE}/login?codekey={key}&chainId={}",
        generate_web_login_chain_id(&cookies)
    );
    let qr_image = create_qr_image(&qr_url)?;

    Ok(NeteaseQrLogin {
        key,
        qr_url,
        qr_image,
        expires_at,
    })
}

pub fn check_qr_login(app: &AppHandle, key: String) -> Result<NeteaseQrCheckResult, String> {
    let key = key.trim();
    if key.is_empty() || key.len() > 128 {
        return Err("网易云二维码 key 无效".to_string());
    }

    let mut pending = read_pending_login(app)?
        .filter(|pending| pending.key == key)
        .unwrap_or_else(|| NeteasePendingLogin {
            key: key.to_string(),
            cookies: BTreeMap::new(),
            created_at: app_data::now_seconds(),
            expires_at: (current_epoch_seconds() + QR_EXPIRE_SECONDS).to_string(),
        });

    let pending_cookie_header = cookie_header(&pending.cookies);
    let response = post_eapi(
        "/api/login/qrcode/client/login",
        json!({ "key": key, "type": 3 }),
        &mut pending.cookies,
    )
    .or_else(|_| {
        let qr_check_url = timestamp_url(QR_CHECK_ENDPOINT);
        post_form(
            &qr_check_url,
            &[("key", key), ("type", "3")],
            Some(&pending_cookie_header),
        )
    })?;
    merge_set_cookies(&mut pending.cookies, &response.set_cookies);
    write_pending_login(app, &pending)?;

    let code = response
        .body
        .get("code")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let message = response
        .body
        .get("message")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| qr_status_message(code))
        .to_string();

    if code == 803 {
        let mut cookies = read_credential(app)?
            .map(|credential| credential.cookies)
            .unwrap_or_default();
        merge_cookies(&mut cookies, &pending.cookies);
        merge_set_cookies(&mut cookies, &response.set_cookies);

        if !cookies.contains_key("MUSIC_U") {
            return Ok(NeteaseQrCheckResult {
                code,
                status: "authorized_without_cookie".to_string(),
                message: "网易云已确认登录，但没有返回可保存的登录 Cookie。".to_string(),
                logged_in: false,
                profile: None,
            });
        }

        let mut credential = NeteaseCredential {
            cookies,
            saved_at: app_data::now_seconds(),
            profile: None,
        };
        write_credential(app, &credential)?;
        clear_pending_login(app)?;

        let status = login_status(app)?;
        credential.profile = status.profile.clone();
        write_credential(app, &credential)?;

        if !status.logged_in {
            return Ok(NeteaseQrCheckResult {
                code,
                status: "authorized".to_string(),
                message: "网易云授权已完成，已保存本机登录状态；账号资料稍后刷新。".to_string(),
                logged_in: true,
                profile: status.profile,
            });
        }

        return Ok(NeteaseQrCheckResult {
            code,
            status: "authorized".to_string(),
            message: status.message,
            logged_in: true,
            profile: status.profile,
        });
    }

    if code == 800 {
        clear_pending_login(app)?;
    }

    Ok(NeteaseQrCheckResult {
        code,
        status: qr_status_name(code).to_string(),
        message,
        logged_in: false,
        profile: None,
    })
}

pub fn login_status(app: &AppHandle) -> Result<NeteaseLoginStatus, String> {
    let Some(mut credential) = read_credential(app)? else {
        return Ok(NeteaseLoginStatus {
            logged_in: false,
            profile: None,
            saved_at: None,
            checked_at: app_data::now_seconds(),
            message: "尚未登录网易云音乐。".to_string(),
        });
    };

    let cookie_header = cookie_header(&credential.cookies);
    let response = post_form(ACCOUNT_STATUS_ENDPOINT, &[], Some(&cookie_header))?;
    let profile = extract_profile(&response.body);
    let checked_at = app_data::now_seconds();

    if let Some(profile) = profile {
        credential.profile = Some(profile.clone());
        write_credential(app, &credential)?;

        return Ok(NeteaseLoginStatus {
            logged_in: true,
            profile: Some(profile),
            saved_at: Some(credential.saved_at),
            checked_at,
            message: "网易云音乐已登录。".to_string(),
        });
    }

    if credential.cookies.contains_key("MUSIC_U") {
        return Ok(NeteaseLoginStatus {
            logged_in: true,
            profile: credential.profile,
            saved_at: Some(credential.saved_at),
            checked_at,
            message: "网易云登录凭据已保存，账号资料暂时不可用。".to_string(),
        });
    }

    Ok(NeteaseLoginStatus {
        logged_in: false,
        profile: credential.profile,
        saved_at: Some(credential.saved_at),
        checked_at,
        message: "网易云登录状态不可用，可能已过期，请重新扫码。".to_string(),
    })
}

pub fn clear_login(app: &AppHandle) -> Result<NeteaseLoginStatus, String> {
    let path = credential_file(app)?;
    match fs::remove_file(&path) {
        Ok(()) => {}
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
        Err(err) => return Err(format!("清除网易云登录状态失败：{err}")),
    }
    clear_pending_login(app)?;

    Ok(NeteaseLoginStatus {
        logged_in: false,
        profile: None,
        saved_at: None,
        checked_at: app_data::now_seconds(),
        message: "已清除本机网易云登录状态。".to_string(),
    })
}

pub fn list_playlists(app: &AppHandle) -> Result<Vec<NeteasePlaylistSummary>, String> {
    let mut credential = require_logged_in_credential(app)?;
    let user_id = credential
        .profile
        .as_ref()
        .map(|profile| profile.user_id)
        .ok_or_else(|| "网易云账号资料暂时不可用，请先刷新登录状态或重新扫码。".to_string())?;
    let mut playlists = Vec::new();
    let mut offset = 0usize;

    while playlists.len() < MAX_NETEASE_PLAYLISTS {
        let cookie = cookie_header(&credential.cookies);
        let response = request_user_playlists(user_id, offset, &cookie, &mut credential.cookies)?;
        merge_set_cookies(&mut credential.cookies, &response.set_cookies);
        ensure_success_code(&response.body, "网易云歌单读取")?;

        let Some(items) = response.body.get("playlist").and_then(Value::as_array) else {
            return Err("网易云歌单响应缺少 playlist 字段。".to_string());
        };

        let before_count = playlists.len();
        for item in items {
            if let Some(playlist) = playlist_summary_from_value(item) {
                playlists.push(playlist);
            }

            if playlists.len() >= MAX_NETEASE_PLAYLISTS {
                break;
            }
        }

        let more = response
            .body
            .get("more")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        if !more || items.is_empty() || playlists.len() == before_count {
            break;
        }

        offset += PLAYLIST_PAGE_LIMIT;
    }

    write_credential(app, &credential)?;
    Ok(playlists)
}

pub fn playlist_detail(
    app: &AppHandle,
    playlist_id: u64,
    page: Option<u64>,
    limit: Option<u64>,
) -> Result<NeteasePlaylistDetail, String> {
    if playlist_id == 0 {
        return Err("网易云歌单 ID 无效。".to_string());
    }

    let (page, limit, offset) = normalize_playlist_detail_paging(page, limit);
    let mut credential = require_logged_in_credential(app)?;
    let mut cookie = cookie_header(&credential.cookies);
    let response = request_playlist_detail(playlist_id, &cookie, &mut credential.cookies)?;
    merge_set_cookies(&mut credential.cookies, &response.set_cookies);
    ensure_success_code(&response.body, "网易云歌单详情读取")?;

    let playlist_value = response
        .body
        .get("playlist")
        .ok_or_else(|| "网易云歌单详情响应缺少 playlist 字段。".to_string())?;
    let playlist = playlist_summary_from_value(playlist_value)
        .ok_or_else(|| "网易云歌单详情缺少有效歌单信息。".to_string())?;
    let track_values = playlist_value
        .get("tracks")
        .and_then(Value::as_array)
        .or_else(|| response.body.get("songs").and_then(Value::as_array))
        .ok_or_else(|| "网易云歌单详情响应缺少 tracks 字段。".to_string())?;
    let track_ids = playlist_value
        .get("trackIds")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| value_as_u64(item.get("id")))
                .collect::<Vec<_>>()
        })
        .filter(|ids| !ids.is_empty())
        .unwrap_or_else(|| {
            track_values
                .iter()
                .filter_map(|track| value_as_u64(track.get("id")))
                .collect::<Vec<_>>()
        });
    let total_track_count = value_as_u64(playlist_value.get("trackCount"))
        .or_else(|| u64::try_from(track_ids.len()).ok())
        .unwrap_or_else(|| track_values.len() as u64);
    let end = offset.saturating_add(limit);
    let tracks = if track_values.len() >= end {
        track_values[offset..end]
            .iter()
            .filter_map(track_summary_from_value)
            .collect::<Vec<_>>()
    } else {
        let page_track_ids = track_ids
            .iter()
            .skip(offset)
            .take(limit)
            .copied()
            .collect::<Vec<_>>();

        if page_track_ids.is_empty() {
            Vec::new()
        } else {
            cookie = cookie_header(&credential.cookies);
            let detail_response =
                request_song_detail(&page_track_ids, &cookie, &mut credential.cookies)?;
            merge_set_cookies(&mut credential.cookies, &detail_response.set_cookies);
            ensure_success_code(&detail_response.body, "网易云歌曲详情读取")?;
            parse_song_detail_tracks(&detail_response.body, &page_track_ids)
        }
    };
    write_credential(app, &credential)?;

    let page_source_count = track_ids
        .iter()
        .skip(offset)
        .take(limit)
        .count()
        .max(tracks.len());
    let loaded_end = u64::try_from(offset)
        .unwrap_or(u64::MAX)
        .saturating_add(u64::try_from(page_source_count).unwrap_or(u64::MAX));
    let truncated = loaded_end < total_track_count;
    let message = if truncated {
        format!(
            "已读取第 {} 页 {} 首歌曲摘要，共 {} 首，可继续加载。",
            page,
            tracks.len(),
            total_track_count,
        )
    } else {
        format!(
            "已读取第 {} 页 {} 首歌曲摘要，共 {} 首。",
            page,
            tracks.len(),
            total_track_count,
        )
    };

    Ok(NeteasePlaylistDetail {
        playlist,
        tracks,
        total_track_count,
        truncated,
        message,
    })
}

pub fn search_songs(keyword: String, page: u64, limit: u64) -> Result<NeteaseSearchResult, String> {
    let keyword = normalize_search_keyword(&keyword, 80)?;
    let page = page.clamp(1, 20);
    let limit = limit.clamp(1, MAX_NETEASE_SEARCH_LIMIT);
    let offset = (page - 1) * limit;
    let response = request_song_search(&keyword, offset, limit)?;
    ensure_success_code(&response.body, "网易云搜索")?;

    let result = response
        .body
        .get("result")
        .ok_or_else(|| "网易云搜索响应缺少 result 字段。".to_string())?;
    let total = value_as_u64(result.get("songCount")).unwrap_or_default();
    let tracks = result
        .get("songs")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(track_summary_from_value)
                .take(limit as usize)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let message = if tracks.is_empty() {
        format!("没有搜索到“{keyword}”。")
    } else {
        format!("已搜索到 {} 首网易云歌曲。", tracks.len())
    };

    Ok(NeteaseSearchResult {
        keyword,
        tracks,
        total,
        message,
    })
}

pub fn song_lyrics(app: &AppHandle, song_id: u64) -> Result<NeteaseLyricsResult, String> {
    if song_id == 0 {
        return Err("网易云歌曲 ID 无效。".to_string());
    }

    let mut credential = require_logged_in_credential(app)?;
    let cookie = cookie_header(&credential.cookies);
    let response = request_song_lyrics(song_id, &cookie, &mut credential.cookies)?;
    merge_set_cookies(&mut credential.cookies, &response.set_cookies);
    write_credential(app, &credential)?;
    ensure_success_code(&response.body, "网易云歌词读取")?;

    let lrc_content = lyric_text(&response.body, "lrc");
    let yrc_content = lyric_text(&response.body, "yrc");
    let klyric_content = lyric_text(&response.body, "klyric");
    let translated_content = lyric_text(&response.body, "tlyric");
    let mut warnings = Vec::new();

    if response
        .body
        .get("nolyric")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        warnings.push("网易云返回纯音乐标记，没有歌词。".to_string());
    }
    if response
        .body
        .get("uncollected")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        warnings.push("网易云暂未收录该歌曲歌词。".to_string());
    }

    let lrc_content = lrc_content.map(|value| clamp_text(value, MAX_NETEASE_LYRIC_CHARS));
    let yrc_content = yrc_content.map(|value| clamp_text(value, MAX_NETEASE_LYRIC_CHARS));
    let klyric_content = klyric_content.map(|value| clamp_text(value, MAX_NETEASE_LYRIC_CHARS));
    let content = lrc_content
        .clone()
        .or_else(|| yrc_content.clone())
        .or_else(|| klyric_content.clone())
        .unwrap_or_default();
    let translated_content =
        translated_content.map(|value| clamp_text(value, MAX_NETEASE_LYRIC_CHARS));

    if content.trim().is_empty()
        && yrc_content.as_deref().unwrap_or("").trim().is_empty()
        && klyric_content.as_deref().unwrap_or("").trim().is_empty()
        && translated_content
            .as_deref()
            .unwrap_or("")
            .trim()
            .is_empty()
    {
        return Err("网易云没有返回可展示的歌词。".to_string());
    }

    Ok(NeteaseLyricsResult {
        song_id,
        content,
        lrc_content,
        yrc_content,
        klyric_content,
        translated_content,
        source: "netease".to_string(),
        warnings,
    })
}

pub fn song_playback_url(
    app: &AppHandle,
    song_id: u64,
    level: Option<String>,
) -> Result<NeteasePlaybackUrl, String> {
    if song_id == 0 {
        return Err("网易云歌曲 ID 无效。".to_string());
    }

    let level = normalize_playback_level(level.as_deref());
    let mut credential = require_logged_in_credential(app)?;
    let cookie = cookie_header(&credential.cookies);
    let response = request_song_playback_url(song_id, &level, &cookie, &mut credential.cookies)?;
    merge_set_cookies(&mut credential.cookies, &response.set_cookies);
    write_credential(app, &credential)?;
    ensure_success_code(&response.body, "网易云播放链接获取")?;

    let items = response
        .body
        .get("data")
        .and_then(Value::as_array)
        .ok_or_else(|| "网易云播放链接响应缺少 data 字段。".to_string())?;
    let item = items
        .iter()
        .find(|item| value_as_u64(item.get("id")) == Some(song_id))
        .or_else(|| items.first())
        .ok_or_else(|| "网易云没有返回播放链接数据。".to_string())?;

    let item_code = value_as_i64(item.get("code")).unwrap_or(0);
    let url = safe_http_url(item.get("url"));
    if item_code != 200 || url.is_none() {
        let message = playback_denied_message(item, item_code);
        return Err(message);
    }

    let url = url.unwrap_or_default();
    Ok(NeteasePlaybackUrl {
        song_id,
        url,
        level: trimmed_string(item.get("level"), 40).unwrap_or(level),
        bitrate: value_as_u64(item.get("br")),
        duration_ms: value_as_u64(item.get("time")),
        file_type: trimmed_string(item.get("type"), 24),
        size: value_as_u64(item.get("size")),
        message: "已获取网易云临时播放链接。".to_string(),
    })
}

fn require_logged_in_credential(app: &AppHandle) -> Result<NeteaseCredential, String> {
    let Some(mut credential) = read_credential(app)? else {
        return Err("请先登录网易云音乐。".to_string());
    };

    if !credential.cookies.contains_key("MUSIC_U") {
        return Err("网易云登录状态不可用，请重新扫码登录。".to_string());
    }

    if credential.profile.is_none() {
        if let Ok(status) = login_status(app) {
            if let Some(profile) = status.profile {
                credential.profile = Some(profile);
                write_credential(app, &credential)?;
            }
        }
    }

    Ok(credential)
}

fn request_user_playlists(
    user_id: u64,
    offset: usize,
    cookie: &str,
    cookies: &mut BTreeMap<String, String>,
) -> Result<NeteaseResponse, String> {
    let limit = PLAYLIST_PAGE_LIMIT.to_string();
    let offset_text = offset.to_string();
    let user_id_text = user_id.to_string();
    let direct = post_form_values(
        &timestamp_url(USER_PLAYLIST_ENDPOINT),
        &[
            ("uid", user_id_text.clone()),
            ("limit", limit.clone()),
            ("offset", offset_text.clone()),
            ("includeVideo", "true".to_string()),
        ],
        Some(cookie),
    );

    if let Ok(response) = direct {
        merge_set_cookies(cookies, &response.set_cookies);
        if ensure_success_code(&response.body, "网易云歌单读取").is_ok()
            && response.body.get("playlist").is_some()
        {
            return Ok(response);
        }
    }

    post_eapi(
        "/api/user/playlist",
        json!({
            "uid": user_id_text,
            "limit": PLAYLIST_PAGE_LIMIT,
            "offset": offset,
            "includeVideo": true
        }),
        cookies,
    )
}

fn request_playlist_detail(
    playlist_id: u64,
    cookie: &str,
    cookies: &mut BTreeMap<String, String>,
) -> Result<NeteaseResponse, String> {
    let playlist_id_text = playlist_id.to_string();
    let direct = post_form_values(
        &timestamp_url(PLAYLIST_DETAIL_ENDPOINT),
        &[
            ("id", playlist_id_text.clone()),
            ("n", "100000".to_string()),
            ("s", "8".to_string()),
        ],
        Some(cookie),
    );

    if let Ok(response) = direct {
        merge_set_cookies(cookies, &response.set_cookies);
        if ensure_success_code(&response.body, "网易云歌单详情读取").is_ok()
            && response.body.get("playlist").is_some()
        {
            return Ok(response);
        }
    }

    post_eapi(
        "/api/v6/playlist/detail",
        json!({
            "id": playlist_id_text,
            "n": 100000,
            "s": 8
        }),
        cookies,
    )
}

fn normalize_playlist_detail_paging(page: Option<u64>, limit: Option<u64>) -> (u64, usize, usize) {
    let page = page.unwrap_or(1).clamp(1, 200);
    let limit = limit
        .unwrap_or(MAX_NETEASE_PLAYLIST_TRACK_PAGE_LIMIT)
        .clamp(1, MAX_NETEASE_PLAYLIST_TRACK_PAGE_LIMIT);
    let offset = page.saturating_sub(1).saturating_mul(limit);
    (
        page,
        usize::try_from(limit).unwrap_or(MAX_NETEASE_PLAYLIST_TRACK_PAGE_LIMIT as usize),
        usize::try_from(offset).unwrap_or(usize::MAX),
    )
}

fn request_song_detail(
    song_ids: &[u64],
    cookie: &str,
    cookies: &mut BTreeMap<String, String>,
) -> Result<NeteaseResponse, String> {
    let ids_text = serde_json::to_string(song_ids).map_err(|err| err.to_string())?;
    let direct = post_form_values(
        &timestamp_url(SONG_DETAIL_ENDPOINT),
        &[("ids", ids_text.clone())],
        Some(cookie),
    );

    if let Ok(response) = direct {
        merge_set_cookies(cookies, &response.set_cookies);
        if ensure_success_code(&response.body, "网易云歌曲详情读取").is_ok()
            && response.body.get("songs").is_some()
        {
            return Ok(response);
        }
    }

    post_eapi(
        "/api/song/detail",
        json!({
            "ids": ids_text,
        }),
        cookies,
    )
}

fn parse_song_detail_tracks(body: &Value, ordered_ids: &[u64]) -> Vec<NeteasePlaylistTrack> {
    let Some(items) = body.get("songs").and_then(Value::as_array) else {
        return Vec::new();
    };

    let mut tracks_by_id = BTreeMap::new();
    for item in items {
        if let Some(track) = track_summary_from_value(item) {
            tracks_by_id.insert(track.id, track);
        }
    }

    ordered_ids
        .iter()
        .filter_map(|id| tracks_by_id.remove(id))
        .collect()
}

fn request_song_search(keyword: &str, offset: u64, limit: u64) -> Result<NeteaseResponse, String> {
    post_form_values(
        &timestamp_url(SONG_SEARCH_ENDPOINT),
        &[
            ("s", keyword.to_string()),
            ("type", "1".to_string()),
            ("offset", offset.to_string()),
            ("limit", limit.to_string()),
        ],
        None,
    )
}

fn request_song_lyrics(
    song_id: u64,
    cookie: &str,
    cookies: &mut BTreeMap<String, String>,
) -> Result<NeteaseResponse, String> {
    let song_id_text = song_id.to_string();
    let direct = post_form_values(
        &timestamp_url(SONG_LYRIC_ENDPOINT),
        &[
            ("id", song_id_text.clone()),
            ("tv", "-1".to_string()),
            ("lv", "-1".to_string()),
            ("rv", "-1".to_string()),
            ("kv", "-1".to_string()),
            ("_nmclfl", "1".to_string()),
        ],
        Some(cookie),
    );

    if let Ok(response) = direct {
        merge_set_cookies(cookies, &response.set_cookies);
        if ensure_success_code(&response.body, "网易云歌词读取").is_ok() {
            return Ok(response);
        }
    }

    post_eapi(
        "/api/song/lyric",
        json!({
            "id": song_id_text,
            "tv": -1,
            "lv": -1,
            "rv": -1,
            "kv": -1,
            "_nmclfl": 1
        }),
        cookies,
    )
}

fn request_song_playback_url(
    song_id: u64,
    level: &str,
    cookie: &str,
    cookies: &mut BTreeMap<String, String>,
) -> Result<NeteaseResponse, String> {
    let song_id_text = song_id.to_string();
    let ids = format!("[{song_id}]");
    let direct = post_form_values(
        &timestamp_url(SONG_PLAYBACK_URL_ENDPOINT),
        &[
            ("ids", ids.clone()),
            ("level", level.to_string()),
            ("encodeType", "flac".to_string()),
        ],
        Some(cookie),
    );

    if let Ok(response) = direct {
        merge_set_cookies(cookies, &response.set_cookies);
        if ensure_success_code(&response.body, "网易云播放链接获取").is_ok()
            && response.body.get("data").is_some()
        {
            return Ok(response);
        }
    }

    post_eapi(
        "/api/song/enhance/player/url/v1",
        json!({
            "ids": format!("[{song_id_text}]"),
            "level": level,
            "encodeType": "flac"
        }),
        cookies,
    )
}

fn ensure_success_code(body: &Value, context: &str) -> Result<(), String> {
    let Some(code) = value_as_i64(body.get("code")) else {
        return Ok(());
    };

    if code == 200 {
        return Ok(());
    }

    let message = body
        .get("message")
        .or_else(|| body.get("msg"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("网易云接口返回异常");
    Err(format!("{context}失败：{message}（code {code}）"))
}

fn lyric_text(body: &Value, key: &str) -> Option<String> {
    body.get(key)
        .and_then(|value| value.get("lyric"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn clamp_text(value: String, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value;
    }

    value.chars().take(max_chars).collect()
}

fn normalize_playback_level(value: Option<&str>) -> String {
    match value.unwrap_or("standard").trim().to_lowercase().as_str() {
        "standard" | "exhigh" | "lossless" | "hires" | "jyeffect" | "sky" | "jymaster" => {
            value.unwrap_or("standard").trim().to_lowercase()
        }
        _ => "standard".to_string(),
    }
}

fn normalize_search_keyword(value: &str, max_chars: usize) -> Result<String, String> {
    let normalized = value.split_whitespace().collect::<Vec<_>>().join(" ");
    let normalized = normalized.trim();
    if normalized.is_empty() {
        return Err("请输入网易云搜索关键词。".to_string());
    }

    Ok(normalized.chars().take(max_chars).collect())
}

fn playback_denied_message(item: &Value, code: i64) -> String {
    let message = item
        .get("message")
        .or_else(|| item.get("msg"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty());
    if let Some(message) = message {
        return format!("网易云暂不能播放该歌曲：{message}（code {code}）");
    }

    if code == 404 {
        return "网易云暂不能播放该歌曲：暂无版权或资源不可用。".to_string();
    }

    if code == 403 {
        return "网易云暂不能播放该歌曲：可能需要会员、地区权限或客户端播放。".to_string();
    }

    format!("网易云暂不能播放该歌曲，接口未返回可访问音频链接（code {code}）。")
}

fn playlist_summary_from_value(value: &Value) -> Option<NeteasePlaylistSummary> {
    let id = value_as_u64(value.get("id"))?;
    let name = trimmed_string(value.get("name"), 120)?;
    let creator_nickname = value
        .get("creator")
        .and_then(|creator| trimmed_string(creator.get("nickname"), 80));

    Some(NeteasePlaylistSummary {
        id,
        name,
        track_count: value_as_u64(value.get("trackCount")).unwrap_or(0),
        play_count: value_as_u64(value.get("playCount")).unwrap_or(0),
        subscribed_count: value_as_u64(value.get("subscribedCount")).unwrap_or(0),
        cover_img_url: safe_http_url(value.get("coverImgUrl")),
        creator_nickname,
        subscribed: value_as_bool(value.get("subscribed")).unwrap_or(false),
        update_time: value_as_u64(value.get("updateTime")),
    })
}

fn track_summary_from_value(value: &Value) -> Option<NeteasePlaylistTrack> {
    let id = value_as_u64(value.get("id"))?;
    let name = trimmed_string(value.get("name"), 160)?;
    let album_value = value.get("al").or_else(|| value.get("album"));
    let album = album_value.and_then(|album| trimmed_string(album.get("name"), 120));
    let cover_img_url = album_value.and_then(|album| {
        safe_http_url(album.get("picUrl")).or_else(|| safe_http_url(album.get("pic")))
    });
    let artists = artist_names(value);

    Some(NeteasePlaylistTrack {
        id,
        name,
        artists,
        album,
        duration_ms: value_as_u64(value.get("dt")).or_else(|| value_as_u64(value.get("duration"))),
        cover_img_url,
        fee: value_as_u64(value.get("fee")),
    })
}

fn artist_names(track: &Value) -> Vec<String> {
    track
        .get("ar")
        .or_else(|| track.get("artists"))
        .and_then(Value::as_array)
        .map(|artists| {
            artists
                .iter()
                .filter_map(|artist| trimmed_string(artist.get("name"), 80))
                .take(6)
                .collect::<Vec<_>>()
        })
        .filter(|artists| !artists.is_empty())
        .unwrap_or_default()
}

fn safe_http_url(value: Option<&Value>) -> Option<String> {
    trimmed_string(value, 500)
        .filter(|url| url.starts_with("http://") || url.starts_with("https://"))
}

fn find_value_by_keys<'a>(value: &'a Value, keys: &[&str]) -> Option<&'a Value> {
    match value {
        Value::Object(map) => {
            for key in keys {
                if let Some((_, matched)) = map
                    .iter()
                    .find(|(current_key, _)| current_key.eq_ignore_ascii_case(key))
                {
                    return Some(matched);
                }
            }
            map.values().find_map(|item| find_value_by_keys(item, keys))
        }
        Value::Array(items) => items
            .iter()
            .find_map(|item| find_value_by_keys(item, keys)),
        _ => None,
    }
}

fn find_string_by_keys(value: &Value, keys: &[&str], max_chars: usize) -> Option<String> {
    trimmed_string(find_value_by_keys(value, keys), max_chars)
}

fn find_u64_by_keys(value: &Value, keys: &[&str]) -> Option<u64> {
    value_as_u64(find_value_by_keys(value, keys))
}

fn normalize_epoch_seconds(value: u64) -> Option<String> {
    if value == 0 {
        return None;
    }

    let seconds = if value > 10_000_000_000 {
        value / 1000
    } else {
        value
    };
    Some(seconds.to_string())
}

fn trimmed_string(value: Option<&Value>, max_chars: usize) -> Option<String> {
    value?
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.chars().take(max_chars).collect())
}

fn value_as_bool(value: Option<&Value>) -> Option<bool> {
    match value? {
        Value::Bool(value) => Some(*value),
        Value::Number(value) => value.as_u64().map(|number| number != 0),
        Value::String(value) => match value.trim() {
            "true" | "1" => Some(true),
            "false" | "0" => Some(false),
            _ => None,
        },
        _ => None,
    }
}

fn value_as_i64(value: Option<&Value>) -> Option<i64> {
    match value? {
        Value::Number(value) => value
            .as_i64()
            .or_else(|| value.as_u64().and_then(|number| i64::try_from(number).ok())),
        Value::String(value) => value.trim().parse().ok(),
        _ => None,
    }
}

fn value_as_u64(value: Option<&Value>) -> Option<u64> {
    match value? {
        Value::Number(value) => value.as_u64().or_else(|| {
            value
                .as_i64()
                .filter(|number| *number >= 0)
                .and_then(|number| u64::try_from(number).ok())
        }),
        Value::String(value) => value.trim().parse().ok(),
        _ => None,
    }
}

fn create_qr_image(qr_url: &str) -> Result<String, String> {
    let code =
        QrCode::new(qr_url.as_bytes()).map_err(|err| format!("生成网易云二维码失败：{err}"))?;
    let image = code
        .render::<svg::Color>()
        .min_dimensions(220, 220)
        .quiet_zone(true)
        .build();
    Ok(format!(
        "data:image/svg+xml;base64,{}",
        general_purpose::STANDARD.encode(image.as_bytes())
    ))
}

fn post_eapi(
    uri: &str,
    mut data: Value,
    cookies: &mut BTreeMap<String, String>,
) -> Result<NeteaseResponse, String> {
    let header = ensure_eapi_cookie_header(cookies, uri);
    if let Value::Object(ref mut object) = data {
        object.insert("header".to_string(), Value::Object(header.clone()));
    } else {
        let mut object = Map::new();
        object.insert("data".to_string(), data);
        object.insert("header".to_string(), Value::Object(header.clone()));
        data = Value::Object(object);
    }

    let data_text = serde_json::to_string(&data).map_err(|err| err.to_string())?;
    let encrypted_params = eapi_encrypt(uri, &data_text)?;
    let eapi_path = uri
        .strip_prefix("/api/")
        .ok_or_else(|| "网易云 eapi 路径不合法".to_string())?;
    let url = format!("{NETEASE_EAPI_BASE}/eapi/{eapi_path}");
    let cookie = map_to_cookie_header(&header);

    let client = Client::builder()
        .timeout(Duration::from_secs(20))
        .user_agent(NETEASE_EAPI_USER_AGENT)
        .build()
        .map_err(|err| format!("创建网易云 eapi 请求客户端失败：{err}"))?;
    let response = client
        .post(url)
        .header("Referer", NETEASE_API_BASE)
        .header("Origin", NETEASE_API_BASE)
        .header("Accept", "application/json, text/plain, */*")
        .header(COOKIE, cookie)
        .form(&[("params", encrypted_params)])
        .send()
        .map_err(|err| format!("请求网易云 eapi 失败：{err}"))?;
    let status = response.status();
    let set_cookies = collect_set_cookies(response.headers());
    let body: Value = response
        .json()
        .map_err(|err| format!("解析网易云 eapi 响应失败：{err}"))?;

    if !status.is_success() {
        return Err(format!("网易云 eapi 返回 HTTP {status}"));
    }

    Ok(NeteaseResponse { body, set_cookies })
}

fn post_form(
    url: &str,
    form: &[(&str, &str)],
    cookie_header: Option<&str>,
) -> Result<NeteaseResponse, String> {
    let client = Client::builder()
        .timeout(Duration::from_secs(20))
        .user_agent(NETEASE_USER_AGENT)
        .build()
        .map_err(|err| format!("创建网易云请求客户端失败：{err}"))?;
    let mut request = client
        .post(url)
        .header("Referer", NETEASE_API_BASE)
        .header("Origin", NETEASE_API_BASE)
        .header("Accept", "application/json, text/plain, */*")
        .form(form);

    if let Some(cookie_header) = cookie_header.filter(|value| !value.trim().is_empty()) {
        request = request.header(COOKIE, cookie_header);
    }

    let response = request
        .send()
        .map_err(|err| format!("请求网易云音乐失败：{err}"))?;
    let status = response.status();
    let set_cookies = collect_set_cookies(response.headers());
    let body: Value = response
        .json()
        .map_err(|err| format!("解析网易云响应失败：{err}"))?;

    if !status.is_success() {
        return Err(format!("网易云接口返回 HTTP {status}"));
    }

    Ok(NeteaseResponse { body, set_cookies })
}

fn post_form_values(
    url: &str,
    form: &[(&str, String)],
    cookie_header: Option<&str>,
) -> Result<NeteaseResponse, String> {
    let borrowed = form
        .iter()
        .map(|(key, value)| (*key, value.as_str()))
        .collect::<Vec<_>>();
    post_form(url, &borrowed, cookie_header)
}

fn ensure_eapi_cookie_header(
    cookies: &mut BTreeMap<String, String>,
    uri: &str,
) -> Map<String, Value> {
    if !cookies.contains_key("_ntes_nuid") {
        cookies.insert("_ntes_nuid".to_string(), pseudo_hex(32));
    }
    if !cookies.contains_key("_ntes_nnid") {
        let nuid = cookies
            .get("_ntes_nuid")
            .cloned()
            .unwrap_or_else(|| pseudo_hex(32));
        cookies.insert(
            "_ntes_nnid".to_string(),
            format!("{nuid},{}", current_epoch_millis()),
        );
    }
    cookies
        .entry("__remember_me".to_string())
        .or_insert_with(|| "true".to_string());
    cookies
        .entry("ntes_kaola_ad".to_string())
        .or_insert_with(|| "1".to_string());
    cookies
        .entry("WNMCID".to_string())
        .or_insert_with(|| format!("{}.{}.01.0", pseudo_letters(6), current_epoch_millis()));
    cookies
        .entry("WEVNSM".to_string())
        .or_insert_with(|| "1.0.0".to_string());
    cookies
        .entry("osver".to_string())
        .or_insert_with(|| "Microsoft-Windows-10-Professional-build-19045-64bit".to_string());
    cookies
        .entry("deviceId".to_string())
        .or_insert_with(|| pseudo_hex(52));
    cookies
        .entry("os".to_string())
        .or_insert_with(|| "pc".to_string());
    cookies
        .entry("channel".to_string())
        .or_insert_with(|| "netease".to_string());
    cookies
        .entry("appver".to_string())
        .or_insert_with(|| "3.1.17.204416".to_string());

    if !uri.contains("login") {
        cookies.insert("NMTID".to_string(), pseudo_hex(16));
    }

    let mut header = Map::new();
    for key in [
        "osver", "deviceId", "os", "appver", "channel", "MUSIC_U", "MUSIC_A",
    ] {
        if let Some(value) = cookies.get(key).filter(|value| !value.trim().is_empty()) {
            header.insert(key.to_string(), Value::String(value.clone()));
        }
    }
    header.insert("versioncode".to_string(), Value::String("140".to_string()));
    header.insert("mobilename".to_string(), Value::String(String::new()));
    header.insert(
        "buildver".to_string(),
        Value::String(current_epoch_seconds().to_string()),
    );
    header.insert(
        "resolution".to_string(),
        Value::String("1920x1080".to_string()),
    );
    header.insert(
        "__csrf".to_string(),
        Value::String(cookies.get("__csrf").cloned().unwrap_or_default()),
    );
    header.insert(
        "requestId".to_string(),
        Value::String(format!(
            "{}_{}",
            current_epoch_millis(),
            current_epoch_millis() % 1000
        )),
    );

    header
}

fn eapi_encrypt(uri: &str, text: &str) -> Result<String, String> {
    let message = format!("nobody{uri}use{text}md5forencrypt");
    let mut hasher = Md5::new();
    hasher.update(message.as_bytes());
    let digest = format!("{:x}", hasher.finalize());
    let data = format!("{uri}-36cd479b6b5-{text}-36cd479b6b5-{digest}");
    let encrypted = Encryptor::<Aes128>::new_from_slice(NETEASE_EAPI_KEY.as_bytes())
        .map_err(|err| format!("网易云 eapi 密钥无效：{err}"))?
        .encrypt_padded_vec_mut::<Pkcs7>(data.as_bytes());

    Ok(bytes_to_upper_hex(&encrypted))
}

fn map_to_cookie_header(values: &Map<String, Value>) -> String {
    values
        .iter()
        .filter_map(|(key, value)| value.as_str().map(|value| (key, value)))
        .filter(|(key, value)| !key.trim().is_empty() && !value.trim().is_empty())
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>()
        .join("; ")
}

fn collect_set_cookies(headers: &HeaderMap) -> Vec<String> {
    headers
        .get_all(SET_COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .map(str::to_string)
        .collect()
}

fn merge_set_cookies(cookies: &mut BTreeMap<String, String>, set_cookies: &[String]) {
    for header in set_cookies {
        if let Some((name, value)) = parse_set_cookie(header) {
            cookies.insert(name, value);
        }
    }
}

fn parse_set_cookie(header: &str) -> Option<(String, String)> {
    let first = header.split(';').next()?.trim();
    let (name, value) = first.split_once('=')?;
    let name = name.trim();
    if name.is_empty() {
        return None;
    }

    Some((name.to_string(), value.trim().to_string()))
}

fn merge_cookies(target: &mut BTreeMap<String, String>, source: &BTreeMap<String, String>) {
    for (name, value) in source {
        if !name.trim().is_empty() && !value.trim().is_empty() {
            target.insert(name.clone(), value.clone());
        }
    }
}

fn cookie_header(cookies: &BTreeMap<String, String>) -> String {
    cookies
        .iter()
        .filter(|(name, value)| !name.trim().is_empty() && !value.trim().is_empty())
        .map(|(name, value)| format!("{name}={value}"))
        .collect::<Vec<_>>()
        .join("; ")
}

fn extract_membership(body: &Value, profile: &Value) -> NeteaseMembershipInfo {
    let vip_type = find_u64_by_keys(
        profile,
        &[
            "vipType",
            "viptype",
            "vip_type",
            "vipCode",
            "redVipType",
            "redVipCode",
        ],
    )
    .or_else(|| {
        find_u64_by_keys(
            body,
            &[
                "vipType",
                "viptype",
                "vip_type",
                "vipCode",
                "redVipType",
                "redVipCode",
            ],
        )
    });
    let level = find_u64_by_keys(profile, &["redVipLevel", "vipLevel", "vip_level"])
    .or_else(|| {
        find_u64_by_keys(body, &["redVipLevel", "vipLevel", "vip_level"])
    })
    .filter(|value| *value > 0);
    let expire_at = find_u64_by_keys(
        profile,
        &[
            "expireTime",
            "expire_time",
            "vipExpireTime",
            "vipExpire",
            "redVipExpireTime",
        ],
    )
    .or_else(|| {
        find_u64_by_keys(
            body,
            &[
                "expireTime",
                "expire_time",
                "vipExpireTime",
                "vipExpire",
                "redVipExpireTime",
            ],
        )
    })
    .and_then(normalize_epoch_seconds);
    let type_name = find_string_by_keys(
        profile,
        &["vipName", "vipTypeName", "vip_type_name", "memberName", "redVipName"],
        40,
    )
    .or_else(|| {
        find_string_by_keys(
            body,
            &["vipName", "vipTypeName", "vip_type_name", "memberName", "redVipName"],
            40,
        )
    });
    let rights_present = find_value_by_keys(profile, &["vipRights", "redVipRights"])
        .or_else(|| find_value_by_keys(body, &["vipRights", "redVipRights"]))
        .is_some();
    let has_membership_field =
        vip_type.is_some() || level.is_some() || expire_at.is_some() || type_name.is_some();
    let active = vip_type.unwrap_or_default() > 0 || level.is_some() || rights_present;

    NeteaseMembershipInfo {
        active,
        status_label: if active {
            "已检测到会员".to_string()
        } else if has_membership_field {
            "普通账号".to_string()
        } else {
            "未检测到会员信息".to_string()
        },
        type_label: type_name.or_else(|| {
            vip_type
                .filter(|value| *value > 0)
                .map(|value| format!("VIP 类型 {value}"))
        }),
        level_label: level.map(|value| format!("等级 {value}")),
        expire_at,
    }
}

fn extract_profile(body: &Value) -> Option<NeteaseLoginProfile> {
    let profile = body
        .get("profile")
        .or_else(|| body.pointer("/data/profile"))?;
    let user_id = profile
        .get("userId")
        .or_else(|| profile.get("user_id"))
        .and_then(Value::as_u64)?;
    let nickname = profile
        .get("nickname")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())?
        .chars()
        .take(80)
        .collect::<String>();
    let avatar_url = profile
        .get("avatarUrl")
        .or_else(|| profile.get("avatar_url"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| value.starts_with("http://") || value.starts_with("https://"))
        .map(|value| value.chars().take(500).collect());

    Some(NeteaseLoginProfile {
        user_id,
        nickname,
        avatar_url,
        membership: Some(extract_membership(body, profile)),
    })
}

fn credential_file(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app_data::data_dir(app)?
        .join(CREDENTIAL_DIR_NAME)
        .join(CREDENTIAL_FILE_NAME))
}

fn pending_file(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app_data::data_dir(app)?
        .join(CREDENTIAL_DIR_NAME)
        .join(PENDING_FILE_NAME))
}

fn read_credential(app: &AppHandle) -> Result<Option<NeteaseCredential>, String> {
    let path = credential_file(app)?;
    if !path.is_file() {
        return Ok(None);
    }

    let content =
        fs::read_to_string(&path).map_err(|err| format!("读取网易云登录状态失败：{err}"))?;
    if content.trim().is_empty() {
        return Ok(None);
    }

    serde_json::from_str(&content)
        .map(Some)
        .map_err(|err| format!("网易云登录状态文件格式错误：{err}"))
}

fn write_credential(app: &AppHandle, credential: &NeteaseCredential) -> Result<(), String> {
    let path = credential_file(app)?;
    let content = serde_json::to_vec_pretty(credential).map_err(|err| err.to_string())?;
    app_data::atomic_write_file(&path, &content)
        .map_err(|err| format!("写入网易云登录状态失败：{err}"))
}

fn read_pending_login(app: &AppHandle) -> Result<Option<NeteasePendingLogin>, String> {
    let path = pending_file(app)?;
    if !path.is_file() {
        return Ok(None);
    }

    let content =
        fs::read_to_string(&path).map_err(|err| format!("读取网易云临时登录状态失败：{err}"))?;
    if content.trim().is_empty() {
        return Ok(None);
    }

    serde_json::from_str(&content)
        .map(Some)
        .map_err(|err| format!("网易云临时登录状态文件格式错误：{err}"))
}

fn write_pending_login(app: &AppHandle, pending: &NeteasePendingLogin) -> Result<(), String> {
    let path = pending_file(app)?;
    let content = serde_json::to_vec_pretty(pending).map_err(|err| err.to_string())?;
    app_data::atomic_write_file(&path, &content)
        .map_err(|err| format!("写入网易云临时登录状态失败：{err}"))
}

fn clear_pending_login(app: &AppHandle) -> Result<(), String> {
    let path = pending_file(app)?;
    match fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(format!("清除网易云临时登录状态失败：{err}")),
    }
}

fn qr_status_name(code: i64) -> &'static str {
    match code {
        800 => "expired",
        801 => "waiting",
        802 => "scanned",
        803 => "authorized",
        _ => "unknown",
    }
}

fn qr_status_message(code: i64) -> &'static str {
    match code {
        800 => "二维码已过期，请重新生成。",
        801 => "等待网易云 App 扫码。",
        802 => "已扫码，请在手机上确认登录。",
        803 => "网易云登录成功。",
        _ => "网易云二维码状态未知。",
    }
}

fn timestamp_url(url: &str) -> String {
    format!("{url}?timestamp={}", current_epoch_millis())
}

fn generate_web_login_chain_id(cookies: &BTreeMap<String, String>) -> String {
    let device_id = cookies
        .get("sDeviceId")
        .or_else(|| cookies.get("deviceId"))
        .filter(|value| !value.trim().is_empty())
        .cloned()
        .unwrap_or_else(|| format!("unknown-{}", current_epoch_millis() % 1_000_000));
    format!("v1_{device_id}_web_login_{}", current_epoch_millis())
}

fn pseudo_hex(len: usize) -> String {
    const HEX: &[u8] = b"0123456789abcdef";
    let mut seed = current_epoch_millis() as u64 ^ ((std::process::id() as u64) << 17) ^ len as u64;
    let mut output = String::with_capacity(len);
    for _ in 0..len {
        seed ^= seed << 7;
        seed ^= seed >> 9;
        seed ^= seed << 8;
        output.push(HEX[(seed as usize) & 0x0f] as char);
    }
    output
}

fn pseudo_letters(len: usize) -> String {
    const LETTERS: &[u8] = b"abcdefghijklmnopqrstuvwxyz";
    let mut seed = current_epoch_millis() as u64 ^ ((std::process::id() as u64) << 11) ^ len as u64;
    let mut output = String::with_capacity(len);
    for _ in 0..len {
        seed ^= seed << 7;
        seed ^= seed >> 9;
        seed ^= seed << 8;
        output.push(LETTERS[(seed as usize) % LETTERS.len()] as char);
    }
    output
}

fn bytes_to_upper_hex(bytes: &[u8]) -> String {
    const HEX: &[u8] = b"0123456789ABCDEF";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

fn current_epoch_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn current_epoch_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}
