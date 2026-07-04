use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    path::PathBuf,
    sync::{mpsc, Arc, Mutex, OnceLock},
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use aes::{Aes128, Aes256};
use base64::{engine::general_purpose, Engine as _};
use cbc::{Decryptor as CbcDecryptor, Encryptor as CbcEncryptor};
use cipher::{block_padding::Pkcs7, BlockDecryptMut, BlockEncryptMut, KeyIvInit};
use md5::{Digest, Md5};
use num_bigint::BigUint;
use qrcode::{render::svg, QrCode};
use reqwest::{
    blocking::Client,
    header::{
        HeaderMap, ACCEPT_ENCODING, ACCEPT_RANGES, CACHE_CONTROL, CONTENT_LENGTH, CONTENT_RANGE,
        CONTENT_TYPE, RANGE, REFERER, SET_COOKIE, USER_AGENT,
    },
    StatusCode,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tauri::AppHandle;

use crate::app_data;

const KUGOU_SEARCH_ENDPOINT: &str = "http://mobilecdn.kugou.com/api/v3/search/song";
const KUGOU_PLAY_INFO_ENDPOINT: &str = "http://m.kugou.com/app/i/getSongInfo.php";
const KUGOU_LYRIC_SEARCH_ENDPOINT: &str = "https://lyrics.kugou.com/search";
const KUGOU_LYRIC_DOWNLOAD_ENDPOINT: &str = "https://lyrics.kugou.com/download";
const KUGOU_LOGIN_BASE: &str = "https://login-user.kugou.com";
const KUGOU_LOGIN_TOKEN_BASE: &str = "http://login.user.kugou.com";
const KUGOU_GATEWAY_BASE: &str = "https://gateway.kugou.com";
const KUGOU_TRACKER_BASE: &str = "http://tracker.kugou.com";
const KUGOU_USER_SERVICE_BASE: &str = "https://userservice.kugou.com";
const KUGOU_QR_KEY_PATH: &str = "/v2/qrcode";
const KUGOU_QR_CHECK_PATH: &str = "/v2/get_userinfo_qrcode";
const KUGOU_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0 Safari/537.36";
const KUGOU_SIGNATURE_WEB_SECRET: &str = "NVPh5oo715z5DIWAeQlhMDsWXXQV4hwt";
const KUGOU_SIGNATURE_ANDROID_SECRET: &str = "LnT6xpN3khm36zse0QzvmgTZ3waWdRSA";
const KUGOU_APPID: &str = "3116";
const KUGOU_QR_APPID: &str = "1001";
const KUGOU_CLIENTVER: &str = "11440";
const KUGOU_SRC_APPID: &str = "2919";
const KUGOU_TRACKER_KEY_SECRET: &str = "185672dd44712f60bb1736df5a377e82";
const KUGOU_LOGIN_TOKEN_LITE_KEY: &str = "c24f74ca2820225badc01946dba4fdf7";
const KUGOU_LOGIN_TOKEN_LITE_IV: &str = "adc01946dba4fdf7";
const KUGOU_LOGIN_TOKEN_LITE_T2_KEY: &str = "fd14b35e3f81af3817a20ae7adae7020";
const KUGOU_LOGIN_TOKEN_LITE_T2_IV: &str = "17a20ae7adae7020";
const KUGOU_LOGIN_TOKEN_LITE_T1_KEY: &str = "5e4ef500e9597fe004bd09a46d8add98";
const KUGOU_LOGIN_TOKEN_LITE_T1_IV: &str = "04bd09a46d8add98";
const KUGOU_DEVICE_REGISTER_MARKER: &str = "KUGOU_API_DFID_REGISTERED_AT";
const KUGOU_DEVICE_REGISTER_TTL_MS: u64 = 24 * 60 * 60 * 1000;
const KUGOU_LITE_RSA_MODULUS_HEX: &str = "c40a2d0da76511f3bb1cc2bbd3afbd8bea83b4d6b05b6c13eb8920c53f1af7679b32ba0d0edb843240ef1b836efed3ee240734c14c1399fd6594d16af22f52525d14d72e0155c6dcc8638d4f7bb94f3a0b1f4c29f991972f2a160a25eb0a9e724336be7f69bbd319ffab1c6dd8470b021dc434f3faba89f4a2a01b33bdbdd08b";
const KUGOU_RSA_PUBLIC_EXPONENT_HEX: &str = "010001";
const KUGOU_QR_EXPIRE_SECONDS: u64 = 180;
const MAX_KUGOU_SEARCH_LIMIT: u64 = 50;
const MAX_KUGOU_PLAYLISTS: usize = 300;
const MAX_KUGOU_RECOMMENDED_PLAYLISTS: usize = 300;
const MAX_KUGOU_PLAYLIST_TRACK_PAGE_LIMIT: u64 = 300;
const MAX_KUGOU_LYRIC_CHARS: usize = 80_000;
const MAX_KUGOU_PROXY_SESSIONS: usize = 64;
const KUGOU_PROXY_SESSION_TTL_MS: u64 = 6 * 60 * 60 * 1000;
const KUGOU_PROXY_UPSTREAM_CONNECT_TIMEOUT_SECONDS: u64 = 12;
const KUGOU_PROXY_UPSTREAM_READ_TIMEOUT_SECONDS: u64 = 18;
const KUGOU_PROXY_PATH_PREFIX: &str = "/kugou/play/";
const CREDENTIAL_DIR_NAME: &str = "music-platforms";
const CREDENTIAL_FILE_NAME: &str = "kugou.json";
const PENDING_FILE_NAME: &str = "kugou-pending.json";

static KUGOU_PLAYBACK_PROXY: OnceLock<Mutex<Option<KugouPlaybackProxyState>>> = OnceLock::new();

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KugouMembershipInfo {
    pub active: bool,
    pub status_label: String,
    pub type_label: Option<String>,
    pub level_label: Option<String>,
    pub expire_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KugouLoginProfile {
    pub user_id: String,
    pub nickname: String,
    pub avatar_url: Option<String>,
    pub membership: Option<KugouMembershipInfo>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KugouLoginStatus {
    pub logged_in: bool,
    pub profile: Option<KugouLoginProfile>,
    pub saved_at: Option<String>,
    pub checked_at: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KugouQrLogin {
    pub key: String,
    pub qr_url: String,
    pub qr_image: String,
    pub expires_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KugouQrCheckResult {
    pub code: i64,
    pub status: String,
    pub message: String,
    pub logged_in: bool,
    pub profile: Option<KugouLoginProfile>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KugouSearchTrack {
    pub id: String,
    pub hash: String,
    pub hash_candidates: Vec<String>,
    pub name: String,
    pub artists: Vec<String>,
    pub album: Option<String>,
    pub duration_ms: Option<u64>,
    pub cover_img_url: Option<String>,
    pub album_id: Option<String>,
    pub album_audio_id: Option<u64>,
    pub audio_id: Option<u64>,
    pub privilege: Option<u64>,
    pub pay_type: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KugouSearchResult {
    pub keyword: String,
    pub tracks: Vec<KugouSearchTrack>,
    pub total: u64,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KugouPlaylistSummary {
    pub id: String,
    pub list_id: String,
    pub global_collection_id: Option<String>,
    pub name: String,
    pub track_count: u64,
    pub cover_img_url: Option<String>,
    pub creator_nickname: Option<String>,
    pub subscribed: bool,
    pub update_time: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KugouPlaylistDetail {
    pub playlist: KugouPlaylistSummary,
    pub tracks: Vec<KugouSearchTrack>,
    pub total_track_count: u64,
    pub truncated: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KugouRecommendedPlaylists {
    pub playlists: Vec<KugouPlaylistSummary>,
    pub total: u64,
    pub page: u64,
    pub truncated: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KugouLyricsResult {
    pub song_id: String,
    pub content: String,
    pub lrc_content: Option<String>,
    pub source: String,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KugouPlaybackUrl {
    pub hash: String,
    pub url: String,
    pub quality_level: Option<String>,
    pub quality_label: Option<String>,
    pub bitrate: Option<u64>,
    pub duration_ms: Option<u64>,
    pub file_type: Option<String>,
    pub size: Option<u64>,
    pub message: String,
    pub proxy_diagnostic: Option<String>,
    pub proxy_likely_preview: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KugouPlaybackProxyStatus {
    pub ok: bool,
    pub message: String,
    pub probe_message: Option<String>,
    pub likely_preview: bool,
    pub last_error: Option<String>,
    pub last_range: Option<String>,
    pub refresh_count: u32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KugouQualityAvailability {
    pub hash: String,
    pub qualities: Vec<KugouQualityAvailabilityItem>,
    pub message: String,
    pub diagnostic: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KugouQualityAvailabilityItem {
    pub quality: String,
    pub label: String,
    pub status: String,
    pub reason: Option<String>,
    pub detail: Option<String>,
}

#[derive(Clone)]
struct KugouPlaybackSession {
    hashes: Vec<String>,
    album_audio_id: Option<u64>,
    audio_id: Option<u64>,
    quality_preference: String,
    playback: KugouPlaybackUrl,
    probe: KugouPlaybackProbe,
    created_at_ms: u64,
    refreshed_at_ms: u64,
    last_error: Option<String>,
    last_error_at_ms: Option<u64>,
    last_range: Option<String>,
    refresh_count: u32,
}

#[derive(Clone)]
struct KugouPlaybackProxyState {
    port: u16,
    sessions: Arc<Mutex<BTreeMap<String, KugouPlaybackSession>>>,
}

struct KugouProxyRequest {
    method: String,
    path: String,
    headers: BTreeMap<String, String>,
}

#[derive(Clone)]
struct KugouPlaybackProbe {
    message: String,
    likely_preview: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct KugouCredential {
    cookies: BTreeMap<String, String>,
    saved_at: String,
    profile: Option<KugouLoginProfile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct KugouPendingLogin {
    key: String,
    cookies: BTreeMap<String, String>,
    created_at: String,
    expires_at: String,
}

pub fn create_qr_login(app: &AppHandle) -> Result<KugouQrLogin, String> {
    let mut cookies = init_device_cookies();
    let mut params = BTreeMap::new();
    params.insert("appid".to_string(), KUGOU_QR_APPID.to_string());
    params.insert("type".to_string(), "1".to_string());
    params.insert("plat".to_string(), "4".to_string());
    params.insert(
        "qrcode_txt".to_string(),
        format!("https://h5.kugou.com/apps/loginQRCode/html/index.html?appid={KUGOU_APPID}&"),
    );
    params.insert("srcappid".to_string(), KUGOU_SRC_APPID.to_string());
    let body = kugou_signed_get_json(KUGOU_LOGIN_BASE, KUGOU_QR_KEY_PATH, params, &mut cookies)?;
    let data = body
        .get("data")
        .ok_or_else(|| "酷狗二维码响应缺少 data 字段。".to_string())?;
    let key = value_as_string(data, "qrcode")
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "酷狗二维码 key 返回为空。".to_string())?;
    let expires_at = (current_epoch_seconds() + KUGOU_QR_EXPIRE_SECONDS).to_string();
    write_pending_login(
        app,
        &KugouPendingLogin {
            key: key.clone(),
            cookies,
            created_at: app_data::now_seconds(),
            expires_at: expires_at.clone(),
        },
    )?;

    let qr_url = format!("https://h5.kugou.com/apps/loginQRCode/html/index.html?qrcode={key}");
    let qr_image = value_as_string(data, "qrcode_img")
        .filter(|value| value.starts_with("data:image/"))
        .unwrap_or_else(|| create_qr_image(&qr_url).unwrap_or_default());
    if qr_image.is_empty() {
        return Err("酷狗二维码图片生成失败。".to_string());
    }

    Ok(KugouQrLogin {
        key,
        qr_url,
        qr_image,
        expires_at,
    })
}

pub fn check_qr_login(app: &AppHandle, key: String) -> Result<KugouQrCheckResult, String> {
    let key = key.trim();
    if key.is_empty() || key.len() > 128 {
        return Err("酷狗二维码 key 无效。".to_string());
    }

    let mut pending = read_pending_login(app)?
        .filter(|pending| pending.key == key)
        .unwrap_or_else(|| KugouPendingLogin {
            key: key.to_string(),
            cookies: init_device_cookies(),
            created_at: app_data::now_seconds(),
            expires_at: (current_epoch_seconds() + KUGOU_QR_EXPIRE_SECONDS).to_string(),
        });
    if pending
        .expires_at
        .parse::<u64>()
        .map(|expires_at| current_epoch_seconds() >= expires_at)
        .unwrap_or(false)
    {
        clear_pending_login(app)?;
        return Ok(KugouQrCheckResult {
            code: 0,
            status: "expired".to_string(),
            message: "酷狗二维码已过期，请重新生成。".to_string(),
            logged_in: false,
            profile: None,
        });
    }

    let mut params = BTreeMap::new();
    params.insert("plat".to_string(), "4".to_string());
    params.insert("appid".to_string(), KUGOU_APPID.to_string());
    params.insert("srcappid".to_string(), KUGOU_SRC_APPID.to_string());
    params.insert("qrcode".to_string(), key.to_string());
    let body = kugou_signed_get_json(
        KUGOU_LOGIN_BASE,
        KUGOU_QR_CHECK_PATH,
        params,
        &mut pending.cookies,
    )?;
    write_pending_login(app, &pending)?;

    let data = body
        .get("data")
        .ok_or_else(|| "酷狗扫码状态响应缺少 data 字段。".to_string())?;
    let code = value_as_i64(data, "status").unwrap_or_default();
    let message = value_as_string(&body, "error")
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| kugou_qr_status_message(code).to_string());

    if code == 4 {
        if let Some(token) = value_as_string(data, "token").filter(|value| !value.is_empty()) {
            pending.cookies.insert("token".to_string(), token);
        }
        if let Some(user_id) = value_as_string(data, "userid").filter(|value| !value.is_empty()) {
            pending.cookies.insert("userid".to_string(), user_id);
        }
        for (cookie_key, response_keys) in [
            ("vip_token", ["vip_token", "vipToken", "viptoken"]),
            ("vip_type", ["vip_type", "vipType", "viptype"]),
            ("roam_type", ["roam_type", "roamType", "roamtype"]),
        ] {
            if let Some(value) =
                first_string(data, &response_keys).filter(|value| !value.is_empty())
            {
                pending.cookies.insert(cookie_key.to_string(), value);
            }
        }
        let Some(user_id) = pending
            .cookies
            .get("userid")
            .cloned()
            .filter(|value| !value.is_empty())
        else {
            return Ok(KugouQrCheckResult {
                code,
                status: "authorized_without_user".to_string(),
                message: "酷狗已确认登录，但没有返回可保存的用户 ID。".to_string(),
                logged_in: false,
                profile: None,
            });
        };
        let Some(_) = pending
            .cookies
            .get("token")
            .filter(|value| !value.is_empty())
        else {
            return Ok(KugouQrCheckResult {
                code,
                status: "authorized_without_token".to_string(),
                message: "酷狗已确认登录，但没有返回可保存的登录 token。".to_string(),
                logged_in: false,
                profile: None,
            });
        };
        let mut profile = extract_login_profile(data, &user_id);
        let mut credential = KugouCredential {
            cookies: pending.cookies,
            saved_at: app_data::now_seconds(),
            profile: Some(profile.clone()),
        };
        let _ = refresh_kugou_login_token(&mut credential);
        profile.membership = Some(kugou_membership_from_cookies(
            &credential.cookies,
            profile.membership.as_ref(),
        ));
        credential.profile = Some(profile.clone());
        write_credential(app, &credential)?;
        clear_pending_login(app)?;

        return Ok(KugouQrCheckResult {
            code,
            status: "authorized".to_string(),
            message: "酷狗音乐已登录。".to_string(),
            logged_in: true,
            profile: Some(profile),
        });
    }

    Ok(KugouQrCheckResult {
        code,
        status: kugou_qr_status_name(code).to_string(),
        message,
        logged_in: false,
        profile: None,
    })
}

pub fn login_status(app: &AppHandle) -> Result<KugouLoginStatus, String> {
    let Some(credential) = read_credential(app)? else {
        return Ok(KugouLoginStatus {
            logged_in: false,
            profile: None,
            saved_at: None,
            checked_at: app_data::now_seconds(),
            message: "尚未登录酷狗音乐。".to_string(),
        });
    };

    let logged_in = credential
        .cookies
        .get("token")
        .filter(|value| !value.trim().is_empty())
        .is_some()
        && credential
            .cookies
            .get("userid")
            .filter(|value| !value.trim().is_empty())
            .is_some();
    let mut profile = credential.profile.clone();
    if let Some(profile) = profile.as_mut() {
        profile.membership = Some(kugou_membership_from_cookies(
            &credential.cookies,
            profile.membership.as_ref(),
        ));
    }
    Ok(KugouLoginStatus {
        logged_in,
        profile,
        saved_at: Some(credential.saved_at),
        checked_at: app_data::now_seconds(),
        message: if logged_in
            && first_cookie_value(
                &credential.cookies,
                &["vip_token", "vipToken", "viptoken", "kg_vip_token"],
            )
            .is_none()
        {
            "酷狗登录凭据已保存在本机，但缺少会员播放 token；如需播放会员歌曲，请清除登录后重新扫码。".to_string()
        } else if logged_in {
            "酷狗登录凭据已保存在本机。".to_string()
        } else {
            "酷狗登录状态不可用，请重新扫码。".to_string()
        },
    })
}

pub fn clear_login(app: &AppHandle) -> Result<KugouLoginStatus, String> {
    let path = credential_file(app)?;
    match fs::remove_file(&path) {
        Ok(()) => {}
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
        Err(err) => return Err(format!("清除酷狗登录状态失败：{err}")),
    }
    clear_pending_login(app)?;

    Ok(KugouLoginStatus {
        logged_in: false,
        profile: None,
        saved_at: None,
        checked_at: app_data::now_seconds(),
        message: "已清除本机酷狗登录状态。".to_string(),
    })
}

pub fn list_playlists(app: &AppHandle) -> Result<Vec<KugouPlaylistSummary>, String> {
    let mut credential = require_logged_in_credential(app)?;
    let user_id = credential
        .cookies
        .get("userid")
        .cloned()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| "酷狗登录状态缺少用户 ID，请重新扫码登录。".to_string())?;
    let token = credential
        .cookies
        .get("token")
        .cloned()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| "酷狗登录状态缺少 token，请重新扫码登录。".to_string())?;
    let body = json!({
        "userid": user_id,
        "token": token,
        "total_ver": 979,
        "type": 2,
        "page": 1,
        "pagesize": MAX_KUGOU_PLAYLISTS,
    });
    let mut params = BTreeMap::new();
    params.insert("plat".to_string(), "1".to_string());
    params.insert("userid".to_string(), user_id);
    params.insert("token".to_string(), token);
    let response = kugou_android_post_json(
        KUGOU_GATEWAY_BASE,
        "/v7/get_all_list",
        params,
        body,
        &mut credential.cookies,
        Some("cloudlist.service.kugou.com"),
    )?;
    write_credential(app, &credential)?;
    ensure_kugou_success(&response, "酷狗歌单读取")?;

    let items = first_array_at_paths(
        &response,
        &[
            &["data", "info"],
            &["data", "list"],
            &["data", "lists"],
            &["data", "playlist"],
            &["data"],
            &["info"],
            &["list"],
        ],
    )
    .ok_or_else(|| "酷狗歌单响应缺少列表字段。".to_string())?;
    let mut playlists = items
        .iter()
        .filter_map(parse_playlist_summary)
        .take(MAX_KUGOU_PLAYLISTS)
        .collect::<Vec<_>>();
    playlists.reverse();

    Ok(playlists)
}

fn normalize_playlist_detail_paging(page: Option<u64>, limit: Option<u64>) -> (u64, u64) {
    (
        page.unwrap_or(1).clamp(1, 200),
        limit
            .unwrap_or(MAX_KUGOU_PLAYLIST_TRACK_PAGE_LIMIT)
            .clamp(1, MAX_KUGOU_PLAYLIST_TRACK_PAGE_LIMIT),
    )
}

pub fn playlist_detail(
    app: &AppHandle,
    list_id: String,
    page: Option<u64>,
    limit: Option<u64>,
) -> Result<KugouPlaylistDetail, String> {
    let list_id = normalize_list_id(&list_id)?;
    let (page, limit) = normalize_playlist_detail_paging(page, limit);
    let mut credential = require_logged_in_credential(app)?;
    let user_id = credential
        .cookies
        .get("userid")
        .cloned()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| "酷狗登录状态缺少用户 ID，请重新扫码登录。".to_string())?;
    let token = credential
        .cookies
        .get("token")
        .cloned()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| "酷狗登录状态缺少 token，请重新扫码登录。".to_string())?;
    let response = request_kugou_playlist_detail_page(
        &list_id,
        &user_id,
        &token,
        1,
        limit,
        &mut credential.cookies,
    )?;
    ensure_kugou_success(&response, "酷狗歌单歌曲读取")?;
    let track_paths: &[&[&str]] = &[
        &["data", "info"],
        &["data", "song_list"],
        &["data", "songlist"],
        &["data", "music_list"],
        &["data", "musiclist"],
        &["data", "songs"],
        &["data", "files"],
        &["data", "items"],
        &["data", "list"],
        &["data", "lists"],
        &["data", "snap"],
        &["data", "snap", "info"],
        &["data", "snap", "list"],
        &["data", "snap", "songs"],
        &["data", "snap", "files"],
        &["data", "snap", "items"],
        &["data", "snap", "data"],
        &["data", "playlist", "info"],
        &["data", "playlist", "songs"],
        &["data", "playlist", "list"],
        &["data", "list_info", "info"],
        &["data", "list_info", "songs"],
        &["data", "list_info", "list"],
        &["data", "content", "info"],
        &["data", "content", "songs"],
        &["data", "content", "list"],
        &["data", "info", "info"],
        &["data", "info", "list"],
        &["data", "info", "lists"],
        &["data", "info", "songs"],
        &["data", "info", "files"],
        &["data", "info", "items"],
        &["data", "info", "data"],
        &["data", "data"],
        &["info"],
        &["song_list"],
        &["songlist"],
        &["songs"],
    ];
    let playlist =
        parse_playlist_summary_from_response(&response).unwrap_or_else(|| KugouPlaylistSummary {
            id: list_id.clone(),
            list_id: list_id.clone(),
            global_collection_id: None,
            name: "酷狗歌单".to_string(),
            track_count: 0,
            cover_img_url: None,
            creator_nickname: credential
                .profile
                .as_ref()
                .map(|profile| profile.nickname.clone()),
            subscribed: false,
            update_time: None,
        });
    let total_track_count = kugou_playlist_total_track_count(&response, &playlist);
    let limit_usize = usize::try_from(limit).unwrap_or(usize::MAX);
    let (tracks, total_track_count, truncated) = if total_track_count > 0 {
        if let Some((original_start, original_end)) =
            reverse_playlist_page_bounds(total_track_count, page, limit)
        {
            let first_upstream_page = original_start / limit + 1;
            let last_upstream_page = (original_end.saturating_sub(1)) / limit + 1;
            let mut selected_items = Vec::new();

            for upstream_page in first_upstream_page..=last_upstream_page {
                let page_response = if upstream_page == 1 {
                    response.clone()
                } else {
                    let page_response = request_kugou_playlist_detail_page(
                        &list_id,
                        &user_id,
                        &token,
                        upstream_page,
                        limit,
                        &mut credential.cookies,
                    )?;
                    ensure_kugou_success(&page_response, "酷狗歌单歌曲读取")?;
                    page_response
                };
                let Some(items) =
                    kugou_playlist_track_items_from_response(&page_response, track_paths)
                else {
                    return Err(format!(
                        "酷狗歌单歌曲响应缺少歌曲列表字段（{}）。",
                        describe_response_shape(&page_response)
                    ));
                };
                let upstream_start = upstream_page.saturating_sub(1).saturating_mul(limit);
                for (index, item) in items.into_iter().enumerate() {
                    let original_index = upstream_start.saturating_add(index as u64);
                    if original_index >= original_start && original_index < original_end {
                        selected_items.push(item);
                    }
                }
            }

            let tracks = parse_playlist_track_items_reversed(&selected_items, limit_usize);
            let loaded_end = page
                .saturating_sub(1)
                .saturating_mul(limit)
                .saturating_add(selected_items.len().max(tracks.len()) as u64);
            (tracks, total_track_count, loaded_end < total_track_count)
        } else {
            (Vec::new(), total_track_count, false)
        }
    } else {
        let page_response = if page == 1 {
            response.clone()
        } else {
            let page_response = request_kugou_playlist_detail_page(
                &list_id,
                &user_id,
                &token,
                page,
                limit,
                &mut credential.cookies,
            )?;
            ensure_kugou_success(&page_response, "酷狗歌单歌曲读取")?;
            page_response
        };
        let Some(items) = kugou_playlist_track_items_from_response(&page_response, track_paths)
        else {
            return Err(format!(
                "酷狗歌单歌曲响应缺少歌曲列表字段（{}）。",
                describe_response_shape(&page_response)
            ));
        };
        let tracks = parse_playlist_track_items_reversed(&items, limit_usize);
        let total_track_count = page
            .saturating_sub(1)
            .saturating_mul(limit)
            .saturating_add(items.len().max(tracks.len()) as u64);
        let truncated = items.len() > tracks.len() || items.len() as u64 >= limit;
        (tracks, total_track_count, truncated)
    };
    write_credential(app, &credential)?;
    let message = if truncated {
        format!(
            "已读取第 {} 页 {} 首酷狗歌单歌曲，共 {} 首，可继续加载。",
            page,
            tracks.len(),
            total_track_count
        )
    } else {
        format!(
            "已读取第 {} 页 {} 首酷狗歌单歌曲，共 {} 首。",
            page,
            tracks.len(),
            total_track_count
        )
    };

    Ok(KugouPlaylistDetail {
        playlist,
        tracks,
        total_track_count,
        truncated,
        message,
    })
}

pub fn recommended_playlists(
    app: &AppHandle,
    page: Option<u64>,
    limit: Option<u64>,
) -> Result<KugouRecommendedPlaylists, String> {
    let page = page.unwrap_or(1).clamp(1, 50);
    let limit = limit.unwrap_or(30).clamp(1, 60);
    let mut credential = read_credential(app)?;
    let mut cookies = credential
        .as_ref()
        .map(|credential| credential.cookies.clone())
        .unwrap_or_else(init_device_cookies);
    let user_id = cookies
        .get("userid")
        .cloned()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "0".to_string());
    let date_time = current_epoch_seconds().to_string();
    let body = json!({
        "appid": KUGOU_APPID,
        "mid": cookies.get("KUGOU_API_MID").cloned().unwrap_or_else(|| "-".to_string()),
        "clientver": KUGOU_CLIENTVER,
        "platform": "android",
        "clienttime": date_time,
        "userid": user_id,
        "module_id": 1,
        "page": page,
        "pagesize": limit,
        "key": sign_params_key(&date_time),
        "special_recommend": {
            "withtag": 1,
            "withsong": 1,
            "sort": 1,
            "ugc": 1,
            "is_selected": 0,
            "withrecommend": 1,
            "area_code": 1,
            "categoryid": 0,
        },
        "req_multi": 1,
        "retrun_min": 5,
        "return_special_falg": 1,
    });
    let response = kugou_android_post_json(
        KUGOU_GATEWAY_BASE,
        "/v2/special_recommend",
        BTreeMap::new(),
        body,
        &mut cookies,
        Some("specialrec.service.kugou.com"),
    )?;
    if let Some(credential) = credential.as_mut() {
        credential.cookies = cookies;
        write_credential(app, credential)?;
    }
    ensure_kugou_success(&response, "酷狗推荐歌单读取")?;

    let items = first_playlist_array_at_paths(
        &response,
        &[
            &["data", "special_recommend"],
            &["data", "special_recommend", "info"],
            &["data", "special_recommend", "list"],
            &["data", "info"],
            &["data", "list"],
            &["data", "lists"],
            &["data", "playlist"],
            &["special_recommend"],
            &["info"],
            &["list"],
        ],
    )
    .or_else(|| find_playlist_array(&response))
    .ok_or_else(|| {
        format!(
            "酷狗推荐歌单响应缺少列表字段（{}）。",
            describe_response_shape(&response)
        )
    })?;
    let playlists = items
        .iter()
        .filter_map(parse_playlist_summary)
        .take(MAX_KUGOU_RECOMMENDED_PLAYLISTS)
        .collect::<Vec<_>>();
    let response_data = response.get("data").unwrap_or(&Value::Null);
    let reported_total = value_as_u64(response_data, "total")
        .or_else(|| value_as_u64(response_data, "count"))
        .or_else(|| value_as_u64(response_data, "total_count"));
    let loaded_end = page
        .saturating_sub(1)
        .saturating_mul(limit)
        .saturating_add(playlists.len() as u64);
    let total = reported_total.unwrap_or(loaded_end);
    let truncated = reported_total
        .map(|total| loaded_end < total)
        .unwrap_or_else(|| playlists.len() as u64 >= limit);
    let message = if playlists.is_empty() {
        "没有读取到酷狗推荐歌单。".to_string()
    } else if truncated {
        format!(
            "已读取第 {page} 页 {} 个酷狗推荐歌单，可继续加载。",
            playlists.len()
        )
    } else {
        format!("已读取第 {page} 页 {} 个酷狗推荐歌单。", playlists.len())
    };

    Ok(KugouRecommendedPlaylists {
        playlists,
        total,
        page,
        truncated,
        message,
    })
}

pub fn recommended_playlist_detail(
    app: &AppHandle,
    playlist_id: String,
    page: Option<u64>,
    limit: Option<u64>,
) -> Result<KugouPlaylistDetail, String> {
    let playlist_id = normalize_list_id(&playlist_id)?;
    let (page, limit) = normalize_playlist_detail_paging(page, limit);
    let mut credential = read_credential(app)?;
    let mut cookies = credential
        .as_ref()
        .map(|credential| credential.cookies.clone())
        .unwrap_or_else(init_device_cookies);
    let playlist =
        request_public_playlist_summary(&playlist_id, &mut cookies).unwrap_or_else(|_| {
            KugouPlaylistSummary {
                id: playlist_id.clone(),
                list_id: playlist_id.clone(),
                global_collection_id: Some(playlist_id.clone()),
                name: "酷狗推荐歌单".to_string(),
                track_count: 0,
                cover_img_url: None,
                creator_nickname: None,
                subscribed: false,
                update_time: None,
            }
        });
    let response = request_public_playlist_track_page(&playlist_id, page, limit, &mut cookies)?;
    if let Some(credential) = credential.as_mut() {
        credential.cookies = cookies;
        write_credential(app, credential)?;
    }
    ensure_kugou_success(&response, "酷狗推荐歌单歌曲读取")?;
    let track_paths: &[&[&str]] = &[
        &["data", "info"],
        &["data", "song_list"],
        &["data", "songlist"],
        &["data", "songs"],
        &["data", "files"],
        &["data", "list"],
        &["data", "lists"],
        &["data", "items"],
        &["data", "data"],
        &["info"],
        &["song_list"],
        &["songlist"],
        &["songs"],
    ];
    let Some(items) = kugou_playlist_track_items_from_response(&response, track_paths) else {
        return Err(format!(
            "酷狗推荐歌单歌曲响应缺少歌曲列表字段（{}）。",
            describe_response_shape(&response)
        ));
    };
    let limit_usize = usize::try_from(limit).unwrap_or(usize::MAX);
    let tracks = items
        .iter()
        .filter_map(parse_kugou_track)
        .take(limit_usize)
        .collect::<Vec<_>>();
    let response_data = response.get("data").unwrap_or(&Value::Null);
    let total_track_count = value_as_u64(response_data, "total")
        .or_else(|| value_as_u64(response_data, "count"))
        .or_else(|| value_as_u64(response_data, "total_count"))
        .or_else(|| value_as_u64(response_data, "song_count"))
        .or_else(|| (playlist.track_count > 0).then_some(playlist.track_count))
        .unwrap_or_else(|| page.saturating_sub(1).saturating_mul(limit) + tracks.len() as u64);
    let loaded_end = page
        .saturating_sub(1)
        .saturating_mul(limit)
        .saturating_add(items.len().max(tracks.len()) as u64);
    let truncated = loaded_end < total_track_count || items.len() > tracks.len();
    let message = if truncated {
        format!(
            "已读取第 {} 页 {} 首酷狗推荐歌单歌曲，共 {} 首，可继续加载。",
            page,
            tracks.len(),
            total_track_count
        )
    } else {
        format!(
            "已读取第 {} 页 {} 首酷狗推荐歌单歌曲，共 {} 首。",
            page,
            tracks.len(),
            total_track_count
        )
    };

    Ok(KugouPlaylistDetail {
        playlist,
        tracks,
        total_track_count,
        truncated,
        message,
    })
}

pub fn daily_recommended_songs(app: &AppHandle) -> Result<KugouSearchResult, String> {
    let mut credential = read_credential(app)?;
    let mut cookies = credential
        .as_ref()
        .map(|credential| credential.cookies.clone())
        .unwrap_or_else(init_device_cookies);
    let user_id = cookies
        .get("userid")
        .cloned()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "0".to_string());
    let response = kugou_android_post_json(
        KUGOU_GATEWAY_BASE,
        "/everyday_song_recommend",
        BTreeMap::new(),
        json!({
            "platform": "android",
            "userid": user_id,
        }),
        &mut cookies,
        Some("everydayrec.service.kugou.com"),
    )?;
    if let Some(credential) = credential.as_mut() {
        credential.cookies = cookies;
        write_credential(app, credential)?;
    }
    ensure_kugou_success(&response, "酷狗每日推荐读取")?;

    let track_paths: &[&[&str]] = &[
        &["data", "song_list"],
        &["data", "songlist"],
        &["data", "songs"],
        &["data", "list"],
        &["data", "info"],
        &["data", "daily"],
        &["data", "recommend"],
        &["song_list"],
        &["songlist"],
        &["songs"],
        &["list"],
    ];
    let items = kugou_playlist_track_items_from_response(&response, track_paths)
        .or_else(|| find_owned_track_array(&response))
        .ok_or_else(|| {
            format!(
                "酷狗每日推荐响应缺少歌曲列表字段（{}）。",
                describe_response_shape(&response)
            )
        })?;
    let tracks = items
        .iter()
        .filter_map(parse_kugou_track)
        .take(MAX_KUGOU_SEARCH_LIMIT as usize)
        .collect::<Vec<_>>();
    let total = response
        .get("data")
        .and_then(|data| {
            value_as_u64(data, "total")
                .or_else(|| value_as_u64(data, "count"))
                .or_else(|| value_as_u64(data, "total_count"))
        })
        .unwrap_or(tracks.len() as u64);
    let message = if tracks.is_empty() {
        "酷狗每日推荐没有返回可展示歌曲。".to_string()
    } else {
        format!("已读取 {} 首酷狗每日推荐歌曲。", tracks.len())
    };

    Ok(KugouSearchResult {
        keyword: "每日推荐".to_string(),
        tracks,
        total,
        message,
    })
}

pub fn search_songs(keyword: String, page: u64, limit: u64) -> Result<KugouSearchResult, String> {
    let keyword = normalize_query(&keyword, 80)?;
    let page = page.clamp(1, 20);
    let limit = limit.clamp(1, MAX_KUGOU_SEARCH_LIMIT);
    let url = format!(
        "{KUGOU_SEARCH_ENDPOINT}?format=json&keyword={}&page={page}&pagesize={limit}&showtype=1",
        url_encode_component(&keyword)
    );
    let body = http_get_json(&url)?;
    let status = value_as_i64(&body, "status").unwrap_or_default();
    if status != 1 {
        return Err(value_as_string(&body, "error")
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| "酷狗搜索失败，请稍后再试。".to_string()));
    }

    let data = body
        .get("data")
        .ok_or_else(|| "酷狗搜索返回缺少 data 字段".to_string())?;
    let total = value_as_u64(data, "total").unwrap_or_default();
    let tracks = data
        .get("info")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(parse_search_track)
                .take(limit as usize)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let message = if tracks.is_empty() {
        format!("没有搜索到“{keyword}”。")
    } else {
        format!("已搜索到 {} 首酷狗歌曲。", tracks.len())
    };

    Ok(KugouSearchResult {
        keyword,
        tracks,
        total,
        message,
    })
}

pub fn read_lyrics(
    hash: String,
    name: String,
    artist: String,
    duration_ms: Option<u64>,
) -> Result<KugouLyricsResult, String> {
    let hash = normalize_hash(&hash)?;
    let name = normalize_optional_query(&name, 120).unwrap_or_else(|| "酷狗歌曲".to_string());
    let artist = normalize_optional_query(&artist, 120).unwrap_or_default();
    let keyword = if artist.is_empty() {
        name.clone()
    } else {
        format!("{artist} - {name}")
    };
    let duration = duration_ms.unwrap_or_default();
    let search_url = format!(
        "{KUGOU_LYRIC_SEARCH_ENDPOINT}?ver=1&man=yes&client=pc&keyword={}&duration={duration}&hash={hash}",
        url_encode_component(&keyword)
    );
    let search_body = http_get_json(&search_url)?;
    let candidates = search_body
        .get("candidates")
        .and_then(Value::as_array)
        .ok_or_else(|| "酷狗歌词返回缺少候选列表".to_string())?;
    let candidate = candidates
        .iter()
        .find(|candidate| {
            value_as_string(candidate, "id").is_some()
                && value_as_string(candidate, "accesskey").is_some()
        })
        .ok_or_else(|| "没有找到可用的酷狗歌词。".to_string())?;
    let id = value_as_string(candidate, "id").ok_or_else(|| "酷狗歌词 ID 为空".to_string())?;
    let access_key = value_as_string(candidate, "accesskey")
        .ok_or_else(|| "酷狗歌词 accesskey 为空".to_string())?;
    let download_url = format!(
        "{KUGOU_LYRIC_DOWNLOAD_ENDPOINT}?ver=1&client=pc&id={}&accesskey={}&fmt=lrc&charset=utf8",
        url_encode_component(&id),
        url_encode_component(&access_key)
    );
    let download_body = http_get_json(&download_url)?;
    let encoded =
        value_as_string(&download_body, "content").ok_or_else(|| "酷狗歌词内容为空".to_string())?;
    let decoded = general_purpose::STANDARD
        .decode(encoded.trim())
        .map_err(|err| format!("酷狗歌词解码失败：{err}"))?;
    let mut content = String::from_utf8_lossy(&decoded).to_string();
    if content.chars().count() > MAX_KUGOU_LYRIC_CHARS {
        content = content.chars().take(MAX_KUGOU_LYRIC_CHARS).collect();
    }
    if content.trim().is_empty() {
        return Err("酷狗歌词内容为空。".to_string());
    }

    Ok(KugouLyricsResult {
        song_id: hash,
        content: content.clone(),
        lrc_content: Some(content),
        source: "kugou-lrc".to_string(),
        warnings: Vec::new(),
    })
}

pub fn get_song_quality_availability(
    app: &AppHandle,
    hash: String,
    hash_candidates: Option<Vec<String>>,
    album_audio_id: Option<u64>,
) -> Result<KugouQualityAvailability, String> {
    let hashes = normalize_hash_candidates(hash, hash_candidates)?;
    let mut credential = read_credential(app)?;
    let mut temporary_cookies = BTreeMap::new();
    let mut device_warning = None;

    let response = {
        let cookies = if let Some(credential) = credential.as_mut() {
            if let (Some(user_id), Some(token)) = (
                credential
                    .cookies
                    .get("userid")
                    .cloned()
                    .filter(|value| !value.trim().is_empty()),
                credential
                    .cookies
                    .get("token")
                    .cloned()
                    .filter(|value| !value.trim().is_empty()),
            ) {
                if let Err(err) = ensure_kugou_device_registered_cookies(
                    &mut credential.cookies,
                    &user_id,
                    &token,
                ) {
                    device_warning = Some(err);
                }
            } else {
                ensure_kugou_device_cookie_defaults(&mut credential.cookies);
            }
            &mut credential.cookies
        } else {
            ensure_kugou_device_cookie_defaults(&mut temporary_cookies);
            &mut temporary_cookies
        };

        get_kugou_privilege_lite(&hashes, album_audio_id, cookies)
    }?;

    if let Some(credential) = credential.as_ref() {
        write_credential(app, credential)?;
    }

    let signals = collect_kugou_quality_availability_signals(&response);
    let diagnostic = kugou_quality_availability_diagnostic(&response, device_warning.as_deref());
    let qualities = build_kugou_quality_availability_items(&signals);

    Ok(KugouQualityAvailability {
        hash: hashes.first().cloned().unwrap_or_default(),
        qualities,
        message: "已完成酷狗音质可用性预检查。".to_string(),
        diagnostic,
    })
}

pub fn get_song_playback_url(
    app: &AppHandle,
    hash: String,
    hash_candidates: Option<Vec<String>>,
    album_audio_id: Option<u64>,
    audio_id: Option<u64>,
    playback_quality: Option<String>,
) -> Result<KugouPlaybackUrl, String> {
    let hashes = normalize_hash_candidates(hash, hash_candidates)?;
    let quality_preference = normalize_kugou_quality_preference(playback_quality.as_deref());
    let (playback, probe) = get_direct_song_playback_url_with_probe(
        app,
        &hashes,
        album_audio_id,
        audio_id,
        &quality_preference,
    )?;
    register_kugou_proxy_session(
        app,
        hashes,
        album_audio_id,
        audio_id,
        quality_preference,
        playback,
        Some(probe),
    )
}

fn get_direct_song_playback_url_with_probe(
    app: &AppHandle,
    hashes: &[String],
    album_audio_id: Option<u64>,
    audio_id: Option<u64>,
    quality_preference: &str,
) -> Result<(KugouPlaybackUrl, KugouPlaybackProbe), String> {
    let mut logged_errors = Vec::new();
    let mut legacy_errors = Vec::new();
    if let Some(mut credential) = read_credential(app)? {
        if credential
            .cookies
            .get("token")
            .filter(|value| !value.trim().is_empty())
            .is_some()
            && credential
                .cookies
                .get("userid")
                .filter(|value| !value.trim().is_empty())
                .is_some()
        {
            for (hash_index, hash) in hashes.iter().enumerate() {
                let candidate_label = kugou_hash_candidate_label(hash_index, hashes.len());
                match get_registered_gateway_song_playback_url(
                    hash,
                    album_audio_id,
                    &mut credential,
                    quality_preference,
                ) {
                    Ok(playback) => {
                        let probe = probe_kugou_playback_url(&playback.url, playback.size);
                        if probe.likely_preview {
                            logged_errors.push(format!(
                                "{candidate_label}: 登录态 /v5/url 返回疑似试听片段，{}",
                                probe.message
                            ));
                        } else {
                            write_credential(app, &credential)?;
                            return Ok((playback, probe));
                        }
                    }
                    Err(err) => {
                        logged_errors
                            .push(format!("{candidate_label}: 登录态 /v5/url 失败：{err}"));
                    }
                }

                if quality_preference == "hires" {
                    continue;
                }

                match get_logged_in_song_playback_url(
                    hash,
                    album_audio_id,
                    audio_id,
                    &mut credential,
                    quality_preference,
                ) {
                    Ok(playback) => {
                        let probe = probe_kugou_playback_url(&playback.url, playback.size);
                        if probe.likely_preview {
                            logged_errors.push(format!(
                                "{candidate_label}: 登录态接口返回疑似试听片段，{}",
                                probe.message
                            ));
                            continue;
                        }
                        write_credential(app, &credential)?;
                        return Ok((playback, probe));
                    }
                    Err(err) => {
                        logged_errors.push(format!("{candidate_label}: {err}"));
                    }
                }
            }
            write_credential(app, &credential)?;
        }
    }

    if quality_preference == "hires" {
        return Err(kugou_hires_playback_failure_message(&logged_errors));
    }

    for (hash_index, hash) in hashes.iter().enumerate() {
        let candidate_label = kugou_hash_candidate_label(hash_index, hashes.len());
        match get_legacy_song_playback_url(hash, quality_preference) {
            Ok(playback) => {
                let probe = probe_kugou_playback_url(&playback.url, playback.size);
                if probe.likely_preview {
                    legacy_errors.push(format!(
                        "{candidate_label}: 旧接口返回疑似试听片段，{}",
                        probe.message
                    ));
                    continue;
                }
                return Ok((playback, probe));
            }
            Err(err) => legacy_errors.push(format!("{candidate_label}: {err}")),
        }
    }

    let legacy_error = summarize_playback_attempt_errors("酷狗播放链接获取失败", &legacy_errors);
    if logged_errors.is_empty() {
        if playback_errors_indicate_paid(&legacy_errors) {
            return Err(
                "普通酷狗接口没有返回可播放的完整音频。即使当前账号有会员，项目仍需要拿到酷狗登录态、设备注册和风控授权后的完整播放地址；项目不会绕过会员、版权、地区或付费限制。"
                    .to_string(),
            );
        }
        return Err(legacy_error);
    }

    if playback_errors_indicate_paid(&legacy_errors)
        && playback_errors_indicate_preview(&logged_errors)
    {
        return Err(
            "当前酷狗账号登录态没有拿到完整播放授权。项目已经尝试普通接口、设备注册后的 /v5/url 链路和登录态会员接口，但最终只拿到疑似试听片段；为了避免只能播放前面一小段，已停止播放。即使账号有会员，如果官方客户端可播但这里仍失败，通常说明还缺少官方客户端专用的设备、风控或网页播放授权。项目不会绕过会员、版权、地区或付费限制。"
                .to_string(),
        );
    }

    Err(format!(
        "{legacy_error}；登录态会员接口也失败：{}",
        summarize_playback_attempt_errors("已尝试登录态会员接口", &logged_errors)
    ))
}

fn register_kugou_proxy_session(
    app: &AppHandle,
    hashes: Vec<String>,
    album_audio_id: Option<u64>,
    audio_id: Option<u64>,
    quality_preference: String,
    playback: KugouPlaybackUrl,
    probe: Option<KugouPlaybackProbe>,
) -> Result<KugouPlaybackUrl, String> {
    let state = ensure_kugou_playback_proxy(app)?;
    let session_id = format!("{}{}", current_epoch_millis(), pseudo_alnum(18));
    let now_ms = current_epoch_millis() as u64;
    let probe = probe.unwrap_or_else(|| probe_kugou_playback_url(&playback.url, playback.size));
    {
        let mut sessions = state
            .sessions
            .lock()
            .map_err(|_| "酷狗播放代理会话锁定失败。".to_string())?;
        prune_kugou_proxy_sessions(&mut sessions, now_ms);
        sessions.insert(
            session_id.clone(),
            KugouPlaybackSession {
                hashes,
                album_audio_id,
                audio_id,
                quality_preference,
                playback: playback.clone(),
                probe: probe.clone(),
                created_at_ms: now_ms,
                refreshed_at_ms: now_ms,
                last_error: None,
                last_error_at_ms: None,
                last_range: None,
                refresh_count: 0,
            },
        );
    }

    Ok(KugouPlaybackUrl {
        url: format!("http://127.0.0.1:{}/kugou/play/{}", state.port, session_id),
        message: format!("{} 已切换为本机播放代理。", playback.message.clone()),
        proxy_diagnostic: Some(probe.message),
        proxy_likely_preview: probe.likely_preview,
        ..playback
    })
}

fn ensure_kugou_playback_proxy(app: &AppHandle) -> Result<KugouPlaybackProxyState, String> {
    let state = KUGOU_PLAYBACK_PROXY.get_or_init(|| Mutex::new(None));
    let mut guard = state
        .lock()
        .map_err(|_| "酷狗播放代理状态锁定失败。".to_string())?;
    if let Some(proxy) = guard.clone() {
        return Ok(proxy);
    }

    let listener = TcpListener::bind(("127.0.0.1", 0))
        .map_err(|err| format!("启动酷狗播放代理失败：{err}"))?;
    listener
        .set_nonblocking(true)
        .map_err(|err| format!("设置酷狗播放代理非阻塞失败：{err}"))?;
    let port = listener
        .local_addr()
        .map_err(|err| format!("读取酷狗播放代理端口失败：{err}"))?
        .port();
    let sessions = Arc::new(Mutex::new(BTreeMap::new()));
    let thread_sessions = Arc::clone(&sessions);
    let app_handle = app.clone();
    thread::spawn(move || run_kugou_playback_proxy(listener, app_handle, thread_sessions));

    let proxy = KugouPlaybackProxyState { port, sessions };
    *guard = Some(proxy.clone());
    Ok(proxy)
}

fn run_kugou_playback_proxy(
    listener: TcpListener,
    app: AppHandle,
    sessions: Arc<Mutex<BTreeMap<String, KugouPlaybackSession>>>,
) {
    loop {
        match listener.accept() {
            Ok((stream, _)) => {
                let app = app.clone();
                let sessions = Arc::clone(&sessions);
                thread::spawn(move || {
                    if let Err(err) = handle_kugou_proxy_connection(stream, app, sessions) {
                        eprintln!("酷狗播放代理请求失败：{err}");
                    }
                });
            }
            Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(80));
            }
            Err(err) => {
                eprintln!("酷狗播放代理监听失败：{err}");
                thread::sleep(Duration::from_millis(250));
            }
        }
    }
}

fn handle_kugou_proxy_connection(
    mut stream: TcpStream,
    app: AppHandle,
    sessions: Arc<Mutex<BTreeMap<String, KugouPlaybackSession>>>,
) -> Result<(), String> {
    stream
        .set_read_timeout(Some(Duration::from_secs(10)))
        .map_err(|err| err.to_string())?;
    stream
        .set_write_timeout(Some(Duration::from_secs(45)))
        .map_err(|err| err.to_string())?;
    let _ = stream.set_nodelay(true);

    let request = read_kugou_proxy_request(&mut stream)?;
    if request.method == "OPTIONS" {
        return write_kugou_proxy_empty_response(&mut stream, 204, "No Content");
    }
    if request.method != "GET" && request.method != "HEAD" {
        return write_kugou_proxy_error(&mut stream, 405, "Method Not Allowed", "请求方法不支持。");
    }

    let session_id =
        kugou_proxy_session_id(&request.path).ok_or_else(|| "酷狗代理路径无效。".to_string())?;
    let mut session = {
        let guard = sessions
            .lock()
            .map_err(|_| "酷狗播放代理会话锁定失败。".to_string())?;
        guard
            .get(&session_id)
            .cloned()
            .ok_or_else(|| "酷狗播放会话不存在或已过期。".to_string())?
    };

    match proxy_kugou_audio_request(&mut stream, &request, &session.playback.url) {
        Ok(outcome) => {
            handle_kugou_proxy_transfer_outcome(&sessions, &session_id, &request, outcome)
        }
        Err(first_err) => {
            let refreshed = get_direct_song_playback_url_with_probe(
                &app,
                &session.hashes,
                session.album_audio_id,
                session.audio_id,
                &session.quality_preference,
            );
            match refreshed {
                Ok((playback, probe)) => {
                    session.playback = playback;
                    session.probe = probe;
                    session.refreshed_at_ms = current_epoch_millis() as u64;
                    session.refresh_count = session.refresh_count.saturating_add(1);
                    {
                        let mut guard = sessions
                            .lock()
                            .map_err(|_| "酷狗播放代理会话锁定失败。".to_string())?;
                        guard.insert(session_id.clone(), session.clone());
                    }
                    match proxy_kugou_audio_request(&mut stream, &request, &session.playback.url) {
                        Ok(outcome) => handle_kugou_proxy_transfer_outcome(
                            &sessions,
                            &session_id,
                            &request,
                            outcome,
                        ),
                        Err(second_err) => {
                            let message = format!(
                                "酷狗代理读取失败，已刷新播放链接但仍无法读取：{}",
                                sanitize_proxy_error(&second_err)
                            );
                            update_kugou_proxy_session_error(
                                &sessions,
                                &session_id,
                                &request,
                                message.clone(),
                            );
                            write_kugou_proxy_error(&mut stream, 502, "Bad Gateway", &message)
                        }
                    }
                }
                Err(refresh_err) => {
                    let message = format!(
                        "酷狗代理读取失败，刷新播放链接也失败：{}",
                        sanitize_proxy_error(&refresh_err)
                    );
                    update_kugou_proxy_session_error(
                        &sessions,
                        &session_id,
                        &request,
                        message.clone(),
                    );
                    write_kugou_proxy_error(&mut stream, 502, "Bad Gateway", &message)
                        .map_err(|err| format!("{first_err}；{err}"))
                }
            }
        }
    }
}

fn read_kugou_proxy_request(stream: &mut TcpStream) -> Result<KugouProxyRequest, String> {
    let mut buffer = Vec::new();
    let mut temp = [0_u8; 4096];
    loop {
        let bytes_read = stream.read(&mut temp).map_err(|err| err.to_string())?;
        if bytes_read == 0 {
            break;
        }
        buffer.extend_from_slice(&temp[..bytes_read]);
        if buffer.len() > 64 * 1024 {
            return Err("酷狗代理请求头过大。".to_string());
        }
        if find_http_header_end(&buffer).is_some() {
            break;
        }
    }

    let header_end =
        find_http_header_end(&buffer).ok_or_else(|| "酷狗代理 HTTP 请求头不完整。".to_string())?;
    let headers_text = String::from_utf8_lossy(&buffer[..header_end]);
    let mut lines = headers_text.lines();
    let request_line = lines
        .next()
        .ok_or_else(|| "酷狗代理 HTTP 请求行缺失。".to_string())?;
    let mut request_parts = request_line.split_whitespace();
    let method = request_parts.next().unwrap_or_default().to_string();
    let path = request_parts.next().unwrap_or_default().to_string();
    let headers = lines
        .filter_map(|line| {
            let (name, value) = line.split_once(':')?;
            Some((name.trim().to_ascii_lowercase(), value.trim().to_string()))
        })
        .collect::<BTreeMap<_, _>>();

    Ok(KugouProxyRequest {
        method,
        path,
        headers,
    })
}

pub fn playback_proxy_status(proxy_url: String) -> Result<KugouPlaybackProxyStatus, String> {
    let session_id = extract_kugou_proxy_session_id(&proxy_url)
        .ok_or_else(|| "酷狗播放代理地址无效。".to_string())?;
    let state = KUGOU_PLAYBACK_PROXY
        .get_or_init(|| Mutex::new(None))
        .lock()
        .map_err(|_| "酷狗播放代理状态锁定失败。".to_string())?
        .clone()
        .ok_or_else(|| "酷狗播放代理尚未启动。".to_string())?;
    let sessions = state
        .sessions
        .lock()
        .map_err(|_| "酷狗播放代理会话锁定失败。".to_string())?;
    let session = sessions
        .get(&session_id)
        .ok_or_else(|| "酷狗播放会话不存在或已过期。".to_string())?;
    let message = session
        .last_error
        .clone()
        .or_else(|| Some(session.probe.message.clone()))
        .unwrap_or_else(|| "酷狗播放代理会话正常。".to_string());

    Ok(KugouPlaybackProxyStatus {
        ok: session.last_error.is_none() && !session.probe.likely_preview,
        message,
        probe_message: Some(session.probe.message.clone()),
        likely_preview: session.probe.likely_preview,
        last_error: session.last_error.clone(),
        last_range: session.last_range.clone(),
        refresh_count: session.refresh_count,
    })
}

fn probe_kugou_playback_url(playback_url: &str, expected_size: Option<u64>) -> KugouPlaybackProbe {
    let first = request_kugou_probe_range(playback_url, "bytes=0-1");
    let Ok(first) = first else {
        return KugouPlaybackProbe {
            message: "播放代理预检失败：无法读取音频开头 Range。".to_string(),
            likely_preview: false,
        };
    };

    let total_size = first
        .content_range_total
        .or(first.content_length)
        .or(expected_size);
    let range_supported =
        first.status == 206 || first.accept_ranges || first.content_range_total.is_some();
    let size_mismatch = match (expected_size, total_size) {
        (Some(expected), Some(total)) if expected > 0 && total > 0 => {
            total.saturating_add(64 * 1024) < expected
        }
        _ => false,
    };

    if let Some(total) = total_size.filter(|value| *value > 128 * 1024) {
        let tail_len = total.min(64 * 1024);
        let tail_start = total.saturating_sub(tail_len);
        let tail_range = format!("bytes={tail_start}-");
        match request_kugou_probe_range(playback_url, &tail_range) {
            Ok(tail) if tail.status == 206 || tail.status == 200 => {
                if size_mismatch {
                    return KugouPlaybackProbe {
                        message:
                            "播放代理预检显示 CDN 返回的可读大小小于歌曲摘要，可能是试听片段。"
                                .to_string(),
                        likely_preview: true,
                    };
                }
                if range_supported {
                    return KugouPlaybackProbe {
                        message: "播放代理预检通过：开头和后半段 Range 均可读取。".to_string(),
                        likely_preview: false,
                    };
                }
                return KugouPlaybackProbe {
                    message: "播放代理预检显示音频可读取，但 CDN 未明确声明支持 Range，拖动进度仍可能失败。".to_string(),
                    likely_preview: false,
                };
            }
            Ok(tail) => {
                return KugouPlaybackProbe {
                    message: format!(
                        "播放代理预检显示后半段 Range 被 CDN 拒绝，HTTP {}，可能是试听片段或权限限制。",
                        tail.status
                    ),
                    likely_preview: true,
                };
            }
            Err(_) => {
                return KugouPlaybackProbe {
                    message: "播放代理预检显示后半段 Range 读取失败，可能是试听片段、临时链接或网络限制。".to_string(),
                    likely_preview: true,
                };
            }
        }
    }

    KugouPlaybackProbe {
        message: if range_supported {
            "播放代理预检通过：音频开头 Range 可读取。".to_string()
        } else {
            "播放代理预检显示 CDN 可能不支持 Range，拖动进度可能失败。".to_string()
        },
        likely_preview: size_mismatch,
    }
}

struct KugouRangeProbe {
    status: u16,
    content_length: Option<u64>,
    content_range_total: Option<u64>,
    accept_ranges: bool,
}

enum KugouProxyTransferOutcome {
    Completed,
    ClientDisconnected,
    UpstreamInterrupted(String),
}

fn request_kugou_probe_range(playback_url: &str, range: &str) -> Result<KugouRangeProbe, String> {
    let client = Client::builder()
        .user_agent(KUGOU_USER_AGENT)
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|err| format!("创建酷狗预检请求客户端失败：{err}"))?;
    let response = client
        .get(playback_url)
        .header(REFERER, "https://www.kugou.com/")
        .header(USER_AGENT, KUGOU_USER_AGENT)
        .header(ACCEPT_ENCODING, "identity")
        .header(RANGE, range)
        .send()
        .map_err(|err| format!("酷狗预检请求失败：{err}"))?;
    let status = response.status().as_u16();
    let headers = response.headers().clone();
    Ok(KugouRangeProbe {
        status,
        content_length: headers
            .get(CONTENT_LENGTH)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse::<u64>().ok()),
        content_range_total: headers
            .get(CONTENT_RANGE)
            .and_then(|value| value.to_str().ok())
            .and_then(parse_content_range_total),
        accept_ranges: headers
            .get(ACCEPT_RANGES)
            .and_then(|value| value.to_str().ok())
            .map(|value| value.to_ascii_lowercase().contains("bytes"))
            .unwrap_or(false),
    })
}

fn update_kugou_proxy_session_success(
    sessions: &Arc<Mutex<BTreeMap<String, KugouPlaybackSession>>>,
    session_id: &str,
    request: &KugouProxyRequest,
) {
    let Ok(mut guard) = sessions.lock() else {
        return;
    };
    if let Some(session) = guard.get_mut(session_id) {
        session.last_range = request.headers.get("range").cloned();
        session.last_error = None;
        session.last_error_at_ms = None;
    }
}

fn update_kugou_proxy_session_error(
    sessions: &Arc<Mutex<BTreeMap<String, KugouPlaybackSession>>>,
    session_id: &str,
    request: &KugouProxyRequest,
    message: String,
) {
    let Ok(mut guard) = sessions.lock() else {
        return;
    };
    if let Some(session) = guard.get_mut(session_id) {
        session.last_range = request.headers.get("range").cloned();
        session.last_error = Some(message);
        session.last_error_at_ms = Some(current_epoch_millis() as u64);
    }
}

fn handle_kugou_proxy_transfer_outcome(
    sessions: &Arc<Mutex<BTreeMap<String, KugouPlaybackSession>>>,
    session_id: &str,
    request: &KugouProxyRequest,
    outcome: KugouProxyTransferOutcome,
) -> Result<(), String> {
    match outcome {
        KugouProxyTransferOutcome::Completed | KugouProxyTransferOutcome::ClientDisconnected => {
            update_kugou_proxy_session_success(sessions, session_id, request);
        }
        KugouProxyTransferOutcome::UpstreamInterrupted(message) => {
            update_kugou_proxy_session_error(sessions, session_id, request, message);
        }
    }

    Ok(())
}

fn proxy_kugou_audio_request(
    stream: &mut TcpStream,
    request: &KugouProxyRequest,
    playback_url: &str,
) -> Result<KugouProxyTransferOutcome, String> {
    let client = Client::builder()
        .user_agent(KUGOU_USER_AGENT)
        .connect_timeout(Duration::from_secs(
            KUGOU_PROXY_UPSTREAM_CONNECT_TIMEOUT_SECONDS,
        ))
        .build()
        .map_err(|err| format!("创建酷狗代理请求客户端失败：{err}"))?;
    let method = if request.method == "HEAD" {
        reqwest::Method::HEAD
    } else {
        reqwest::Method::GET
    };
    let mut builder = client
        .request(method, playback_url)
        .header(REFERER, "https://www.kugou.com/")
        .header(USER_AGENT, KUGOU_USER_AGENT)
        .header(ACCEPT_ENCODING, "identity");
    if let Some(range) = request
        .headers
        .get("range")
        .filter(|value| !value.is_empty())
    {
        builder = builder.header(RANGE, range);
    }

    let response = builder
        .send()
        .map_err(|err| format!("酷狗 CDN 请求失败：{err}"))?;
    let status = response.status();
    if should_refresh_kugou_proxy_status(status) {
        return Err(format_kugou_proxy_status_error(status, request));
    }
    if !status.is_success() {
        return Err(format_kugou_proxy_status_error(status, request));
    }

    write_kugou_proxy_upstream_headers(stream, status, response.headers())?;
    if request.method == "HEAD" {
        return Ok(KugouProxyTransferOutcome::Completed);
    }

    Ok(stream_kugou_proxy_body(response, stream))
}

fn stream_kugou_proxy_body<R>(mut response: R, stream: &mut TcpStream) -> KugouProxyTransferOutcome
where
    R: Read + Send + 'static,
{
    let (sender, receiver) = mpsc::sync_channel::<Result<Vec<u8>, String>>(2);
    thread::spawn(move || {
        let mut buffer = [0_u8; 64 * 1024];
        loop {
            match response.read(&mut buffer) {
                Ok(0) => {
                    let _ = sender.send(Ok(Vec::new()));
                    break;
                }
                Ok(bytes_read) => {
                    if sender.send(Ok(buffer[..bytes_read].to_vec())).is_err() {
                        break;
                    }
                }
                Err(err) => {
                    let _ = sender.send(Err(err.to_string()));
                    break;
                }
            }
        }
    });

    let mut transferred = 0_u64;
    loop {
        let chunk = match receiver.recv_timeout(Duration::from_secs(
            KUGOU_PROXY_UPSTREAM_READ_TIMEOUT_SECONDS,
        )) {
            Ok(Ok(chunk)) if chunk.is_empty() => return KugouProxyTransferOutcome::Completed,
            Ok(Ok(chunk)) => chunk,
            Ok(Err(err)) => {
                return KugouProxyTransferOutcome::UpstreamInterrupted(format!(
                    "酷狗 CDN 音频流读取中断：{err}；已转发 {transferred} 字节"
                ));
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                return KugouProxyTransferOutcome::UpstreamInterrupted(format!(
                    "酷狗 CDN 音频流超过 {} 秒未返回数据；已转发 {transferred} 字节",
                    KUGOU_PROXY_UPSTREAM_READ_TIMEOUT_SECONDS
                ));
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                return KugouProxyTransferOutcome::UpstreamInterrupted(format!(
                    "酷狗 CDN 音频流读取线程提前结束；已转发 {transferred} 字节"
                ));
            }
        };

        if let Err(err) = stream.write_all(&chunk) {
            if is_kugou_proxy_client_disconnect(&err) {
                return KugouProxyTransferOutcome::ClientDisconnected;
            }
            return KugouProxyTransferOutcome::UpstreamInterrupted(format!(
                "写入本机播放器音频流失败：{err}；已转发 {transferred} 字节"
            ));
        }
        transferred = transferred.saturating_add(chunk.len() as u64);
    }
}

fn is_kugou_proxy_client_disconnect(err: &std::io::Error) -> bool {
    matches!(
        err.kind(),
        std::io::ErrorKind::BrokenPipe
            | std::io::ErrorKind::ConnectionAborted
            | std::io::ErrorKind::ConnectionReset
            | std::io::ErrorKind::NotConnected
            | std::io::ErrorKind::TimedOut
    ) || matches!(err.raw_os_error(), Some(10053 | 10054 | 10058))
}

fn write_kugou_proxy_upstream_headers(
    stream: &mut TcpStream,
    status: StatusCode,
    headers: &HeaderMap,
) -> Result<(), String> {
    let reason = status.canonical_reason().unwrap_or("OK");
    let mut response_headers = format!(
        "HTTP/1.1 {} {}\r\nAccess-Control-Allow-Origin: *\r\nAccess-Control-Allow-Headers: Range, Content-Type\r\nAccess-Control-Expose-Headers: Content-Length, Content-Range, Accept-Ranges\r\nAccept-Ranges: bytes\r\nConnection: close\r\n",
        status.as_u16(),
        reason,
    );
    for name in [
        CONTENT_TYPE,
        CONTENT_LENGTH,
        CONTENT_RANGE,
        ACCEPT_RANGES,
        CACHE_CONTROL,
    ] {
        if let Some(value) = headers.get(&name).and_then(|value| value.to_str().ok()) {
            response_headers.push_str(name.as_str());
            response_headers.push_str(": ");
            response_headers.push_str(value);
            response_headers.push_str("\r\n");
        }
    }
    if headers.get(CONTENT_TYPE).is_none() {
        response_headers.push_str("Content-Type: audio/mpeg\r\n");
    }
    response_headers.push_str("\r\n");
    stream
        .write_all(response_headers.as_bytes())
        .map_err(|err| err.to_string())
}

fn write_kugou_proxy_empty_response(
    stream: &mut TcpStream,
    status: u16,
    reason: &'static str,
) -> Result<(), String> {
    let response = format!(
        "HTTP/1.1 {status} {reason}\r\nAccess-Control-Allow-Origin: *\r\nAccess-Control-Allow-Headers: Range, Content-Type\r\nAccess-Control-Allow-Methods: GET, HEAD, OPTIONS\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
    );
    stream
        .write_all(response.as_bytes())
        .map_err(|err| err.to_string())
}

fn write_kugou_proxy_error(
    stream: &mut TcpStream,
    status: u16,
    reason: &'static str,
    message: &str,
) -> Result<(), String> {
    let body = message.as_bytes();
    let headers = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: {}\r\nAccess-Control-Allow-Origin: *\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream
        .write_all(headers.as_bytes())
        .and_then(|_| stream.write_all(body))
        .map_err(|err| err.to_string())
}

fn should_refresh_kugou_proxy_status(status: StatusCode) -> bool {
    matches!(
        status.as_u16(),
        401 | 403 | 404 | 408 | 410 | 416 | 429 | 500..=599
    )
}

fn format_kugou_proxy_status_error(status: StatusCode, request: &KugouProxyRequest) -> String {
    let range = request
        .headers
        .get("range")
        .map(|value| format!("，请求 Range {value}"))
        .unwrap_or_default();
    match status.as_u16() {
        416 => format!(
            "酷狗 CDN 拒绝后半段 Range{range}，该链接可能只返回试听片段或可读长度小于歌曲摘要。"
        ),
        401 | 403 => format!("酷狗 CDN 拒绝访问{range}，可能是登录态、会员、版权或地区限制。"),
        404 | 410 => format!("酷狗 CDN 返回链接失效{range}，需要刷新临时播放链接。"),
        429 => format!("酷狗 CDN 请求过于频繁{range}，稍后重试。"),
        _ => format!("酷狗 CDN 返回 HTTP {status}{range}"),
    }
}

fn kugou_proxy_session_id(path: &str) -> Option<String> {
    let path = path.split('?').next().unwrap_or(path);
    let session_id = path.strip_prefix(KUGOU_PROXY_PATH_PREFIX)?;
    if session_id.is_empty()
        || session_id.len() > 64
        || !session_id.chars().all(|char| char.is_ascii_alphanumeric())
    {
        return None;
    }

    Some(session_id.to_string())
}

fn extract_kugou_proxy_session_id(value: &str) -> Option<String> {
    let value = value.trim();
    if let Some(index) = value.find(KUGOU_PROXY_PATH_PREFIX) {
        return kugou_proxy_session_id(&value[index..]);
    }

    kugou_proxy_session_id(value)
}

fn parse_content_range_total(value: &str) -> Option<u64> {
    let total = value.trim().rsplit('/').next()?.trim();
    if total == "*" {
        return None;
    }

    total.parse::<u64>().ok()
}

fn prune_kugou_proxy_sessions(sessions: &mut BTreeMap<String, KugouPlaybackSession>, now_ms: u64) {
    sessions.retain(|_, session| {
        now_ms.saturating_sub(session.created_at_ms) <= KUGOU_PROXY_SESSION_TTL_MS
    });
    while sessions.len() >= MAX_KUGOU_PROXY_SESSIONS {
        let Some(oldest_key) = sessions.keys().next().cloned() else {
            break;
        };
        sessions.remove(&oldest_key);
    }
}

fn find_http_header_end(buffer: &[u8]) -> Option<usize> {
    buffer.windows(4).position(|window| window == b"\r\n\r\n")
}

fn sanitize_proxy_error(message: &str) -> String {
    if message.len() > 180 {
        format!("{}...", message.chars().take(180).collect::<String>())
    } else {
        message.to_string()
    }
}

fn get_legacy_song_playback_url(
    hash: &str,
    quality_preference: &str,
) -> Result<KugouPlaybackUrl, String> {
    let url = format!(
        "{KUGOU_PLAY_INFO_ENDPOINT}?cmd=playInfo&hash={}",
        url_encode_component(&hash)
    );
    let body = http_get_json(&url)?;
    let play_url = value_as_string(&body, "url")
        .filter(|value| value.starts_with("http://") || value.starts_with("https://"))
        .ok_or_else(|| {
            let reason = value_as_string(&body, "error")
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| "可能需要会员、版权受限、地区受限或接口暂不可用".to_string());
            format!(
                "酷狗播放链接为空：{reason}（{}）",
                kugou_playback_failure_diagnostic(&body)
            )
        })?;

    let bitrate = normalize_kugou_bitrate(
        value_as_u64(&body, "bitRate").or_else(|| value_as_u64(&body, "bitrate")),
    );
    let file_type = value_as_string(&body, "extName").filter(|value| !value.is_empty());
    let observed_quality_level = observed_kugou_quality_level(&body, bitrate, file_type.as_deref());
    if !kugou_observed_quality_satisfies_preference(
        quality_preference,
        observed_quality_level.as_deref(),
    ) {
        return Err(format!(
            "{}（{}）",
            kugou_quality_mismatch_message(quality_preference, observed_quality_level.as_deref()),
            kugou_playback_failure_diagnostic(&body)
        ));
    }
    let quality_level = observed_quality_level
        .or_else(|| resolve_kugou_quality_level(&body, None, bitrate, file_type.as_deref()));
    let quality_label = kugou_quality_label_string(quality_level.as_deref());

    Ok(KugouPlaybackUrl {
        hash: hash.to_string(),
        url: play_url,
        quality_level,
        quality_label,
        bitrate,
        duration_ms: value_as_u64(&body, "timeLength")
            .filter(|value| *value > 0)
            .map(|value| value * 1000)
            .or_else(|| {
                body.get("extra")
                    .and_then(|extra| value_as_u64(extra, "128timelength"))
            }),
        file_type,
        size: value_as_u64(&body, "fileSize").filter(|value| *value > 0),
        message: "已获取酷狗临时播放地址。".to_string(),
        proxy_diagnostic: None,
        proxy_likely_preview: false,
    })
}

fn get_registered_gateway_song_playback_url(
    hash: &str,
    album_audio_id: Option<u64>,
    credential: &mut KugouCredential,
    quality_preference: &str,
) -> Result<KugouPlaybackUrl, String> {
    let user_id = credential
        .cookies
        .get("userid")
        .cloned()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| "酷狗登录状态缺少用户 ID，请重新扫码登录。".to_string())?;
    let token = credential
        .cookies
        .get("token")
        .cloned()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| "酷狗登录状态缺少 token，请重新扫码登录。".to_string())?;
    ensure_kugou_device_registered(credential, &user_id, &token)?;

    let is_lite = true;
    let request_hash = hash.to_ascii_lowercase();
    let mut quality_errors = Vec::new();
    for quality in kugou_gateway_quality_candidates(quality_preference) {
        let mut params = BTreeMap::new();
        params.insert("album_id".to_string(), "0".to_string());
        params.insert("area_code".to_string(), "1".to_string());
        params.insert("hash".to_string(), request_hash.clone());
        params.insert("ssa_flag".to_string(), "is_fromtrack".to_string());
        params.insert("version".to_string(), "11430".to_string());
        params.insert(
            "page_id".to_string(),
            if is_lite { "967177915" } else { "151369488" }.to_string(),
        );
        params.insert("quality".to_string(), quality.to_string());
        params.insert(
            "album_audio_id".to_string(),
            album_audio_id.unwrap_or_default().to_string(),
        );
        params.insert("behavior".to_string(), "play".to_string());
        params.insert(
            "pid".to_string(),
            if is_lite { "411" } else { "2" }.to_string(),
        );
        params.insert("cmd".to_string(), "26".to_string());
        params.insert("pidversion".to_string(), "3001".to_string());
        params.insert("IsFreePart".to_string(), "0".to_string());
        params.insert(
            "ppage_id".to_string(),
            if is_lite {
                "356753938,823673182,967485191"
            } else {
                "463467626,350369493,788954147"
            }
            .to_string(),
        );
        params.insert("cdnBackup".to_string(), "1".to_string());
        params.insert("module".to_string(), String::new());
        params.insert("clientver".to_string(), "11430".to_string());

        let response = match kugou_android_get_json_with_client(
            KUGOU_GATEWAY_BASE,
            "/v5/url",
            params,
            credential,
            Some("trackercdn.kugou.com"),
            Some(&request_hash),
            KUGOU_APPID,
            KUGOU_CLIENTVER,
            KUGOU_SIGNATURE_ANDROID_SECRET,
        ) {
            Ok(response) => response,
            Err(err) => {
                quality_errors.push(format!("{quality}: {err}"));
                continue;
            }
        };
        let Some(play_url) = find_play_url(&response) else {
            let reason = playback_failure_reason(&response).unwrap_or_else(|| {
                "可能需要会员、版权受限、地区受限、设备注册未通过或该音质不可用".to_string()
            });
            quality_errors.push(format!(
                "{quality}: {reason}（{}）",
                kugou_playback_failure_diagnostic(&response)
            ));
            continue;
        };

        let bitrate = normalize_kugou_bitrate(find_u64_by_keys(
            &response,
            &["bitrate", "bitRate", "quality"],
        ));
        let file_type =
            find_string_by_keys(&response, &["file_type", "fileType", "ext", "extName"])
                .filter(|value| !value.is_empty());
        let observed_quality_level =
            observed_kugou_quality_level(&response, bitrate, file_type.as_deref());
        if !kugou_observed_quality_satisfies_preference(
            quality_preference,
            observed_quality_level.as_deref(),
        ) {
            quality_errors.push(format!(
                "{quality}: {}（{}）",
                kugou_quality_mismatch_message(
                    quality_preference,
                    observed_quality_level.as_deref()
                ),
                kugou_playback_failure_diagnostic(&response)
            ));
            continue;
        }
        let quality_level = observed_quality_level.or_else(|| {
            resolve_kugou_quality_level(&response, Some(quality), bitrate, file_type.as_deref())
        });
        let quality_label = kugou_quality_label_string(quality_level.as_deref());

        return Ok(KugouPlaybackUrl {
            hash: hash.to_string(),
            url: play_url,
            quality_level,
            quality_label,
            bitrate,
            duration_ms: find_u64_by_keys(
                &response,
                &[
                    "duration_ms",
                    "durationMs",
                    "timelength",
                    "timeLength",
                    "time",
                ],
            )
            .filter(|value| *value > 0)
            .map(|value| if value > 10_000 { value } else { value * 1000 }),
            file_type,
            size: find_u64_by_keys(&response, &["size", "fileSize", "filesize"])
                .filter(|value| *value > 0),
            message: format!(
                "已通过酷狗登录态设备注册接口获取临时播放地址（{}）。",
                kugou_quality_label(quality)
            ),
            proxy_diagnostic: None,
            proxy_likely_preview: false,
        });
    }

    Err(format!(
        "酷狗 /v5/url 播放链接为空：{}",
        summarize_playback_attempt_errors("已尝试音质候选", &quality_errors)
    ))
}

fn get_logged_in_song_playback_url(
    hash: &str,
    album_audio_id: Option<u64>,
    audio_id: Option<u64>,
    credential: &mut KugouCredential,
    quality_preference: &str,
) -> Result<KugouPlaybackUrl, String> {
    let mut token_refreshed_for_missing_vip = false;
    let mut token_refreshed_after_error = false;
    let response = 'request: loop {
        let user_id = credential
            .cookies
            .get("userid")
            .cloned()
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| "酷狗登录状态缺少用户 ID，请重新扫码登录。".to_string())?;
        let token = credential
            .cookies
            .get("token")
            .cloned()
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| "酷狗登录状态缺少 token，请重新扫码登录。".to_string())?;
        if first_cookie_value(
            &credential.cookies,
            &["vip_token", "vipToken", "viptoken", "kg_vip_token"],
        )
        .is_none()
        {
            if token_refreshed_for_missing_vip {
                return Err(
                    "酷狗会员播放缺少 vip_token；已尝试通过 /login/token 刷新但仍未返回会员 token，请清除酷狗登录后重新扫码。"
                        .to_string(),
                );
            }
            token_refreshed_for_missing_vip = true;
            refresh_kugou_login_token(credential)?;
            continue 'request;
        }
        let vip_token = first_cookie_value(
            &credential.cookies,
            &["vip_token", "vipToken", "viptoken", "kg_vip_token"],
        )
        .ok_or_else(|| {
            "酷狗会员播放缺少 vip_token；已尝试通过 /login/token 刷新但仍未返回会员 token，请清除酷狗登录后重新扫码。"
                .to_string()
        })?;
        let vip_type = first_cookie_value(
            &credential.cookies,
            &["vip_type", "vipType", "viptype", "kg_vip_type"],
        )
        .unwrap_or_else(|| "0".to_string());
        let vip_type_number = vip_type.trim().parse::<u64>().unwrap_or_default();
        let dfid = first_cookie_value(&credential.cookies, &["dfid", "DFID", "KUGOU_API_DFID"])
            .unwrap_or_else(|| pseudo_alnum(24));
        credential
            .cookies
            .entry("dfid".to_string())
            .or_insert_with(|| dfid.clone());
        let mid = credential
            .cookies
            .get("KUGOU_API_MID")
            .cloned()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| calculate_mid(hash));
        let collect_time = current_epoch_millis() as u64;
        let tracker_key = playback_tracker_key(hash, &mid, &user_id);
        let bodies = playback_request_bodies(
            hash,
            album_audio_id,
            audio_id,
            &user_id,
            &token,
            &vip_token,
            &tracker_key,
            collect_time,
            vip_type_number,
            quality_preference,
        );
        if bodies.is_empty() {
            return Err(kugou_priv_url_unsupported_quality_message(
                quality_preference,
            ));
        }
        let mut body_errors = Vec::new();
        for (label, body) in bodies {
            let response = kugou_android_post_json_with_client(
                KUGOU_TRACKER_BASE,
                "/v6/priv_url",
                BTreeMap::new(),
                body,
                &mut credential.cookies,
                None,
                KUGOU_APPID,
                KUGOU_CLIENTVER,
                KUGOU_SIGNATURE_ANDROID_SECRET,
            )?;
            match ensure_kugou_success(&response, "酷狗会员播放链接获取") {
                Ok(()) => break 'request response,
                Err(err) if is_kugou_unmarshal_error(&response) => {
                    body_errors.push(format!(
                        "{label}: {}",
                        append_kugou_playback_diagnostic(err, &response)
                    ));
                }
                Err(err) if is_kugou_token_api_error(&response) => {
                    if token_refreshed_after_error {
                        return Err(format!(
                            "酷狗会员播放 token 刷新后仍无效，请清除酷狗登录后重新扫码。原始错误：{}",
                            append_kugou_playback_diagnostic(err, &response)
                        ));
                    }
                    token_refreshed_after_error = true;
                    refresh_kugou_login_token(credential)?;
                    continue 'request;
                }
                Err(err) => return Err(append_kugou_playback_diagnostic(err, &response)),
            }
        }

        return Err(if body_errors.is_empty() {
            "酷狗会员播放链接获取失败：请求体候选均未返回可用播放地址。".to_string()
        } else {
            summarize_playback_attempt_errors("酷狗会员播放请求体不被接口接受", &body_errors)
        });
    };
    let play_url = find_play_url(&response).ok_or_else(|| {
        let reason = playback_failure_reason(&response).unwrap_or_else(|| {
            "可能需要重新登录、补充 dfid 设备验证、版权受限或该音质不可用".to_string()
        });
        format!(
            "酷狗会员播放链接为空：{reason}（{}）",
            kugou_playback_failure_diagnostic(&response)
        )
    })?;

    let bitrate = normalize_kugou_bitrate(find_u64_by_keys(
        &response,
        &["bitrate", "bitRate", "quality", "audio_quality"],
    ));
    let file_type = find_string_by_keys(&response, &["file_type", "fileType", "ext", "extName"])
        .filter(|value| !value.is_empty());
    let observed_quality_level =
        observed_kugou_quality_level(&response, bitrate, file_type.as_deref());
    if !kugou_observed_quality_satisfies_preference(
        quality_preference,
        observed_quality_level.as_deref(),
    ) {
        return Err(format!(
            "{}（{}）",
            kugou_quality_mismatch_message(quality_preference, observed_quality_level.as_deref()),
            kugou_playback_failure_diagnostic(&response)
        ));
    }
    let quality_level = observed_quality_level.or_else(|| match quality_preference {
        "highest" => None,
        value => normalize_kugou_returned_quality(value),
    });
    let quality_label = kugou_quality_label_string(quality_level.as_deref());

    Ok(KugouPlaybackUrl {
        hash: hash.to_string(),
        url: play_url,
        quality_level,
        quality_label,
        bitrate,
        duration_ms: find_u64_by_keys(
            &response,
            &[
                "duration_ms",
                "durationMs",
                "timelength",
                "timeLength",
                "duration",
            ],
        )
        .filter(|value| *value > 0)
        .map(|value| if value > 10_000 { value } else { value * 1000 }),
        file_type,
        size: find_u64_by_keys(&response, &["size", "fileSize", "filesize"])
            .filter(|value| *value > 0),
        message: "已通过酷狗登录态获取临时播放地址。".to_string(),
        proxy_diagnostic: None,
        proxy_likely_preview: false,
    })
}

fn playback_request_bodies(
    hash: &str,
    album_audio_id: Option<u64>,
    audio_id: Option<u64>,
    user_id: &str,
    token: &str,
    vip_token: &str,
    tracker_key: &str,
    collect_time: u64,
    vip_type: u64,
    quality_preference: &str,
) -> Vec<(&'static str, Value)> {
    let mut attempts = Vec::new();
    let qualities = kugou_priv_url_quality_candidates(quality_preference);
    if qualities.is_empty() {
        return attempts;
    }
    attempts.push((
        "数值 album_audio_id",
        playback_request_body(
            hash,
            album_audio_id.map(Value::from),
            audio_id.map(Value::from),
            Value::String(user_id.to_string()),
            token,
            vip_token,
            tracker_key,
            collect_time,
            Value::from(vip_type),
            &qualities,
        ),
    ));
    if let Some(album_audio_id) = album_audio_id {
        attempts.push((
            "字符串 album_audio_id",
            playback_request_body(
                hash,
                Some(Value::String(album_audio_id.to_string())),
                audio_id.map(Value::from),
                Value::String(user_id.to_string()),
                token,
                vip_token,
                tracker_key,
                collect_time,
                Value::from(vip_type),
                &qualities,
            ),
        ));
    } else {
        attempts.push((
            "省略 album_audio_id",
            playback_request_body(
                hash,
                None,
                audio_id.map(Value::from),
                Value::String(user_id.to_string()),
                token,
                vip_token,
                tracker_key,
                collect_time,
                Value::from(vip_type),
                &qualities,
            ),
        ));
    }
    if let Some(audio_id) = audio_id {
        attempts.push((
            "数值 audio_id",
            playback_request_body(
                hash,
                album_audio_id.map(Value::from),
                Some(Value::from(audio_id)),
                Value::String(user_id.to_string()),
                token,
                vip_token,
                tracker_key,
                collect_time,
                Value::from(vip_type),
                &qualities,
            ),
        ));
        if album_audio_id.is_none() {
            attempts.push((
                "audio_id 作为 album_audio_id",
                playback_request_body(
                    hash,
                    Some(Value::from(audio_id)),
                    Some(Value::from(audio_id)),
                    Value::String(user_id.to_string()),
                    token,
                    vip_token,
                    tracker_key,
                    collect_time,
                    Value::from(vip_type),
                    &qualities,
                ),
            ));
        }
    }

    if let Ok(user_id_number) = user_id.trim().parse::<u64>() {
        attempts.push((
            "userid 数字格式",
            playback_request_body(
                hash,
                album_audio_id.map(Value::from),
                audio_id.map(Value::from),
                Value::from(user_id_number),
                token,
                vip_token,
                tracker_key,
                collect_time,
                Value::from(vip_type),
                &qualities,
            ),
        ));
    }

    attempts
}

fn playback_request_body(
    hash: &str,
    album_audio_id: Option<Value>,
    audio_id: Option<Value>,
    user_id: Value,
    token: &str,
    vip_token: &str,
    tracker_key: &str,
    collect_time: u64,
    vip: Value,
    qualities: &[&str],
) -> Value {
    let mut resource = serde_json::Map::new();
    if let Some(album_audio_id) = album_audio_id {
        resource.insert("album_audio_id".to_string(), album_audio_id);
    }
    if let Some(audio_id) = audio_id.clone() {
        resource.insert("audio_id".to_string(), audio_id.clone());
        resource.insert("id".to_string(), audio_id);
    } else {
        resource.insert("id".to_string(), Value::from(0));
    }
    resource.insert(
        "collect_list_id".to_string(),
        Value::String("3".to_string()),
    );
    resource.insert("collect_time".to_string(), Value::from(collect_time));
    resource.insert("hash".to_string(), Value::String(hash.to_string()));
    resource.insert("page_id".to_string(), Value::from(1));
    resource.insert("type".to_string(), Value::String("audio".to_string()));

    json!({
        "area_code": "1",
        "behavior": "play",
        "qualities": qualities.iter().map(|quality| Value::String((*quality).to_string())).collect::<Vec<_>>(),
        "resource": Value::Object(resource),
        "token": token,
        "tracker_param": {
            "all_m": 1,
            "auth": "",
            "is_free_part": 0,
            "key": tracker_key,
            "module_id": 0,
            "need_climax": 1,
            "need_xcdn": 1,
            "open_time": "",
            "pid": "411",
            "pidversion": "3001",
            "priv_vip_type": "6",
            "viptoken": vip_token,
        },
        "userid": user_id,
        "vip": vip,
    })
}

#[derive(Default)]
struct KugouQualityAvailabilitySignals {
    available: BTreeSet<String>,
    unavailable: BTreeSet<String>,
    reasons: BTreeMap<String, String>,
    details: BTreeMap<String, String>,
}

fn get_kugou_privilege_lite(
    hashes: &[String],
    album_audio_id: Option<u64>,
    cookies: &mut BTreeMap<String, String>,
) -> Result<Value, String> {
    let appid = KUGOU_APPID.parse::<u64>().unwrap_or(3116);
    let clientver = KUGOU_CLIENTVER.parse::<u64>().unwrap_or(11440);
    let resource = hashes
        .iter()
        .map(|hash| {
            json!({
                "type": "audio",
                "page_id": 0,
                "hash": hash.to_ascii_lowercase(),
                "album_id": 0,
                "album_audio_id": album_audio_id.unwrap_or_default(),
            })
        })
        .collect::<Vec<_>>();
    let qualities = [
        "128",
        "320",
        "flac",
        "high",
        "viper_atmos",
        "viper_tape",
        "viper_clear",
        "viper_hifi",
        "super",
        "multitrack",
    ];
    let body = json!({
        "appid": appid,
        "area_code": 1,
        "behavior": "play",
        "clientver": clientver,
        "need_hash_offset": 1,
        "relate": 1,
        "support_verify": 1,
        "resource": resource,
        "qualities": qualities,
    });
    let response = kugou_android_post_json_with_client(
        KUGOU_GATEWAY_BASE,
        "/v2/get_res_privilege/lite",
        BTreeMap::new(),
        body,
        cookies,
        Some("media.store.kugou.com"),
        KUGOU_APPID,
        KUGOU_CLIENTVER,
        KUGOU_SIGNATURE_ANDROID_SECRET,
    )?;
    ensure_kugou_success(&response, "酷狗音质可用性预检")
        .map_err(|err| append_kugou_playback_diagnostic(err, &response))?;
    Ok(response)
}

fn build_kugou_quality_availability_items(
    signals: &KugouQualityAvailabilitySignals,
) -> Vec<KugouQualityAvailabilityItem> {
    [
        "viper_clear",
        "super",
        "viper_hifi",
        "viper_atmos",
        "hires",
        "flac",
        "high",
        "standard",
    ]
    .into_iter()
    .map(|quality| {
        let aliases = kugou_quality_availability_aliases(quality);
        let matched_available = aliases
            .iter()
            .find(|alias| signals.available.iter().any(|quality| quality == **alias))
            .copied();
        let matched_unavailable = aliases
            .iter()
            .find(|alias| signals.unavailable.iter().any(|quality| quality == **alias))
            .copied();
        let (status, reason, detail) = if let Some(alias) = matched_available {
            (
                "available".to_string(),
                None,
                signals.details.get(alias).cloned(),
            )
        } else if quality == "hires" {
            (
                "unavailable".to_string(),
                Some(
                    "当前酷狗可用性接口和会员接口都不能验证 Hi-Res 播放源，已禁用该档位，避免把低音质链接当作 Hi-Res。"
                        .to_string(),
                ),
                Some("Hi-Res 仍只能依赖登录态 /v5/url 返回授权播放源；当前预检没有发现可验证 Hi-Res 能力。".to_string()),
            )
        } else if let Some(alias) = matched_unavailable {
            (
                "unavailable".to_string(),
                Some(
                    signals
                        .reasons
                        .get(alias)
                        .cloned()
                        .unwrap_or_else(|| "酷狗接口返回该档位当前不可用。".to_string()),
                ),
                signals.details.get(alias).cloned(),
            )
        } else {
            (
                "unknown".to_string(),
                Some("酷狗接口没有明确返回该档位状态，播放时仍会按授权结果严格验证。".to_string()),
                None,
            )
        };

        KugouQualityAvailabilityItem {
            quality: quality.to_string(),
            label: kugou_quality_label(quality).to_string(),
            status,
            reason,
            detail,
        }
    })
    .collect()
}

fn collect_kugou_quality_availability_signals(response: &Value) -> KugouQualityAvailabilitySignals {
    let mut signals = KugouQualityAvailabilitySignals::default();
    visit_kugou_quality_availability_value(response, None, &mut signals);
    signals
}

fn visit_kugou_quality_availability_value(
    value: &Value,
    quality_context: Option<&str>,
    signals: &mut KugouQualityAvailabilitySignals,
) {
    match value {
        Value::Object(map) => {
            let object_quality = first_string(
                value,
                &[
                    "quality",
                    "quality_type",
                    "qualityType",
                    "quality_level",
                    "qualityLevel",
                    "level",
                    "level_name",
                    "levelName",
                    "audio_quality",
                    "audioQuality",
                    "audio_quality_name",
                    "audioQualityName",
                ],
            )
            .and_then(|quality| normalize_kugou_availability_quality(&quality));
            let context = object_quality.as_deref().or(quality_context);
            if let Some(quality) = context {
                if let Some((available, reason)) = quality_object_availability(value) {
                    record_kugou_quality_signal(signals, quality, available, reason, value);
                }
            }

            for (key, field) in map {
                if let Some(quality) = quality_from_availability_key(key) {
                    if let Some((available, reason)) =
                        quality_field_availability(key, field, quality.as_str())
                    {
                        record_kugou_quality_signal(signals, &quality, available, reason, field);
                    }
                    visit_kugou_quality_availability_value(field, Some(&quality), signals);
                } else {
                    visit_kugou_quality_availability_value(field, context, signals);
                }
            }
        }
        Value::Array(items) => {
            for item in items {
                visit_kugou_quality_availability_value(item, quality_context, signals);
            }
        }
        Value::String(text) => {
            if let Some(parsed) = parse_json_string_value(text) {
                visit_kugou_quality_availability_value(&parsed, quality_context, signals);
            }
        }
        _ => {}
    }
}

fn record_kugou_quality_signal(
    signals: &mut KugouQualityAvailabilitySignals,
    quality: &str,
    available: bool,
    reason: Option<String>,
    source: &Value,
) {
    let Some(quality) = normalize_kugou_availability_quality(quality) else {
        return;
    };
    if available {
        signals.available.insert(quality.clone());
        signals.unavailable.remove(&quality);
    } else if !signals.available.contains(&quality) {
        signals.unavailable.insert(quality.clone());
    }
    if let Some(reason) = reason.filter(|value| !value.trim().is_empty()) {
        signals.reasons.insert(quality.clone(), reason);
    }
    signals
        .details
        .entry(quality)
        .or_insert_with(|| summarize_diagnostic_value(source));
}

fn quality_object_availability(value: &Value) -> Option<(bool, Option<String>)> {
    if has_non_empty_field(
        value,
        &["hash", "file_hash", "fileHash", "hash_value", "hashValue"],
    ) || has_positive_field(
        value,
        &[
            "size",
            "fileSize",
            "filesize",
            "file_size",
            "bitrate",
            "bitRate",
        ],
    ) {
        return Some((true, None));
    }

    if has_positive_field(value, &["pay_block_tpl", "payBlockTpl", "pay_block"]) {
        return Some((false, Some("该档位需要会员或付费授权。".to_string())));
    }

    for key in [
        "status",
        "priv_status",
        "privStatus",
        "auth_status",
        "authStatus",
    ] {
        let Some(field) = field_value(value, key) else {
            continue;
        };
        if let Some(available) = availability_from_scalar(field) {
            return Some((
                available,
                if available {
                    None
                } else {
                    Some("酷狗接口返回该档位当前不可用。".to_string())
                },
            ));
        }
    }

    for key in ["privilege", "privilege2", "privilege_type", "privilegeType"] {
        let Some(field) = field_value(value, key) else {
            continue;
        };
        if let Some(value) = scalar_u64(field) {
            return Some((
                value > 0,
                if value > 0 {
                    None
                } else {
                    Some("酷狗接口返回该档位权限为 0。".to_string())
                },
            ));
        }
    }

    None
}

fn quality_field_availability(
    key: &str,
    value: &Value,
    quality: &str,
) -> Option<(bool, Option<String>)> {
    let key = key.to_ascii_lowercase();
    if key.contains("hash") {
        return Some((
            non_empty_scalar(value),
            if non_empty_scalar(value) {
                None
            } else {
                Some(format!(
                    "酷狗接口没有返回{}文件 hash。",
                    kugou_quality_label(quality)
                ))
            },
        ));
    }
    if key.contains("filesize")
        || key.contains("file_size")
        || key.ends_with("size")
        || key.contains("bitrate")
    {
        let available = scalar_u64(value).map(|number| number > 0).unwrap_or(false);
        return Some((
            available,
            if available {
                None
            } else {
                Some(format!(
                    "酷狗接口没有返回{}文件大小。",
                    kugou_quality_label(quality)
                ))
            },
        ));
    }
    if key.contains("privilege") || key.contains("status") {
        if let Some(available) = availability_from_scalar(value) {
            return Some((
                available,
                if available {
                    None
                } else {
                    Some(format!(
                        "酷狗接口返回{}当前不可用。",
                        kugou_quality_label(quality)
                    ))
                },
            ));
        }
    }

    None
}

fn quality_from_availability_key(key: &str) -> Option<String> {
    let lowered = key.to_ascii_lowercase();
    let compact = lowered
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .collect::<String>();
    let quality_key = [
        "viper_atmos",
        "viper_tape",
        "viper_clear",
        "viper_hifi",
        "multitrack",
        "hires",
        "hi-res",
        "lossless",
        "flac",
        "super",
        "high",
        "320",
        "128",
        "standard",
    ]
    .into_iter()
    .find(|quality| {
        let token = quality
            .chars()
            .filter(|ch| ch.is_ascii_alphanumeric())
            .collect::<String>();
        compact.contains(&token)
    })?;
    normalize_kugou_availability_quality(quality_key)
}

fn normalize_kugou_availability_quality(value: &str) -> Option<String> {
    normalize_kugou_returned_quality(value).map(|quality| match quality.as_str() {
        "lossless" => "flac".to_string(),
        "exhigh" | "320" => "320".to_string(),
        "standard" | "128" => "128".to_string(),
        other => other.to_string(),
    })
}

fn kugou_quality_availability_aliases(quality: &str) -> Vec<&'static str> {
    match quality {
        "standard" => vec!["128", "standard"],
        "high" => vec!["high", "320", "exhigh"],
        "flac" | "lossless" => vec!["flac", "lossless"],
        "hires" => vec!["hires"],
        "super" => vec!["super"],
        "viper_clear" => vec!["viper_clear"],
        "viper_hifi" => vec!["viper_hifi"],
        "viper_tape" => vec!["viper_tape"],
        "viper_atmos" => vec!["viper_atmos"],
        "multitrack" => vec!["multitrack"],
        _ => Vec::new(),
    }
}

fn availability_from_scalar(value: &Value) -> Option<bool> {
    match value {
        Value::Bool(value) => Some(*value),
        Value::Number(number) => number.as_i64().map(|number| number > 0),
        Value::String(text) => {
            let text = text.trim().to_ascii_lowercase();
            if text.is_empty() {
                return Some(false);
            }
            if [
                "0",
                "false",
                "no",
                "deny",
                "denied",
                "block",
                "blocked",
                "unavailable",
            ]
            .contains(&text.as_str())
            {
                return Some(false);
            }
            if ["1", "true", "yes", "allow", "allowed", "available", "ok"].contains(&text.as_str())
            {
                return Some(true);
            }
            text.parse::<i64>().ok().map(|number| number > 0)
        }
        _ => None,
    }
}

fn scalar_u64(value: &Value) -> Option<u64> {
    match value {
        Value::Number(number) => number.as_u64(),
        Value::String(text) => text.trim().parse::<u64>().ok(),
        _ => None,
    }
}

fn non_empty_scalar(value: &Value) -> bool {
    match value {
        Value::String(text) => !text.trim().is_empty(),
        Value::Number(number) => number.as_u64().map(|value| value > 0).unwrap_or(true),
        Value::Bool(value) => *value,
        Value::Array(items) => !items.is_empty(),
        Value::Object(map) => !map.is_empty(),
        Value::Null => false,
    }
}

fn has_non_empty_field(value: &Value, keys: &[&str]) -> bool {
    keys.iter().any(|key| {
        field_value(value, key)
            .map(non_empty_scalar)
            .unwrap_or(false)
    })
}

fn has_positive_field(value: &Value, keys: &[&str]) -> bool {
    keys.iter().any(|key| {
        field_value(value, key)
            .and_then(scalar_u64)
            .map(|number| number > 0)
            .unwrap_or(false)
    })
}

fn kugou_quality_availability_diagnostic(
    response: &Value,
    device_warning: Option<&str>,
) -> Option<String> {
    let mut parts = vec![describe_response_shape(response)];
    if let Some(device_warning) = device_warning {
        parts.push(format!(
            "设备注册提示={}",
            sanitize_diagnostic_text(device_warning)
        ));
    }
    if let Some(message) = playback_failure_reason(response) {
        parts.push(format!("message={}", sanitize_diagnostic_text(&message)));
    }
    Some(parts.join("；"))
}

fn refresh_kugou_login_token(credential: &mut KugouCredential) -> Result<(), String> {
    let token = credential
        .cookies
        .get("token")
        .cloned()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| "酷狗登录刷新缺少 token，请重新扫码登录。".to_string())?;
    let user_id = credential
        .cookies
        .get("userid")
        .cloned()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| "酷狗登录刷新缺少用户 ID，请重新扫码登录。".to_string())?;
    let now_ms = current_epoch_millis() as u64;
    let p3 = aes256_cbc_encrypt_hex_value(
        &json!({
            "clienttime": now_ms / 1000,
            "token": token,
        }),
        KUGOU_LOGIN_TOKEN_LITE_KEY,
        KUGOU_LOGIN_TOKEN_LITE_IV,
    )?;
    let (encrypted_params, encrypted_params_key) =
        aes256_cbc_encrypt_hex_with_temp_key(&json!({}))?;
    let pk = rsa_raw_encrypt_lite_hex(
        &serde_json::to_string(&json!({
            "clienttime_ms": now_ms,
            "key": encrypted_params_key,
        }))
        .map_err(|err| format!("序列化酷狗登录刷新 pk 失败：{err}"))?,
    )?;
    let guid = credential
        .cookies
        .get("KUGOU_API_GUID")
        .cloned()
        .unwrap_or_else(pseudo_uuid);
    let mac = credential
        .cookies
        .get("KUGOU_API_MAC")
        .cloned()
        .unwrap_or_else(|| pseudo_alnum(12));
    let dev = credential
        .cookies
        .get("KUGOU_API_DEV")
        .cloned()
        .unwrap_or_else(|| pseudo_alnum(16));
    credential
        .cookies
        .entry("KUGOU_API_GUID".to_string())
        .or_insert(guid.clone());
    credential
        .cookies
        .entry("KUGOU_API_MAC".to_string())
        .or_insert(mac.clone());
    credential
        .cookies
        .entry("KUGOU_API_DEV".to_string())
        .or_insert(dev.clone());
    let t2 = aes256_cbc_encrypt_hex_text(
        &format!("{guid}|0f607264fc6318a92b9e13c65db7cd3c|{mac}|{dev}|{now_ms}"),
        KUGOU_LOGIN_TOKEN_LITE_T2_KEY,
        KUGOU_LOGIN_TOKEN_LITE_T2_IV,
    )?;
    let existing_t1 = credential.cookies.get("t1").cloned().unwrap_or_default();
    let t1 = aes256_cbc_encrypt_hex_text(
        &format!("{existing_t1}|{now_ms}"),
        KUGOU_LOGIN_TOKEN_LITE_T1_KEY,
        KUGOU_LOGIN_TOKEN_LITE_T1_IV,
    )?;
    let body = json!({
        "dfid": first_cookie_value(&credential.cookies, &["dfid", "DFID"]).unwrap_or_else(|| "-".to_string()),
        "p3": p3,
        "plat": 1,
        "t1": t1,
        "t2": t2,
        "t3": "MCwwLDAsMCwwLDAsMCwwLDA=",
        "pk": pk,
        "params": encrypted_params,
        "userid": user_id,
        "clienttime_ms": now_ms,
        "dev": dev,
    });
    let response = kugou_android_post_json(
        KUGOU_LOGIN_TOKEN_BASE,
        "/v5/login_by_token",
        BTreeMap::new(),
        body,
        &mut credential.cookies,
        None,
    )?;
    ensure_kugou_success(&response, "酷狗登录 token 刷新")?;
    let data = response
        .get("data")
        .ok_or_else(|| "酷狗登录 token 刷新响应缺少 data 字段。".to_string())?;
    if let Some(secu_params) =
        value_as_string(data, "secu_params").filter(|value| !value.trim().is_empty())
    {
        if let Some(decoded) =
            aes256_cbc_decrypt_hex_with_temp_key(&secu_params, &encrypted_params_key)?
        {
            if let Some(token) = decoded.as_str().filter(|value| !value.trim().is_empty()) {
                credential
                    .cookies
                    .insert("token".to_string(), token.to_string());
            } else {
                merge_login_token_fields(credential, &decoded);
            }
        }
    }
    merge_login_token_fields(credential, data);

    Ok(())
}

fn ensure_kugou_device_registered(
    credential: &mut KugouCredential,
    user_id: &str,
    token: &str,
) -> Result<(), String> {
    ensure_kugou_device_registered_cookies(&mut credential.cookies, user_id, token)
}

fn ensure_kugou_device_registered_cookies(
    cookies: &mut BTreeMap<String, String>,
    user_id: &str,
    token: &str,
) -> Result<(), String> {
    ensure_kugou_device_cookie_defaults(cookies);
    let has_dfid = first_cookie_value(cookies, &["dfid", "DFID"])
        .filter(|value| value != "-")
        .is_some();
    let registered_at = cookies
        .get(KUGOU_DEVICE_REGISTER_MARKER)
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or_default();
    let now_ms = current_epoch_millis() as u64;
    if has_dfid
        && registered_at > 0
        && now_ms.saturating_sub(registered_at) < KUGOU_DEVICE_REGISTER_TTL_MS
    {
        return Ok(());
    }

    match register_kugou_device(cookies, user_id, token) {
        Ok(dfid) => {
            cookies.insert("dfid".to_string(), dfid);
            cookies.insert(KUGOU_DEVICE_REGISTER_MARKER.to_string(), now_ms.to_string());
            Ok(())
        }
        Err(err) => Err(format!("酷狗设备注册失败：{err}")),
    }
}

fn ensure_kugou_device_cookie_defaults(cookies: &mut BTreeMap<String, String>) {
    let guid = first_cookie_value(cookies, &["KUGOU_API_GUID"]).unwrap_or_else(|| {
        let guid = pseudo_uuid();
        cookies.insert("KUGOU_API_GUID".to_string(), guid.clone());
        guid
    });
    cookies
        .entry("KUGOU_API_MID".to_string())
        .or_insert_with(|| calculate_mid(&guid));
    cookies
        .entry("KUGOU_API_MAC".to_string())
        .or_insert_with(|| "02:00:00:00:00:00".to_string());
    cookies
        .entry("KUGOU_API_DEV".to_string())
        .or_insert_with(|| pseudo_alnum(10));
}

fn register_kugou_device(
    cookies: &mut BTreeMap<String, String>,
    user_id: &str,
    token: &str,
) -> Result<String, String> {
    let guid = first_cookie_value(cookies, &["KUGOU_API_GUID"]).unwrap_or_else(pseudo_uuid);
    let aes_key = pseudo_alnum(6).to_lowercase();
    let payload = json!({
        "availableRamSize": 4983533568_u64,
        "availableRomSize": 48114719_u64,
        "availableSDSize": 48114717_u64,
        "basebandVer": "",
        "batteryLevel": 100,
        "batteryStatus": 3,
        "brand": "Redmi",
        "buildSerial": "unknown",
        "device": "marble",
        "imei": guid,
        "imsi": "",
        "manufacturer": "Xiaomi",
        "uuid": first_cookie_value(cookies, &["KUGOU_API_GUID"]).unwrap_or_else(pseudo_uuid),
        "accelerometer": false,
        "accelerometerValue": "",
        "gravity": false,
        "gravityValue": "",
        "gyroscope": false,
        "gyroscopeValue": "",
        "light": false,
        "lightValue": "",
        "magnetic": false,
        "magneticValue": "",
        "orientation": false,
        "orientationValue": "",
        "pressure": false,
        "pressureValue": "",
        "step_counter": false,
        "step_counterValue": "",
        "temperature": false,
        "temperatureValue": "",
    });
    let encrypted_body = playlist_aes_encrypt_base64_value(&payload, &aes_key)?;
    let rsa_payload = serde_json::to_string(&json!({
        "aes": aes_key,
        "uid": user_id,
        "token": token,
    }))
    .map_err(|err| format!("序列化酷狗设备注册参数失败：{err}"))?;
    let p = rsa_pkcs1_encrypt_lite_hex(&rsa_payload)?;

    let mut params = BTreeMap::new();
    params.insert("part".to_string(), "1".to_string());
    params.insert("platid".to_string(), "1".to_string());
    params.insert("p".to_string(), p);

    let response = kugou_android_post_text_encrypted_json(
        KUGOU_USER_SERVICE_BASE,
        "/risk/v2/r_register_dev",
        params,
        &encrypted_body,
        cookies,
        &aes_key,
        KUGOU_APPID,
        KUGOU_CLIENTVER,
        KUGOU_SIGNATURE_ANDROID_SECRET,
    )?;
    ensure_kugou_success(&response, "酷狗设备注册")?;
    response
        .get("data")
        .and_then(|data| value_as_string(data, "dfid"))
        .or_else(|| value_as_string(&response, "dfid"))
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| "酷狗设备注册响应缺少 dfid。".to_string())
}

fn merge_login_token_fields(credential: &mut KugouCredential, data: &Value) {
    for (cookie_key, response_keys) in [
        ("t1", &["t1"][..]),
        ("token", &["token"][..]),
        ("userid", &["userid", "user_id"][..]),
        ("vip_type", &["vip_type", "vipType", "viptype"][..]),
        ("vip_token", &["vip_token", "vipToken", "viptoken"][..]),
    ] {
        if let Some(value) = first_string(data, response_keys).filter(|value| !value.is_empty()) {
            credential.cookies.insert(cookie_key.to_string(), value);
        }
    }
}

fn kugou_signed_get_json(
    base_url: &str,
    path: &str,
    params: BTreeMap<String, String>,
    cookies: &mut BTreeMap<String, String>,
) -> Result<Value, String> {
    let mut final_params = BTreeMap::new();
    final_params.insert(
        "dfid".to_string(),
        cookies
            .get("dfid")
            .cloned()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| "-".to_string()),
    );
    final_params.insert(
        "mid".to_string(),
        cookies
            .get("KUGOU_API_MID")
            .cloned()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| "-".to_string()),
    );
    final_params.insert("uuid".to_string(), "-".to_string());
    final_params.insert("appid".to_string(), KUGOU_APPID.to_string());
    final_params.insert("clientver".to_string(), KUGOU_CLIENTVER.to_string());
    final_params.insert(
        "clienttime".to_string(),
        current_epoch_seconds().to_string(),
    );
    if let Some(token) = cookies
        .get("token")
        .filter(|value| !value.trim().is_empty())
    {
        final_params.insert("token".to_string(), token.clone());
    }
    if let Some(user_id) = cookies
        .get("userid")
        .filter(|value| !value.trim().is_empty())
    {
        final_params.insert("userid".to_string(), user_id.clone());
    }
    for (key, value) in params {
        final_params.insert(key, value);
    }
    final_params.insert("signature".to_string(), signature_web(&final_params));

    let query = final_params
        .iter()
        .map(|(key, value)| {
            format!(
                "{}={}",
                url_encode_component(key),
                url_encode_component(value)
            )
        })
        .collect::<Vec<_>>()
        .join("&");
    let url = format!("{base_url}{path}?{query}");
    let client = Client::builder()
        .user_agent(KUGOU_USER_AGENT)
        .timeout(Duration::from_secs(18))
        .build()
        .map_err(|err| format!("创建酷狗登录请求客户端失败：{err}"))?;
    let response = client
        .get(url)
        .header(REFERER, "https://www.kugou.com/")
        .send()
        .map_err(|err| format!("请求酷狗登录接口失败：{err}"))?;
    let status = response.status();
    merge_set_cookies(cookies, &collect_set_cookies(response.headers()));
    let body: Value = response
        .json()
        .map_err(|err| format!("解析酷狗登录响应失败：{err}"))?;
    if !status.is_success() {
        return Err(format!("酷狗登录接口返回 HTTP {status}"));
    }

    let api_status = value_as_i64(&body, "status").unwrap_or(1);
    let error_code = value_as_i64(&body, "error_code").unwrap_or_default();
    if api_status != 1 && error_code != 0 {
        let message = value_as_string(&body, "error")
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| "酷狗登录接口返回异常".to_string());
        return Err(format!("{message}（code {error_code}）"));
    }

    Ok(body)
}

fn kugou_android_post_json(
    base_url: &str,
    path: &str,
    params: BTreeMap<String, String>,
    data: Value,
    cookies: &mut BTreeMap<String, String>,
    router: Option<&str>,
) -> Result<Value, String> {
    kugou_android_post_json_with_client(
        base_url,
        path,
        params,
        data,
        cookies,
        router,
        KUGOU_APPID,
        KUGOU_CLIENTVER,
        KUGOU_SIGNATURE_ANDROID_SECRET,
    )
}

fn kugou_android_post_json_with_client(
    base_url: &str,
    path: &str,
    params: BTreeMap<String, String>,
    data: Value,
    cookies: &mut BTreeMap<String, String>,
    router: Option<&str>,
    appid: &str,
    clientver: &str,
    signature_secret: &str,
) -> Result<Value, String> {
    let mut final_params = BTreeMap::new();
    final_params.insert(
        "dfid".to_string(),
        cookies
            .get("dfid")
            .cloned()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| "-".to_string()),
    );
    final_params.insert(
        "mid".to_string(),
        cookies
            .get("KUGOU_API_MID")
            .cloned()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| "-".to_string()),
    );
    final_params.insert("uuid".to_string(), "-".to_string());
    final_params.insert("appid".to_string(), appid.to_string());
    final_params.insert("clientver".to_string(), clientver.to_string());
    final_params.insert(
        "clienttime".to_string(),
        current_epoch_seconds().to_string(),
    );
    if let Some(token) = cookies
        .get("token")
        .filter(|value| !value.trim().is_empty())
    {
        final_params.insert("token".to_string(), token.clone());
    }
    if let Some(user_id) = cookies
        .get("userid")
        .filter(|value| !value.trim().is_empty())
    {
        final_params.insert("userid".to_string(), user_id.clone());
    }
    for (key, value) in params {
        final_params.insert(key, value);
    }

    let data_text = serde_json::to_string(&data).map_err(|err| err.to_string())?;
    final_params.insert(
        "signature".to_string(),
        signature_android_with_secret(&final_params, &data_text, signature_secret),
    );
    let query = final_params
        .iter()
        .map(|(key, value)| {
            format!(
                "{}={}",
                url_encode_component(key),
                url_encode_component(value)
            )
        })
        .collect::<Vec<_>>()
        .join("&");
    let url = format!("{base_url}{path}?{query}");
    let client = Client::builder()
        .timeout(Duration::from_secs(20))
        .user_agent("Android15-1070-11083-46-0-DiscoveryDRADProtocol-wifi")
        .build()
        .map_err(|err| format!("创建酷狗请求客户端失败：{err}"))?;
    let mut request = client
        .post(url)
        .header(
            "dfid",
            final_params.get("dfid").cloned().unwrap_or_default(),
        )
        .header(
            "clienttime",
            final_params.get("clienttime").cloned().unwrap_or_default(),
        )
        .header("mid", final_params.get("mid").cloned().unwrap_or_default())
        .header("kg-rc", "1")
        .header("kg-thash", "5d816a0")
        .header("kg-rec", "1")
        .header("kg-rf", "B9EDA08A64250DEFFBCADDEE00F8F25F")
        .header(CONTENT_TYPE, "application/json")
        .body(data_text);
    if let Some(router) = router {
        request = request.header("x-router", router);
    }

    let response = request
        .send()
        .map_err(|err| format!("请求酷狗接口失败：{err}"))?;
    let status = response.status();
    merge_set_cookies(cookies, &collect_set_cookies(response.headers()));
    let body: Value = response
        .json()
        .map_err(|err| format!("解析酷狗接口响应失败：{err}"))?;
    if !status.is_success() {
        return Err(format!("酷狗接口返回 HTTP {status}"));
    }

    Ok(body)
}

fn kugou_android_get_json_with_client(
    base_url: &str,
    path: &str,
    params: BTreeMap<String, String>,
    credential: &mut KugouCredential,
    router: Option<&str>,
    key_hash: Option<&str>,
    appid: &str,
    clientver: &str,
    signature_secret: &str,
) -> Result<Value, String> {
    let cookies = &mut credential.cookies;
    let mut final_params = kugou_android_base_params(cookies, appid, clientver);
    for (key, value) in params {
        final_params.insert(key, value);
    }
    if let Some(hash) = key_hash {
        let mid = final_params.get("mid").cloned().unwrap_or_default();
        let user_id = final_params
            .get("userid")
            .cloned()
            .unwrap_or_else(|| "0".to_string());
        final_params.insert(
            "key".to_string(),
            playback_tracker_key(hash, &mid, &user_id),
        );
    }
    final_params.insert(
        "signature".to_string(),
        signature_android_with_secret(&final_params, "", signature_secret),
    );
    let query = final_params
        .iter()
        .map(|(key, value)| {
            format!(
                "{}={}",
                url_encode_component(key),
                url_encode_component(value)
            )
        })
        .collect::<Vec<_>>()
        .join("&");
    let url = format!("{base_url}{path}?{query}");
    let client = Client::builder()
        .timeout(Duration::from_secs(20))
        .user_agent("Android15-1070-11083-46-0-DiscoveryDRADProtocol-wifi")
        .build()
        .map_err(|err| format!("创建酷狗请求客户端失败：{err}"))?;
    let mut request = client
        .get(url)
        .header(
            "dfid",
            final_params.get("dfid").cloned().unwrap_or_default(),
        )
        .header(
            "clienttime",
            final_params.get("clienttime").cloned().unwrap_or_default(),
        )
        .header("mid", final_params.get("mid").cloned().unwrap_or_default())
        .header("kg-rc", "1")
        .header("kg-thash", "5d816a0")
        .header("kg-rec", "1")
        .header("kg-rf", "B9EDA08A64250DEFFBCADDEE00F8F25F");
    if let Some(router) = router {
        request = request.header("x-router", router);
    }

    let response = request
        .send()
        .map_err(|err| format!("请求酷狗接口失败：{err}"))?;
    let status = response.status();
    merge_set_cookies(cookies, &collect_set_cookies(response.headers()));
    let body: Value = response
        .json()
        .map_err(|err| format!("解析酷狗接口响应失败：{err}"))?;
    if !status.is_success() {
        return Err(format!("酷狗接口返回 HTTP {status}"));
    }

    Ok(body)
}

fn kugou_android_post_text_encrypted_json(
    base_url: &str,
    path: &str,
    params: BTreeMap<String, String>,
    data_text: &str,
    cookies: &mut BTreeMap<String, String>,
    decrypt_key: &str,
    appid: &str,
    clientver: &str,
    signature_secret: &str,
) -> Result<Value, String> {
    let mut final_params = kugou_android_base_params(cookies, appid, clientver);
    for (key, value) in params {
        final_params.insert(key, value);
    }
    final_params.insert(
        "signature".to_string(),
        signature_android_with_secret(&final_params, data_text, signature_secret),
    );
    let query = final_params
        .iter()
        .map(|(key, value)| {
            format!(
                "{}={}",
                url_encode_component(key),
                url_encode_component(value)
            )
        })
        .collect::<Vec<_>>()
        .join("&");
    let url = format!("{base_url}{path}?{query}");
    let client = Client::builder()
        .timeout(Duration::from_secs(20))
        .user_agent("Android15-1070-11083-46-0-DiscoveryDRADProtocol-wifi")
        .build()
        .map_err(|err| format!("创建酷狗请求客户端失败：{err}"))?;
    let response = client
        .post(url)
        .header(
            "dfid",
            final_params.get("dfid").cloned().unwrap_or_default(),
        )
        .header(
            "clienttime",
            final_params.get("clienttime").cloned().unwrap_or_default(),
        )
        .header("mid", final_params.get("mid").cloned().unwrap_or_default())
        .header("kg-rc", "1")
        .header("kg-thash", "5d816a0")
        .header("kg-rec", "1")
        .header("kg-rf", "B9EDA08A64250DEFFBCADDEE00F8F25F")
        .body(data_text.to_string())
        .send()
        .map_err(|err| format!("请求酷狗设备注册接口失败：{err}"))?;
    let status = response.status();
    merge_set_cookies(cookies, &collect_set_cookies(response.headers()));
    let bytes = response
        .bytes()
        .map_err(|err| format!("读取酷狗设备注册响应失败：{err}"))?;
    if !status.is_success() {
        return Err(format!("酷狗设备注册接口返回 HTTP {status}"));
    }

    playlist_aes_decrypt_bytes(&bytes, decrypt_key).or_else(|decrypt_err| {
        let text = String::from_utf8_lossy(&bytes).to_string();
        serde_json::from_str::<Value>(&text)
            .map_err(|_| format!("解密酷狗设备注册响应失败：{decrypt_err}；响应不是可解析 JSON"))
    })
}

fn kugou_android_base_params(
    cookies: &BTreeMap<String, String>,
    appid: &str,
    clientver: &str,
) -> BTreeMap<String, String> {
    let mut final_params = BTreeMap::new();
    final_params.insert(
        "dfid".to_string(),
        cookies
            .get("dfid")
            .cloned()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| "-".to_string()),
    );
    final_params.insert(
        "mid".to_string(),
        cookies
            .get("KUGOU_API_MID")
            .cloned()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| "-".to_string()),
    );
    final_params.insert("uuid".to_string(), "-".to_string());
    final_params.insert("appid".to_string(), appid.to_string());
    final_params.insert("clientver".to_string(), clientver.to_string());
    final_params.insert(
        "clienttime".to_string(),
        current_epoch_seconds().to_string(),
    );
    if let Some(token) = cookies
        .get("token")
        .filter(|value| !value.trim().is_empty())
    {
        final_params.insert("token".to_string(), token.clone());
    }
    if let Some(user_id) = cookies
        .get("userid")
        .filter(|value| !value.trim().is_empty())
    {
        final_params.insert("userid".to_string(), user_id.clone());
    }
    final_params
}

fn require_logged_in_credential(app: &AppHandle) -> Result<KugouCredential, String> {
    let Some(credential) = read_credential(app)? else {
        return Err("请先登录酷狗音乐。".to_string());
    };
    if credential
        .cookies
        .get("token")
        .filter(|value| !value.trim().is_empty())
        .is_none()
        || credential
            .cookies
            .get("userid")
            .filter(|value| !value.trim().is_empty())
            .is_none()
    {
        return Err("酷狗登录状态不可用，请重新扫码登录。".to_string());
    }

    Ok(credential)
}

fn ensure_kugou_success(body: &Value, context: &str) -> Result<(), String> {
    let status = value_as_i64(body, "status").unwrap_or(1);
    let error_code = value_as_i64(body, "error_code").unwrap_or_default();
    let code = value_as_i64(body, "code").unwrap_or(200);
    if (status == 1 || status == 200) && error_code == 0 && (code == 0 || code == 1 || code == 200)
    {
        return Ok(());
    }

    let message = value_as_string(body, "error")
        .or_else(|| value_as_string(body, "errmsg"))
        .or_else(|| value_as_string(body, "message"))
        .or_else(|| value_as_string(body, "msg"))
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "酷狗接口返回异常".to_string());
    Err(format!(
        "{context}失败：{message}（status {status}，code {code}，error_code {error_code}）"
    ))
}

fn is_kugou_unmarshal_error(body: &Value) -> bool {
    let error_code = value_as_i64(body, "error_code").unwrap_or_default();
    let message = playback_failure_reason(body)
        .unwrap_or_default()
        .to_ascii_lowercase();
    error_code == 20010
        || message.contains("unmarshal")
        || (message.contains("param") && message.contains("error"))
}

fn is_kugou_token_api_error(body: &Value) -> bool {
    let error_code = value_as_i64(body, "error_code").unwrap_or_default();
    let message = playback_failure_reason(body)
        .unwrap_or_default()
        .to_ascii_lowercase();
    error_code == 20018 || message.contains("token api error")
}

fn first_cookie_value(cookies: &BTreeMap<String, String>, keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|key| {
        cookies
            .iter()
            .find(|(current_key, _)| current_key.eq_ignore_ascii_case(key))
            .map(|(_, value)| value.trim().to_string())
            .filter(|value| !value.is_empty())
    })
}

fn playback_tracker_key(hash: &str, mid: &str, user_id: &str) -> String {
    let mut hasher = Md5::new();
    hasher
        .update(format!("{hash}{KUGOU_TRACKER_KEY_SECRET}{KUGOU_APPID}{mid}{user_id}").as_bytes());
    format!("{:x}", hasher.finalize())
}

fn find_play_url(value: &Value) -> Option<String> {
    match value {
        Value::Object(_) => {
            for key in [
                "url",
                "play_url",
                "playUrl",
                "priv_url",
                "privUrl",
                "purl",
                "cdn",
                "cdn_url",
                "cdnUrl",
                "backup_url",
                "backupUrl",
                "audio_url",
                "audioUrl",
                "file_url",
                "fileUrl",
            ] {
                if let Some(url) = field_value(value, key).and_then(find_play_url) {
                    return Some(url);
                }
            }
            value.as_object()?.values().find_map(find_play_url)
        }
        Value::Array(items) => items.iter().find_map(find_play_url),
        Value::String(text) => normalize_play_url(text)
            .or_else(|| parse_json_string_value(text).and_then(|value| find_play_url(&value))),
        _ => None,
    }
}

fn parse_json_string_value(value: &str) -> Option<Value> {
    let value = value.trim().replace("\\/", "/");
    if !(value.starts_with('{') || value.starts_with('[')) {
        return None;
    }

    serde_json::from_str(&value).ok()
}

fn normalize_play_url(value: &str) -> Option<String> {
    let value = value.trim().replace("\\/", "/");
    if !(value.starts_with("http://") || value.starts_with("https://")) {
        return None;
    }
    let lowered = value.to_ascii_lowercase();
    if [".jpg", ".jpeg", ".png", ".webp", ".gif", ".svg"]
        .iter()
        .any(|suffix| lowered.contains(suffix))
    {
        return None;
    }

    Some(value)
}

fn normalize_kugou_quality_preference(value: Option<&str>) -> String {
    let raw = value.unwrap_or("highest").trim().to_lowercase();
    match raw.as_str() {
        "standard" | "128" | "128k" | "std" => "standard".to_string(),
        "high" | "exhigh" | "320" | "320k" | "hq" => "high".to_string(),
        "lossless" | "flac" | "sq" => "lossless".to_string(),
        "hifi" | "viper_hifi" | "viper_hq" => "viper_hifi".to_string(),
        "hires" | "hi-res" => "hires".to_string(),
        "jyeffect" | "sky" => "viper_clear".to_string(),
        "super" | "viper_clear" | "viper_tape" | "viper_atmos" | "multitrack" | "highest" => raw,
        _ => "highest".to_string(),
    }
}

fn kugou_gateway_quality_candidates(preference: &str) -> Vec<&'static str> {
    match preference {
        "standard" => vec!["standard", "128"],
        "high" => vec!["high", "320"],
        "lossless" => vec!["flac"],
        "super" => vec!["super"],
        "viper_hifi" => vec!["viper_hifi"],
        "hires" => vec!["hires"],
        "viper_clear" => vec!["viper_clear"],
        "viper_tape" => vec!["viper_tape"],
        "viper_atmos" => vec!["viper_atmos"],
        "multitrack" => vec!["multitrack"],
        _ => vec![
            "viper_clear",
            "super",
            "viper_hifi",
            "viper_tape",
            "viper_atmos",
            "multitrack",
            "hires",
            "flac",
            "high",
            "320",
            "128",
        ],
    }
}

fn kugou_priv_url_quality_candidates(preference: &str) -> Vec<&'static str> {
    match preference {
        "standard" => vec!["128"],
        "high" => vec!["high", "320"],
        "lossless" => vec!["flac"],
        "super" => vec!["super"],
        "viper_hifi" => vec!["viper_hifi"],
        "hires" => Vec::new(),
        "viper_clear" => vec!["viper_clear"],
        "viper_tape" => vec!["viper_tape"],
        "viper_atmos" => vec!["viper_atmos"],
        "multitrack" => vec!["multitrack"],
        _ => vec![
            "viper_clear",
            "super",
            "viper_hifi",
            "viper_tape",
            "viper_atmos",
            "multitrack",
            "flac",
            "high",
            "320",
            "128",
        ],
    }
}

fn kugou_priv_url_unsupported_quality_message(preference: &str) -> String {
    format!(
        "酷狗会员 /v6/priv_url 不接受{}参数；已跳过会员接口该音质请求。当前该音质只能依赖登录态 /v5/url 返回授权播放源。",
        kugou_quality_label(preference)
    )
}

fn kugou_hires_playback_failure_message(logged_errors: &[String]) -> String {
    if logged_errors.is_empty() {
        return "酷狗 Hi-Res 音质只能通过登录态 /v5/url 获取授权播放源；当前没有可用酷狗登录态或登录态缺少用户 ID/token，请重新登录酷狗后再试。项目不会用普通旧接口或会员 /v6/priv_url 的低音质链接冒充 Hi-Res。"
            .to_string();
    }

    format!(
        "酷狗 Hi-Res 播放源获取失败：{}。会员 /v6/priv_url 不接受 Hi-Res 参数，普通旧接口也不能验证 Hi-Res 播放源；项目已停止继续尝试，避免把低音质链接当作 Hi-Res 成功。",
        summarize_playback_attempt_errors("登录态 /v5/url", logged_errors)
    )
}

fn kugou_quality_label(quality: &str) -> &'static str {
    match quality {
        "super" => "蝰蛇超清音质",
        "viper_clear" => "蝰蛇母带音质",
        "viper_hifi" => "蝰蛇HIFI音质",
        "viper_tape" => "蝰蛇磁带",
        "viper_atmos" => "蝰蛇全景声2.0",
        "multitrack" => "多轨",
        "hires" => "Hi-Res音质",
        "lossless" => "无损音质",
        "flac" => "无损音质",
        "high" => "高品音质",
        "exhigh" => "极高",
        "320" => "320k",
        "128" | "standard" => "标准音质",
        _ => "平台返回音质",
    }
}

fn normalize_kugou_bitrate(value: Option<u64>) -> Option<u64> {
    value.filter(|number| *number >= 64)
}

fn normalize_kugou_returned_quality(value: &str) -> Option<String> {
    let raw = value.trim().to_lowercase();
    if raw.is_empty() {
        return None;
    }

    match raw.as_str() {
        "super" | "viper_clear" | "viper_hifi" | "viper_tape" | "viper_atmos" | "multitrack"
        | "hires" | "lossless" | "flac" | "high" | "exhigh" | "320" | "128" | "standard" => {
            Some(raw)
        }
        "sq" => Some("flac".to_string()),
        "hq" | "320k" => Some("320".to_string()),
        "128k" | "std" => Some("128".to_string()),
        text if text.contains("viper") && text.contains("atmos") => Some("viper_atmos".to_string()),
        text if text.contains("viper") && text.contains("hifi") => Some("viper_hifi".to_string()),
        text if text.contains("viper") && text.contains("tape") => Some("viper_tape".to_string()),
        text if text.contains("viper") || text.contains("master") => {
            Some("viper_clear".to_string())
        }
        text if text.contains("全景声") => Some("viper_atmos".to_string()),
        text if text.contains("母带") => Some("viper_clear".to_string()),
        text if text.contains("hifi") => Some("viper_hifi".to_string()),
        text if text.contains("超清") => Some("super".to_string()),
        text if text.contains("无损") => Some("flac".to_string()),
        text if text.contains("高品") => Some("high".to_string()),
        text if text.contains("标准") => Some("128".to_string()),
        text if text.contains("hires") || text.contains("hi-res") => Some("hires".to_string()),
        text if text.contains("flac") || text.contains("lossless") => Some("flac".to_string()),
        text if text.contains("320") => Some("320".to_string()),
        text if text.contains("128") => Some("128".to_string()),
        _ => None,
    }
}

fn infer_kugou_quality_from_media(file_type: Option<&str>, bitrate: Option<u64>) -> Option<String> {
    let file_type = file_type.unwrap_or_default().trim().to_lowercase();
    if file_type.contains("flac") || file_type.contains("ape") || file_type.contains("wav") {
        return Some("flac".to_string());
    }

    let kbps = bitrate.map(|value| if value >= 1000 { value / 1000 } else { value });
    match kbps {
        Some(value) if value >= 320 => Some("320".to_string()),
        Some(value) if value >= 128 => Some("128".to_string()),
        _ => None,
    }
}

fn observed_kugou_quality_level(
    response: &Value,
    bitrate: Option<u64>,
    file_type: Option<&str>,
) -> Option<String> {
    find_string_by_keys(
        response,
        &[
            "quality_level",
            "qualityLevel",
            "quality_name",
            "qualityName",
            "audio_quality_name",
            "audioQualityName",
            "audio_quality",
            "audioQuality",
            "quality_type",
            "qualityType",
            "level",
            "level_name",
            "levelName",
            "quality",
        ],
    )
    .and_then(|value| normalize_kugou_returned_quality(&value))
    .or_else(|| infer_kugou_quality_from_media(file_type, bitrate))
}

fn kugou_observed_quality_satisfies_preference(
    preference: &str,
    observed_quality: Option<&str>,
) -> bool {
    let Some(observed_quality) = observed_quality.and_then(normalize_kugou_returned_quality) else {
        return true;
    };

    match preference {
        "highest" => true,
        "standard" => matches!(observed_quality.as_str(), "standard" | "128"),
        "high" => matches!(observed_quality.as_str(), "high" | "exhigh" | "320"),
        "lossless" => matches!(observed_quality.as_str(), "lossless" | "flac"),
        "hires" => observed_quality == "hires",
        "super" => observed_quality == "super",
        "viper_clear" => observed_quality == "viper_clear",
        "viper_hifi" => observed_quality == "viper_hifi",
        "viper_tape" => observed_quality == "viper_tape",
        "viper_atmos" => observed_quality == "viper_atmos",
        "multitrack" => observed_quality == "multitrack",
        _ => true,
    }
}

fn kugou_quality_mismatch_message(preference: &str, observed_quality: Option<&str>) -> String {
    let expected_label = kugou_quality_label(preference);
    let actual_label = observed_quality
        .and_then(normalize_kugou_returned_quality)
        .map(|quality| kugou_quality_label(&quality).to_string())
        .unwrap_or_else(|| "未知音质".to_string());

    format!(
        "酷狗返回的实际音质为{actual_label}，不满足所选{expected_label}；已拒绝把低音质链接当作切换成功。"
    )
}

fn resolve_kugou_quality_level(
    response: &Value,
    requested_quality: Option<&str>,
    bitrate: Option<u64>,
    file_type: Option<&str>,
) -> Option<String> {
    find_string_by_keys(
        response,
        &[
            "quality_level",
            "qualityLevel",
            "quality_name",
            "qualityName",
            "audio_quality_name",
            "audioQualityName",
            "level",
            "quality_type",
            "qualityType",
        ],
    )
    .and_then(|value| normalize_kugou_returned_quality(&value))
    .or_else(|| {
        requested_quality
            .filter(|quality| {
                matches!(
                    *quality,
                    "super"
                        | "viper_clear"
                        | "viper_hifi"
                        | "viper_tape"
                        | "viper_atmos"
                        | "multitrack"
                        | "hires"
                )
            })
            .and_then(normalize_kugou_returned_quality)
    })
    .or_else(|| infer_kugou_quality_from_media(file_type, bitrate))
    .or_else(|| requested_quality.and_then(normalize_kugou_returned_quality))
}

fn kugou_quality_label_string(level: Option<&str>) -> Option<String> {
    level
        .map(kugou_quality_label)
        .filter(|label| *label != "平台返回音质")
        .map(str::to_string)
}

fn kugou_hash_candidate_label(index: usize, total: usize) -> String {
    if total <= 1 {
        "候选音源".to_string()
    } else {
        format!("候选音源 {}/{}", index.saturating_add(1), total)
    }
}

fn summarize_playback_attempt_errors(prefix: &str, errors: &[String]) -> String {
    if errors.is_empty() {
        return format!("{prefix}：未返回可用播放地址");
    }

    if errors.len() == 1 {
        return errors[0].clone();
    }

    let preview = errors
        .iter()
        .take(2)
        .map(|error| error.as_str())
        .collect::<Vec<_>>()
        .join("；");
    if errors.len() > 2 {
        format!(
            "{prefix}：已尝试 {} 个候选，错误摘要：{preview}；...",
            errors.len()
        )
    } else {
        format!("{prefix}：已尝试 {} 个候选，错误：{preview}", errors.len())
    }
}

fn playback_errors_indicate_paid(errors: &[String]) -> bool {
    errors.iter().any(|error| {
        let lowered = error.to_ascii_lowercase();
        error.contains("需要付费")
            || error.contains("会员")
            || error.contains("版权")
            || lowered.contains("vip")
            || lowered.contains("paid")
    })
}

fn playback_errors_indicate_preview(errors: &[String]) -> bool {
    errors.iter().any(|error| {
        error.contains("疑似试听片段")
            || error.contains("试听片段")
            || error.contains("可读大小小于歌曲摘要")
            || error.contains("只返回试听")
    })
}

fn playback_failure_reason(value: &Value) -> Option<String> {
    find_string_by_keys(
        value,
        &[
            "error",
            "errmsg",
            "message",
            "msg",
            "reason",
            "fail_reason",
            "failReason",
            "toast",
            "tips",
        ],
    )
    .filter(|value| !value.trim().is_empty())
}

fn append_kugou_playback_diagnostic(message: String, response: &Value) -> String {
    format!(
        "{message}（{}）",
        kugou_playback_failure_diagnostic(response)
    )
}

fn kugou_playback_failure_diagnostic(response: &Value) -> String {
    let mut parts = vec![describe_response_shape(response)];

    if let Some(priv_status) = find_value_by_keys(response, &["priv_status", "privStatus"])
        .and_then(diagnostic_scalar_value)
    {
        parts.push(format!("priv_status={priv_status}"));
    }

    if let Some(fail_process) = find_value_by_keys(response, &["fail_process", "failProcess"]) {
        parts.push(format!(
            "fail_process={}",
            summarize_diagnostic_value(fail_process)
        ));
    }

    if let Some(auth_through) = find_value_by_keys(response, &["auth_through", "authThrough"]) {
        parts.push(format!(
            "auth_through={}",
            summarize_diagnostic_value(auth_through)
        ));
    }

    if let Some(tracker_through) =
        find_value_by_keys(response, &["tracker_through", "trackerThrough"])
    {
        let tracker_summary = summarize_selected_object_fields(
            tracker_through,
            &[
                "identity_block",
                "cpy_grade",
                "musicpack_advance",
                "all_quality_free",
                "cpy_level",
                "area_block",
                "risk_block",
                "play_block",
            ],
        )
        .unwrap_or_else(|| summarize_diagnostic_value(tracker_through));
        parts.push(format!("tracker_through={tracker_summary}"));
    }

    if let Some(trans_param) = find_value_by_keys(response, &["trans_param", "transParam"]) {
        parts.push(format!(
            "trans_param={}",
            summarize_kugou_trans_param(trans_param)
        ));
    }

    if let Some(hash_offset) = find_value_by_keys(response, &["hash_offset", "hashOffset"]) {
        let offset_summary = summarize_selected_object_fields(
            hash_offset,
            &["start_ms", "end_ms", "start_byte", "end_byte", "file_type"],
        )
        .unwrap_or_else(|| summarize_diagnostic_value(hash_offset));
        parts.push(format!("hash_offset={offset_summary}"));
    }

    parts.join("；")
}

fn summarize_kugou_trans_param(value: &Value) -> String {
    let mut parts = Vec::new();
    if let Some(summary) = summarize_selected_object_fields(
        value,
        &[
            "display",
            "display_rate",
            "pay_block_tpl",
            "cpy_attr0",
            "classmap",
            "ipmap",
        ],
    ) {
        parts.push(summary);
    }

    if let Some(qualitymap) = field_value(value, "qualitymap") {
        let mut quality_parts = vec![format!("字段={}", object_keys_summary(qualitymap))];
        if let Some(bits_len) = field_value(qualitymap, "bits")
            .and_then(Value::as_str)
            .map(|text| text.trim().chars().count())
        {
            quality_parts.push(format!("bits长度={bits_len}"));
        }
        if let Some(attr_summary) =
            summarize_selected_object_fields(qualitymap, &["attr0", "attr1"])
        {
            quality_parts.push(attr_summary);
        }
        parts.push(format!("qualitymap{{{}}}", quality_parts.join(",")));
    }

    let present_hash_fields = present_field_names(
        value,
        &[
            "std_hash",
            "hash_128",
            "hash_320",
            "hash_flac",
            "flac_hash",
            "sq_hash",
            "hires_hash",
            "high_hash",
            "super_file_hash",
            "hash_multitrack",
            "ogg_128_hash",
            "ogg_320_hash",
            "ogg_128_filesize",
            "ogg_320_filesize",
        ],
    );
    if !present_hash_fields.is_empty() {
        parts.push(format!("存在字段={}", present_hash_fields.join(",")));
    }

    if parts.is_empty() {
        summarize_diagnostic_value(value)
    } else {
        parts.join(",")
    }
}

fn summarize_selected_object_fields(value: &Value, keys: &[&str]) -> Option<String> {
    let mut parts = Vec::new();
    for key in keys {
        let Some(field) = field_value(value, key) else {
            continue;
        };
        if let Some(scalar) = diagnostic_scalar_value(field) {
            parts.push(format!("{key}={scalar}"));
        } else if field.is_object() {
            parts.push(format!("{key}字段={}", object_keys_summary(field)));
        } else if field.is_array() {
            parts.push(format!("{key}={}", summarize_diagnostic_value(field)));
        }
    }

    if !parts.is_empty() {
        return Some(parts.join(","));
    }

    value
        .as_object()
        .map(|_| format!("字段={}", object_keys_summary(value)))
}

fn present_field_names(value: &Value, keys: &[&str]) -> Vec<String> {
    keys.iter()
        .filter(|key| {
            field_value(value, key)
                .map(|field| match field {
                    Value::Null => false,
                    Value::String(text) => !text.trim().is_empty(),
                    Value::Array(items) => !items.is_empty(),
                    Value::Object(map) => !map.is_empty(),
                    _ => true,
                })
                .unwrap_or(false)
        })
        .map(|key| (*key).to_string())
        .collect()
}

fn summarize_diagnostic_value(value: &Value) -> String {
    match value {
        Value::Array(items) => {
            if items.is_empty() {
                return "空数组".to_string();
            }
            let preview = items
                .iter()
                .take(5)
                .map(|item| {
                    diagnostic_scalar_value(item).unwrap_or_else(|| match item {
                        Value::Object(_) => format!("对象字段={}", object_keys_summary(item)),
                        Value::Array(nested) => format!("数组长度={}", nested.len()),
                        Value::Null => "null".to_string(),
                        _ => "复杂值".to_string(),
                    })
                })
                .collect::<Vec<_>>()
                .join(",");
            if items.len() > 5 {
                format!("数组长度={}，前5项={preview}", items.len())
            } else {
                format!("数组[{preview}]")
            }
        }
        Value::Object(_) => format!("对象字段={}", object_keys_summary(value)),
        _ => diagnostic_scalar_value(value).unwrap_or_else(|| "复杂值".to_string()),
    }
}

fn diagnostic_scalar_value(value: &Value) -> Option<String> {
    match value {
        Value::Bool(value) => Some(value.to_string()),
        Value::Number(value) => Some(value.to_string()),
        Value::String(text) => Some(sanitize_diagnostic_text(text)),
        _ => None,
    }
}

fn sanitize_diagnostic_text(value: &str) -> String {
    let text = value.split_whitespace().collect::<Vec<_>>().join(" ");
    let text = text.trim();
    if text.is_empty() {
        return "空".to_string();
    }
    let lowered = text.to_ascii_lowercase();
    if lowered.contains("http://") || lowered.contains("https://") {
        return "链接".to_string();
    }
    if (16..=128).contains(&text.len()) && text.chars().all(|char| char.is_ascii_hexdigit()) {
        return format!("hash长度{}", text.len());
    }

    text.chars().take(48).collect::<String>()
}

fn find_value_by_keys<'a>(value: &'a Value, keys: &[&str]) -> Option<&'a Value> {
    match value {
        Value::Object(map) => {
            for key in keys {
                if let Some(found) = field_value(value, key) {
                    return Some(found);
                }
            }
            map.values()
                .find_map(|value| find_value_by_keys(value, keys))
        }
        Value::Array(items) => items
            .iter()
            .find_map(|value| find_value_by_keys(value, keys)),
        _ => None,
    }
}

fn find_string_by_keys(value: &Value, keys: &[&str]) -> Option<String> {
    match value {
        Value::Object(map) => {
            for key in keys {
                if let Some(text) = value_as_string(value, key).filter(|value| !value.is_empty()) {
                    return Some(text);
                }
            }
            map.values()
                .find_map(|value| find_string_by_keys(value, keys))
        }
        Value::Array(items) => items
            .iter()
            .find_map(|value| find_string_by_keys(value, keys)),
        _ => None,
    }
}

fn find_u64_by_keys(value: &Value, keys: &[&str]) -> Option<u64> {
    match value {
        Value::Object(map) => {
            for key in keys {
                if let Some(number) = value_as_u64(value, key) {
                    return Some(number);
                }
            }
            map.values().find_map(|value| find_u64_by_keys(value, keys))
        }
        Value::Array(items) => items.iter().find_map(|value| find_u64_by_keys(value, keys)),
        _ => None,
    }
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

fn kugou_vip_type_label(vip_type: u64) -> String {
    match vip_type {
        0 => "普通账号".to_string(),
        _ => format!("VIP 类型 {vip_type}"),
    }
}

fn extract_kugou_membership(data: &Value) -> KugouMembershipInfo {
    let vip_type = first_u64(
        data,
        &["vip_type", "vipType", "viptype", "kg_vip_type", "vip"],
    );
    let type_label = first_string(
        data,
        &[
            "vip_name",
            "vipName",
            "vip_type_name",
            "vipTypeName",
            "memberName",
        ],
    )
    .filter(|value| !value.is_empty())
    .or_else(|| {
        vip_type
            .filter(|value| *value > 0)
            .map(kugou_vip_type_label)
    });
    let level_label = first_u64(data, &["vip_level", "vipLevel", "memberLevel"])
        .filter(|value| *value > 0)
        .map(|value| format!("等级 {value}"));
    let expire_at = first_u64(
        data,
        &[
            "vip_end_time",
            "vipEndTime",
            "vip_expire_time",
            "vipExpireTime",
            "expire_time",
            "expireTime",
        ],
    )
    .and_then(normalize_epoch_seconds);
    let has_vip_token = first_string(data, &["vip_token", "vipToken", "viptoken"])
        .filter(|value| !value.is_empty())
        .is_some();
    let has_membership_field =
        vip_type.is_some() || type_label.is_some() || level_label.is_some() || expire_at.is_some();
    let active = vip_type.unwrap_or_default() > 0 || has_vip_token;

    KugouMembershipInfo {
        active,
        status_label: if active && has_vip_token {
            "已检测到会员播放凭据".to_string()
        } else if active {
            "已检测到会员".to_string()
        } else if has_membership_field {
            "普通账号".to_string()
        } else {
            "未检测到会员信息".to_string()
        },
        type_label,
        level_label,
        expire_at,
    }
}

fn kugou_membership_from_cookies(
    cookies: &BTreeMap<String, String>,
    fallback: Option<&KugouMembershipInfo>,
) -> KugouMembershipInfo {
    let vip_type = first_cookie_value(cookies, &["vip_type", "vipType", "viptype", "kg_vip_type"])
        .and_then(|value| value.parse::<u64>().ok());
    let has_vip_token = first_cookie_value(
        cookies,
        &["vip_token", "vipToken", "viptoken", "kg_vip_token"],
    )
    .is_some();
    let fallback_active = fallback.map(|value| value.active).unwrap_or(false);
    let active = vip_type.unwrap_or_default() > 0 || has_vip_token || fallback_active;
    let type_label = fallback
        .and_then(|value| value.type_label.clone())
        .or_else(|| {
            vip_type
                .filter(|value| *value > 0)
                .map(kugou_vip_type_label)
        });
    let level_label = fallback.and_then(|value| value.level_label.clone());
    let expire_at = fallback.and_then(|value| value.expire_at.clone());
    let has_membership_field =
        vip_type.is_some() || type_label.is_some() || level_label.is_some() || expire_at.is_some();

    KugouMembershipInfo {
        active,
        status_label: if active && has_vip_token {
            "已检测到会员播放凭据".to_string()
        } else if active {
            "已检测到会员".to_string()
        } else if has_membership_field {
            "普通账号".to_string()
        } else {
            "未检测到会员信息".to_string()
        },
        type_label,
        level_label,
        expire_at,
    }
}

fn extract_login_profile(data: &Value, user_id: &str) -> KugouLoginProfile {
    let nickname = value_as_string(data, "nickname")
        .or_else(|| value_as_string(data, "username"))
        .or_else(|| value_as_string(data, "nick_name"))
        .or_else(|| value_as_string(data, "user_name"))
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| format!("酷狗用户 {user_id}"));
    let avatar_url = value_as_string(data, "pic")
        .or_else(|| value_as_string(data, "img"))
        .or_else(|| value_as_string(data, "avatar"))
        .and_then(|value| normalize_cover_url(value));

    KugouLoginProfile {
        user_id: user_id.to_string(),
        nickname,
        avatar_url,
        membership: Some(extract_kugou_membership(data)),
    }
}

fn init_device_cookies() -> BTreeMap<String, String> {
    let guid = pseudo_uuid();
    let mid = calculate_mid(&guid);
    let mut cookies = BTreeMap::new();
    cookies.insert("KUGOU_API_GUID".to_string(), guid);
    cookies.insert("KUGOU_API_MID".to_string(), mid);
    cookies.insert("KUGOU_API_MAC".to_string(), pseudo_alnum(12));
    cookies.insert("KUGOU_API_DEV".to_string(), pseudo_alnum(16));
    cookies
}

fn calculate_mid(seed: &str) -> String {
    let mut hasher = Md5::new();
    hasher.update(seed.as_bytes());
    let digest = hasher.finalize();
    let mut bytes = [0u8; 8];
    bytes.copy_from_slice(&digest[..8]);
    u64::from_be_bytes(bytes).to_string()
}

fn signature_web(params: &BTreeMap<String, String>) -> String {
    let params_string = params
        .iter()
        .filter(|(key, _)| key.as_str() != "signature")
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>()
        .join("");
    let mut hasher = Md5::new();
    hasher.update(
        format!("{KUGOU_SIGNATURE_WEB_SECRET}{params_string}{KUGOU_SIGNATURE_WEB_SECRET}")
            .as_bytes(),
    );
    format!("{:x}", hasher.finalize())
}

fn signature_android_with_secret(
    params: &BTreeMap<String, String>,
    data_text: &str,
    signature_secret: &str,
) -> String {
    let params_string = params
        .iter()
        .filter(|(key, _)| key.as_str() != "signature")
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>()
        .join("");
    let mut hasher = Md5::new();
    hasher.update(
        format!("{signature_secret}{params_string}{data_text}{signature_secret}").as_bytes(),
    );
    format!("{:x}", hasher.finalize())
}

fn sign_params_key(data: &str) -> String {
    let mut hasher = Md5::new();
    hasher.update(
        format!("{KUGOU_APPID}OIlwieks28dk2k092lksi2UIkp{KUGOU_CLIENTVER}{data}").as_bytes(),
    );
    format!("{:x}", hasher.finalize())
}

fn aes256_cbc_encrypt_hex_value(value: &Value, key: &str, iv: &str) -> Result<String, String> {
    let text =
        serde_json::to_string(value).map_err(|err| format!("序列化酷狗加密参数失败：{err}"))?;
    aes256_cbc_encrypt_hex_text(&text, key, iv)
}

fn aes256_cbc_encrypt_hex_text(text: &str, key: &str, iv: &str) -> Result<String, String> {
    let encrypted = CbcEncryptor::<Aes256>::new_from_slices(key.as_bytes(), iv.as_bytes())
        .map_err(|err| format!("初始化酷狗 AES 加密失败：{err}"))?
        .encrypt_padded_vec_mut::<Pkcs7>(text.as_bytes());
    Ok(hex_encode(&encrypted))
}

fn aes256_cbc_encrypt_hex_with_temp_key(value: &Value) -> Result<(String, String), String> {
    let temp_key = pseudo_alnum(16).to_lowercase();
    let md5_key = md5_hex(&temp_key);
    let iv = &md5_key[md5_key.len() - 16..];
    let encrypted = aes256_cbc_encrypt_hex_value(value, &md5_key, iv)?;
    Ok((encrypted, temp_key))
}

fn aes256_cbc_decrypt_hex_with_temp_key(
    data: &str,
    temp_key: &str,
) -> Result<Option<Value>, String> {
    let md5_key = md5_hex(temp_key);
    let iv = &md5_key[md5_key.len() - 16..];
    let encrypted = hex_decode(data)?;
    let decrypted = CbcDecryptor::<Aes256>::new_from_slices(md5_key.as_bytes(), iv.as_bytes())
        .map_err(|err| format!("初始化酷狗 AES 解密失败：{err}"))?
        .decrypt_padded_vec_mut::<Pkcs7>(&encrypted)
        .map_err(|err| format!("解密酷狗会员 token 失败：{err}"))?;
    let text =
        String::from_utf8(decrypted).map_err(|err| format!("解析酷狗会员 token 失败：{err}"))?;
    if text.trim().is_empty() {
        return Ok(None);
    }
    Ok(Some(
        serde_json::from_str::<Value>(&text).unwrap_or_else(|_| Value::String(text)),
    ))
}

fn playlist_aes_encrypt_base64_value(value: &Value, temp_key: &str) -> Result<String, String> {
    let text =
        serde_json::to_string(value).map_err(|err| format!("序列化酷狗设备注册参数失败：{err}"))?;
    let md5_key = md5_hex(temp_key);
    let key = &md5_key[..16];
    let iv = &md5_key[16..32];
    let encrypted = CbcEncryptor::<Aes128>::new_from_slices(key.as_bytes(), iv.as_bytes())
        .map_err(|err| format!("初始化酷狗设备注册 AES 加密失败：{err}"))?
        .encrypt_padded_vec_mut::<Pkcs7>(text.as_bytes());
    Ok(general_purpose::STANDARD.encode(encrypted))
}

fn playlist_aes_decrypt_bytes(bytes: &[u8], temp_key: &str) -> Result<Value, String> {
    let md5_key = md5_hex(temp_key);
    let key = &md5_key[..16];
    let iv = &md5_key[16..32];
    let decrypt = |ciphertext: &[u8]| {
        CbcDecryptor::<Aes128>::new_from_slices(key.as_bytes(), iv.as_bytes())
            .map_err(|err| format!("初始化酷狗设备注册 AES 解密失败：{err}"))?
            .decrypt_padded_vec_mut::<Pkcs7>(ciphertext)
            .map_err(|err| format!("解密酷狗设备注册响应失败：{err}"))
    };
    let decrypted = decrypt(bytes).or_else(|raw_err| {
        let text = String::from_utf8_lossy(bytes);
        let decoded = general_purpose::STANDARD
            .decode(text.trim())
            .map_err(|_| raw_err.clone())?;
        decrypt(&decoded).map_err(|_| raw_err)
    })?;
    let text =
        String::from_utf8(decrypted).map_err(|err| format!("解析酷狗设备注册响应失败：{err}"))?;
    if text.trim().is_empty() {
        return Err("酷狗设备注册响应为空。".to_string());
    }

    Ok(serde_json::from_str::<Value>(&text).unwrap_or_else(|_| Value::String(text)))
}

fn rsa_raw_encrypt_lite_hex(text: &str) -> Result<String, String> {
    let modulus = BigUint::parse_bytes(KUGOU_LITE_RSA_MODULUS_HEX.as_bytes(), 16)
        .ok_or_else(|| "解析酷狗 RSA 公钥失败。".to_string())?;
    let exponent = BigUint::parse_bytes(KUGOU_RSA_PUBLIC_EXPONENT_HEX.as_bytes(), 16)
        .ok_or_else(|| "解析酷狗 RSA 指数失败。".to_string())?;
    let key_len = KUGOU_LITE_RSA_MODULUS_HEX.len() / 2;
    let bytes = text.as_bytes();
    if bytes.len() > key_len {
        return Err("酷狗 RSA 加密数据过长。".to_string());
    }
    let mut padded = vec![0u8; key_len];
    padded[..bytes.len()].copy_from_slice(bytes);
    let encrypted = BigUint::from_bytes_be(&padded).modpow(&exponent, &modulus);
    let mut output = encrypted.to_bytes_be();
    if output.len() < key_len {
        let mut padded_output = vec![0u8; key_len - output.len()];
        padded_output.extend(output);
        output = padded_output;
    }
    Ok(hex_encode(&output))
}

fn rsa_pkcs1_encrypt_lite_hex(text: &str) -> Result<String, String> {
    let modulus = BigUint::parse_bytes(KUGOU_LITE_RSA_MODULUS_HEX.as_bytes(), 16)
        .ok_or_else(|| "解析酷狗 RSA 公钥失败。".to_string())?;
    let exponent = BigUint::parse_bytes(KUGOU_RSA_PUBLIC_EXPONENT_HEX.as_bytes(), 16)
        .ok_or_else(|| "解析酷狗 RSA 指数失败。".to_string())?;
    let key_len = KUGOU_LITE_RSA_MODULUS_HEX.len() / 2;
    let bytes = text.as_bytes();
    if bytes.len() + 11 > key_len {
        return Err("酷狗 RSA 加密数据过长。".to_string());
    }

    let padding_len = key_len - bytes.len() - 3;
    let mut padded = Vec::with_capacity(key_len);
    padded.push(0);
    padded.push(2);
    padded.extend(pseudo_nonzero_bytes(padding_len));
    padded.push(0);
    padded.extend(bytes);

    let encrypted = BigUint::from_bytes_be(&padded).modpow(&exponent, &modulus);
    let mut output = encrypted.to_bytes_be();
    if output.len() < key_len {
        let mut padded_output = vec![0u8; key_len - output.len()];
        padded_output.extend(output);
        output = padded_output;
    }
    Ok(hex_encode(&output))
}

fn pseudo_nonzero_bytes(len: usize) -> Vec<u8> {
    let mut seed = current_epoch_millis() as u64 ^ ((std::process::id() as u64) << 23) ^ len as u64;
    let mut output = Vec::with_capacity(len);
    for _ in 0..len {
        seed = seed
            .wrapping_mul(2862933555777941757)
            .wrapping_add(3037000493);
        output.push(((seed % 255) + 1) as u8);
    }
    output
}

fn md5_hex(value: &str) -> String {
    let mut hasher = Md5::new();
    hasher.update(value.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>()
}

fn hex_decode(value: &str) -> Result<Vec<u8>, String> {
    let value = value.trim();
    if value.len() % 2 != 0 {
        return Err("酷狗 hex 数据长度无效。".to_string());
    }
    (0..value.len())
        .step_by(2)
        .map(|index| {
            u8::from_str_radix(&value[index..index + 2], 16)
                .map_err(|err| format!("解析酷狗 hex 数据失败：{err}"))
        })
        .collect()
}

fn create_qr_image(qr_url: &str) -> Result<String, String> {
    let code =
        QrCode::new(qr_url.as_bytes()).map_err(|err| format!("生成酷狗二维码失败：{err}"))?;
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

fn collect_set_cookies(headers: &HeaderMap) -> Vec<String> {
    headers
        .get_all(SET_COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .map(str::to_string)
        .collect()
}

fn merge_set_cookies(cookies: &mut BTreeMap<String, String>, set_cookies: &[String]) {
    for raw in set_cookies {
        let Some(first_part) = raw.split(';').next() else {
            continue;
        };
        let Some((key, value)) = first_part.split_once('=') else {
            continue;
        };
        let key = key.trim();
        if key.is_empty() {
            continue;
        }

        cookies.insert(key.to_string(), value.trim().to_string());
    }
}

fn kugou_qr_status_name(code: i64) -> &'static str {
    match code {
        1 => "waiting",
        2 | 3 => "scanned",
        4 => "authorized",
        0 | 5 => "expired",
        _ => "unknown",
    }
}

fn kugou_qr_status_message(code: i64) -> &'static str {
    match code {
        1 => "等待使用酷狗音乐 App 扫码。",
        2 | 3 => "已扫码，请在手机上确认登录。",
        4 => "酷狗授权已完成。",
        0 | 5 => "酷狗二维码已过期，请重新生成。",
        _ => "酷狗返回未知扫码状态。",
    }
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

fn write_credential(app: &AppHandle, credential: &KugouCredential) -> Result<(), String> {
    let path = credential_file(app)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| format!("创建酷狗凭据目录失败：{err}"))?;
    }
    let content = serde_json::to_string_pretty(credential)
        .map_err(|err| format!("序列化酷狗凭据失败：{err}"))?;
    fs::write(path, content).map_err(|err| format!("保存酷狗凭据失败：{err}"))
}

fn read_credential(app: &AppHandle) -> Result<Option<KugouCredential>, String> {
    let path = credential_file(app)?;
    if !path.is_file() {
        return Ok(None);
    }
    let content = fs::read_to_string(path).map_err(|err| format!("读取酷狗凭据失败：{err}"))?;
    serde_json::from_str(&content)
        .map(Some)
        .map_err(|err| format!("解析酷狗凭据失败：{err}"))
}

fn write_pending_login(app: &AppHandle, pending: &KugouPendingLogin) -> Result<(), String> {
    let path = pending_file(app)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| format!("创建酷狗登录目录失败：{err}"))?;
    }
    let content = serde_json::to_string_pretty(pending)
        .map_err(|err| format!("序列化酷狗登录状态失败：{err}"))?;
    fs::write(path, content).map_err(|err| format!("保存酷狗登录状态失败：{err}"))
}

fn read_pending_login(app: &AppHandle) -> Result<Option<KugouPendingLogin>, String> {
    let path = pending_file(app)?;
    if !path.is_file() {
        return Ok(None);
    }
    let content = fs::read_to_string(path).map_err(|err| format!("读取酷狗登录状态失败：{err}"))?;
    serde_json::from_str(&content)
        .map(Some)
        .map_err(|err| format!("解析酷狗登录状态失败：{err}"))
}

fn clear_pending_login(app: &AppHandle) -> Result<(), String> {
    let path = pending_file(app)?;
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(format!("清除酷狗登录状态失败：{err}")),
    }
}

fn first_array_at_paths<'a>(value: &'a Value, paths: &[&[&str]]) -> Option<&'a Vec<Value>> {
    for path in paths {
        if let Some(items) = value_at_path(value, path).and_then(Value::as_array) {
            return Some(items);
        }
    }

    None
}

fn first_track_array_at_paths<'a>(value: &'a Value, paths: &[&[&str]]) -> Option<&'a Vec<Value>> {
    for path in paths {
        let Some(items) = value_at_path(value, path).and_then(Value::as_array) else {
            continue;
        };
        if is_track_array(items) {
            return Some(items);
        }
    }

    None
}

fn first_playlist_array_at_paths<'a>(
    value: &'a Value,
    paths: &[&[&str]],
) -> Option<&'a Vec<Value>> {
    for path in paths {
        let Some(items) = value_at_path(value, path).and_then(Value::as_array) else {
            continue;
        };
        if is_playlist_array(items) {
            return Some(items);
        }
    }

    None
}

fn first_empty_array_at_paths<'a>(value: &'a Value, paths: &[&[&str]]) -> Option<&'a Vec<Value>> {
    for path in paths {
        let Some(items) = value_at_path(value, path).and_then(Value::as_array) else {
            continue;
        };
        if items.is_empty() {
            return Some(items);
        }
    }

    None
}

fn find_playlist_array(value: &Value) -> Option<&Vec<Value>> {
    match value {
        Value::Array(items) => {
            if is_playlist_array(items) {
                return Some(items);
            }
            items.iter().find_map(find_playlist_array)
        }
        Value::Object(map) => map.values().find_map(find_playlist_array),
        _ => None,
    }
}

fn find_track_array(value: &Value) -> Option<&Vec<Value>> {
    match value {
        Value::Array(items) => {
            if is_track_array(items) {
                return Some(items);
            }
            items.iter().find_map(find_track_array)
        }
        Value::Object(map) => map.values().find_map(find_track_array),
        _ => None,
    }
}

fn find_owned_track_array(value: &Value) -> Option<Vec<Value>> {
    match value {
        Value::Array(items) => {
            if is_track_array(items) {
                return Some(items.clone());
            }
            items.iter().find_map(find_owned_track_array)
        }
        Value::Object(map) => map.values().find_map(find_owned_track_array),
        Value::String(text) => decode_track_items_from_text(text),
        _ => None,
    }
}

fn decode_track_items_from_value(value: &Value) -> Option<Vec<Value>> {
    find_owned_track_array(value).or_else(|| value.as_str().and_then(decode_track_items_from_text))
}

fn decode_track_items_from_text(text: &str) -> Option<Vec<Value>> {
    let mut current = text.trim().to_string();
    if current.is_empty() {
        return None;
    }

    for _ in 0..3 {
        let parsed = serde_json::from_str::<Value>(&current).ok()?;
        if let Some(items) = find_owned_track_array(&parsed) {
            return Some(items);
        }
        let Some(inner) = parsed
            .as_str()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        else {
            return None;
        };
        if inner == current {
            return None;
        }
        current = inner.to_string();
    }

    None
}

fn is_track_array(items: &[Value]) -> bool {
    if items.is_empty() {
        return false;
    }

    let mut checked = 0usize;
    let mut track_like = 0usize;
    for item in items.iter().take(8) {
        if !item.is_object() {
            continue;
        }
        checked += 1;
        if parse_kugou_track(item).is_some() {
            track_like += 1;
        }
    }

    checked > 0 && track_like > 0 && track_like * 2 >= checked
}

fn is_playlist_array(items: &[Value]) -> bool {
    if items.is_empty() {
        return false;
    }

    let mut checked = 0usize;
    let mut playlist_like = 0usize;
    for item in items.iter().take(8) {
        if !item.is_object() {
            continue;
        }
        checked += 1;
        if parse_playlist_summary(item).is_some() {
            playlist_like += 1;
        }
    }

    checked > 0 && playlist_like > 0 && playlist_like * 2 >= checked
}

fn request_kugou_playlist_detail_page(
    list_id: &str,
    user_id: &str,
    token: &str,
    page: u64,
    limit: u64,
    cookies: &mut BTreeMap<String, String>,
) -> Result<Value, String> {
    let body = json!({
        "listid": list_id,
        "userid": user_id,
        "area_code": 1,
        "show_relate_goods": 0,
        "pagesize": limit,
        "allplatform": 1,
        "show_cover": 1,
        "type": 0,
        "token": token,
        "page": page,
    });
    kugou_android_post_json(
        KUGOU_GATEWAY_BASE,
        "/v4/get_list_all_file",
        BTreeMap::new(),
        body,
        cookies,
        Some("cloudlist.service.kugou.com"),
    )
}

fn request_public_playlist_summary(
    playlist_id: &str,
    cookies: &mut BTreeMap<String, String>,
) -> Result<KugouPlaylistSummary, String> {
    let user_id = cookies
        .get("userid")
        .cloned()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "0".to_string());
    let token = cookies
        .get("token")
        .cloned()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_default();
    let response = kugou_android_post_json(
        KUGOU_GATEWAY_BASE,
        "/v3/get_list_info",
        BTreeMap::new(),
        json!({
            "data": [{ "global_collection_id": playlist_id }],
            "userid": user_id,
            "token": token,
        }),
        cookies,
        Some("pubsongs.kugou.com"),
    )?;
    ensure_kugou_success(&response, "酷狗推荐歌单信息读取")?;
    let items = first_playlist_array_at_paths(
        &response,
        &[
            &["data"],
            &["data", "info"],
            &["data", "list"],
            &["data", "lists"],
            &["info"],
            &["list"],
        ],
    )
    .or_else(|| find_playlist_array(&response))
    .ok_or_else(|| "酷狗推荐歌单信息响应缺少歌单摘要。".to_string())?;

    items
        .iter()
        .find_map(parse_playlist_summary)
        .ok_or_else(|| "酷狗推荐歌单信息缺少有效歌单摘要。".to_string())
}

fn request_public_playlist_track_page(
    playlist_id: &str,
    page: u64,
    limit: u64,
    cookies: &mut BTreeMap<String, String>,
) -> Result<Value, String> {
    let mut params = BTreeMap::new();
    params.insert("area_code".to_string(), "1".to_string());
    params.insert(
        "begin_idx".to_string(),
        page.saturating_sub(1).saturating_mul(limit).to_string(),
    );
    params.insert("plat".to_string(), "1".to_string());
    params.insert("type".to_string(), "1".to_string());
    params.insert("mode".to_string(), "1".to_string());
    params.insert("personal_switch".to_string(), "1".to_string());
    params.insert(
        "extend_fields".to_string(),
        "abtags,hot_cmt,popularization".to_string(),
    );
    params.insert("pagesize".to_string(), limit.to_string());
    params.insert("global_collection_id".to_string(), playlist_id.to_string());

    let mut temp_credential = KugouCredential {
        cookies: std::mem::take(cookies),
        saved_at: app_data::now_seconds(),
        profile: None,
    };
    let response = kugou_android_get_json_with_client(
        KUGOU_GATEWAY_BASE,
        "/pubsongs/v2/get_other_list_file_nofilt",
        params,
        &mut temp_credential,
        None,
        None,
        KUGOU_APPID,
        KUGOU_CLIENTVER,
        KUGOU_SIGNATURE_ANDROID_SECRET,
    );
    *cookies = temp_credential.cookies;

    response
}

fn parse_playlist_summary_from_response(response: &Value) -> Option<KugouPlaylistSummary> {
    response
        .get("data")
        .and_then(|data| {
            data.get("list_info")
                .or_else(|| data.get("list"))
                .or_else(|| data.get("info"))
        })
        .filter(|value| value.is_object())
        .and_then(parse_playlist_summary)
}

fn kugou_playlist_total_track_count(response: &Value, playlist: &KugouPlaylistSummary) -> u64 {
    let response_data = response.get("data").unwrap_or(&Value::Null);
    value_as_u64(response_data, "total")
        .or_else(|| value_as_u64(response_data, "count"))
        .or_else(|| value_as_u64(response_data, "total_count"))
        .or_else(|| (playlist.track_count > 0).then_some(playlist.track_count))
        .unwrap_or_default()
}

fn kugou_playlist_track_items_from_response(
    response: &Value,
    track_paths: &[&[&str]],
) -> Option<Vec<Value>> {
    first_track_array_at_paths(response, track_paths)
        .or_else(|| find_track_array(response))
        .or_else(|| first_empty_array_at_paths(response, track_paths))
        .cloned()
        .or_else(|| {
            value_at_path(response, &["data", "snap"]).and_then(decode_track_items_from_value)
        })
}

fn reverse_playlist_page_bounds(total: u64, page: u64, limit: u64) -> Option<(u64, u64)> {
    let reverse_offset = page.saturating_sub(1).saturating_mul(limit);
    if reverse_offset >= total {
        return None;
    }

    let reverse_end = reverse_offset.saturating_add(limit).min(total);
    Some((
        total.saturating_sub(reverse_end),
        total.saturating_sub(reverse_offset),
    ))
}

fn parse_playlist_track_items_reversed(items: &[Value], limit: usize) -> Vec<KugouSearchTrack> {
    items
        .iter()
        .rev()
        .filter_map(parse_kugou_track)
        .take(limit)
        .collect::<Vec<_>>()
}

fn value_at_path<'a>(value: &'a Value, path: &[&str]) -> Option<&'a Value> {
    let mut current = value;
    for key in path {
        current = current.get(*key)?;
    }
    Some(current)
}

fn parse_playlist_summary(item: &Value) -> Option<KugouPlaylistSummary> {
    let global_collection_id = first_string(
        item,
        &[
            "global_collection_id",
            "global_collectionid",
            "collection_id",
            "global_id",
        ],
    );
    let list_id = first_string(
        item,
        &[
            "listid",
            "list_id",
            "id",
            "list_create_listid",
            "specialid",
            "special_id",
            "specialId",
        ],
    )
    .or_else(|| global_collection_id.clone())?;
    let name = first_string(
        item,
        &[
            "name",
            "listname",
            "list_name",
            "specialname",
            "title",
            "filename",
        ],
    )
    .map(|value| strip_html_tags(&value).trim().to_string())
    .filter(|value| !value.is_empty())
    .unwrap_or_else(|| "酷狗歌单".to_string());
    let cover_img_url = first_string(
        item,
        &[
            "imgurl",
            "pic",
            "picurl",
            "cover",
            "cover_img_url",
            "image",
            "list_pic",
            "cover_url",
            "sizable_cover",
            "union_cover",
            "icon",
        ],
    )
    .and_then(normalize_cover_url);
    let creator_nickname = first_string(
        item,
        &[
            "nickname",
            "username",
            "user_name",
            "creator",
            "author_name",
            "list_create_username",
        ],
    )
    .filter(|value| !value.is_empty());
    let subscribed = first_bool(
        item,
        &["subscribed", "is_collect", "is_fav", "is_subscribe"],
    )
    .unwrap_or_else(|| {
        first_string(item, &["type", "list_type", "source"])
            .map(|value| value != "0")
            .unwrap_or(false)
    });

    Some(KugouPlaylistSummary {
        id: global_collection_id
            .clone()
            .unwrap_or_else(|| list_id.clone()),
        list_id,
        global_collection_id,
        name,
        track_count: first_u64(
            item,
            &[
                "count",
                "total",
                "song_count",
                "file_count",
                "total_count",
                "songcount",
                "music_count",
                "musiccount",
                "filecount",
            ],
        )
        .unwrap_or_default(),
        cover_img_url,
        creator_nickname,
        subscribed,
        update_time: first_u64(
            item,
            &[
                "update_time",
                "updateTime",
                "modify_time",
                "modifyTime",
                "mtime",
                "create_time",
                "createTime",
                "ctime",
                "add_time",
                "addTime",
                "addtime",
                "list_create_time",
                "listCreateTime",
            ],
        ),
    })
}

fn parse_kugou_track(item: &Value) -> Option<KugouSearchTrack> {
    parse_search_track(item).or_else(|| {
        [
            "audio_info",
            "audio",
            "song",
            "track",
            "file",
            "music",
            "base",
            "info",
        ]
        .iter()
        .find_map(|key| item.get(*key).and_then(parse_search_track))
    })
}

fn parse_search_track(item: &Value) -> Option<KugouSearchTrack> {
    let hash_candidates = extract_hash_candidates(item);
    let hash = hash_candidates.first()?.clone();
    if hash.is_empty() {
        return None;
    }

    let filename =
        first_string(item, &["filename", "file_name", "fileName", "name"]).unwrap_or_default();
    let raw_name = first_string(
        item,
        &[
            "songname",
            "song_name",
            "songname_original",
            "audio_name",
            "audioname",
            "name",
            "title",
            "filename",
        ],
    )
    .unwrap_or_else(|| filename.clone());
    let name = strip_html_tags(&raw_name).trim().to_string();
    let singer = first_string(
        item,
        &[
            "singername",
            "singer_name",
            "author_name",
            "author",
            "artist",
            "artists",
            "singer",
            "singers",
        ],
    )
    .or_else(|| split_artist_from_filename(&filename))
    .unwrap_or_else(|| "未知歌手".to_string());
    let artists = split_artists(&strip_html_tags(&singer));
    let album = first_string(
        item,
        &[
            "album_name",
            "albumname",
            "album",
            "albumName",
            "albumtitle",
        ],
    )
    .map(|value| strip_html_tags(&value).trim().to_string())
    .filter(|value| !value.is_empty());
    let duration_ms = first_u64(
        item,
        &[
            "duration",
            "timelength",
            "timelen",
            "time_length",
            "duration_ms",
            "durationms",
            "filetime",
        ],
    )
    .filter(|value| *value > 0)
    .map(|value| if value > 10_000 { value } else { value * 1000 });
    let cover_img_url = item
        .get("trans_param")
        .and_then(|value| value_as_string(value, "union_cover"))
        .or_else(|| {
            first_string(
                item,
                &[
                    "cover",
                    "cover_img_url",
                    "image",
                    "img",
                    "imgurl",
                    "albumimg",
                    "pic",
                    "picurl",
                    "album_img",
                ],
            )
        })
        .and_then(normalize_cover_url);
    let album_id =
        first_string(item, &["album_id", "albumid", "albumID"]).filter(|value| !value.is_empty());
    let album_audio_id = first_u64(
        item,
        &[
            "album_audio_id",
            "albumaudioid",
            "mixsongid",
            "emixsongid",
            "mix_id",
            "mixid",
        ],
    );
    let audio_id = first_u64(
        item,
        &[
            "audio_id",
            "audioid",
            "mixsongid",
            "emixsongid",
            "fileid",
            "file_id",
            "id",
        ],
    );
    let id = album_audio_id
        .map(|value| value.to_string())
        .or_else(|| audio_id.map(|value| value.to_string()))
        .unwrap_or_else(|| hash.clone());

    Some(KugouSearchTrack {
        id,
        hash,
        hash_candidates,
        name: if name.is_empty() {
            "酷狗歌曲".to_string()
        } else {
            name
        },
        artists,
        album,
        duration_ms,
        cover_img_url,
        album_id,
        album_audio_id,
        audio_id,
        privilege: first_u64(item, &["privilege", "privilege2", "privilege_type"]),
        pay_type: first_u64(item, &["pay_type", "paytype", "payType"]),
    })
}

fn extract_hash_candidates(item: &Value) -> Vec<String> {
    let mut hashes = Vec::new();
    collect_hash_candidates_from_value(item, &mut hashes);
    if let Some(trans_param) = field_value(item, "trans_param") {
        collect_hash_candidates_from_value(trans_param, &mut hashes);
    }
    for nested_key in [
        "audio_info",
        "audio",
        "song",
        "track",
        "file",
        "music",
        "base",
        "info",
    ] {
        if let Some(nested) = field_value(item, nested_key) {
            collect_hash_candidates_from_value(nested, &mut hashes);
        }
    }
    hashes
}

fn collect_hash_candidates_from_value(value: &Value, hashes: &mut Vec<String>) {
    for key in [
        "hash",
        "file_hash",
        "filehash",
        "audio_hash",
        "audiohash",
        "song_hash",
        "songhash",
        "hash128",
        "128hash",
        "hash_128",
        "filehash128",
        "128filehash",
        "hash320",
        "320hash",
        "hash_320",
        "filehash320",
        "320filehash",
        "hqhash",
        "hq_hash",
        "hqfilehash",
        "hq_file_hash",
        "sqhash",
        "sq_hash",
        "sqfilehash",
        "sq_file_hash",
        "res_hash",
        "reshash",
        "resfilehash",
        "res_file_hash",
        "superhash",
        "super_hash",
        "superfilehash",
        "super_file_hash",
        "ogg_hash",
        "oggfilehash",
        "ogg_file_hash",
        "ogg128hash",
        "ogg_128_hash",
        "ogg320hash",
        "ogg_320_hash",
    ] {
        if let Some(value) = value_as_string(value, key) {
            push_unique_hash_candidate(hashes, &value);
        }
    }
}

fn describe_response_shape(value: &Value) -> String {
    let status = value_as_i64(value, "status").unwrap_or_default();
    let code = value_as_i64(value, "code").unwrap_or_default();
    let error_code = value_as_i64(value, "error_code").unwrap_or_default();
    let top_keys = object_keys_summary(value);
    let data_keys = value
        .get("data")
        .map(object_keys_summary)
        .unwrap_or_else(|| "无 data 对象".to_string());
    let info_shape = value_at_path(value, &["data", "info"])
        .map(value_shape_summary)
        .unwrap_or_else(|| "无 data.info".to_string());
    let snap_shape = value_at_path(value, &["data", "snap"])
        .map(value_shape_summary)
        .unwrap_or_else(|| "无 data.snap".to_string());

    format!(
        "status={status}, code={code}, error_code={error_code}, 顶层字段={top_keys}, data字段={data_keys}, data.info={info_shape}, data.snap={snap_shape}"
    )
}

fn object_keys_summary(value: &Value) -> String {
    let Some(map) = value.as_object() else {
        return "非对象".to_string();
    };
    if map.is_empty() {
        return "空对象".to_string();
    }

    let mut keys = map.keys().take(12).cloned().collect::<Vec<_>>();
    keys.sort();
    if map.len() > keys.len() {
        keys.push("...".to_string());
    }
    keys.join(",")
}

fn value_shape_summary(value: &Value) -> String {
    match value {
        Value::Array(items) => {
            let first_item = items
                .first()
                .map(object_keys_summary)
                .unwrap_or_else(|| "无首项".to_string());
            format!("数组，长度={}，首项字段={first_item}", items.len())
        }
        Value::Object(_) => format!("对象，字段={}", object_keys_summary(value)),
        Value::String(text) => {
            let trimmed = text.trim();
            let looks_like_json = trimmed.starts_with('{') || trimmed.starts_with('[');
            format!("字符串，长度={}，疑似JSON={looks_like_json}", text.len())
        }
        Value::Number(_) => "数字".to_string(),
        Value::Bool(_) => "布尔值".to_string(),
        Value::Null => "null".to_string(),
    }
}

fn http_get_json(url: &str) -> Result<Value, String> {
    let client = Client::builder()
        .user_agent(KUGOU_USER_AGENT)
        .timeout(Duration::from_secs(18))
        .build()
        .map_err(|err| format!("创建酷狗请求客户端失败：{err}"))?;
    let response = client
        .get(url)
        .header(REFERER, "https://www.kugou.com/")
        .send()
        .map_err(|err| format!("请求酷狗接口失败：{err}"))?;
    let status = response.status();
    let text = response
        .text()
        .map_err(|err| format!("读取酷狗接口响应失败：{err}"))?;
    if !status.is_success() {
        return Err(format!("酷狗接口返回 HTTP {status}"));
    }

    serde_json::from_str(&text).map_err(|err| format!("酷狗接口 JSON 解析失败：{err}"))
}

fn normalize_query(value: &str, max_chars: usize) -> Result<String, String> {
    normalize_optional_query(value, max_chars).ok_or_else(|| "请输入酷狗搜索关键词。".to_string())
}

fn normalize_optional_query(value: &str, max_chars: usize) -> Option<String> {
    let normalized = value.split_whitespace().collect::<Vec<_>>().join(" ");
    let normalized = normalized.trim();
    if normalized.is_empty() {
        return None;
    }

    Some(normalized.chars().take(max_chars).collect())
}

fn normalize_hash(value: &str) -> Result<String, String> {
    let hash = value.trim();
    if hash.len() < 16 || hash.len() > 64 || !hash.chars().all(|char| char.is_ascii_hexdigit()) {
        return Err("酷狗歌曲 hash 无效。".to_string());
    }

    Ok(hash.to_ascii_uppercase())
}

fn normalize_hash_candidates(
    hash: String,
    hash_candidates: Option<Vec<String>>,
) -> Result<Vec<String>, String> {
    let mut hashes = Vec::new();
    push_unique_hash_candidate(&mut hashes, &normalize_hash(&hash)?);

    if let Some(candidates) = hash_candidates {
        for candidate in candidates {
            push_unique_hash_candidate(&mut hashes, &candidate);
        }
    }

    if hashes.is_empty() {
        return Err("酷狗歌曲 hash 无效。".to_string());
    }

    Ok(hashes)
}

fn push_unique_hash_candidate(hashes: &mut Vec<String>, value: &str) {
    let Ok(hash) = normalize_hash(value) else {
        return;
    };
    if !hashes.iter().any(|candidate| candidate == &hash) {
        hashes.push(hash);
    }
}

fn normalize_list_id(value: &str) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty() || value.len() > 128 {
        return Err("酷狗歌单 ID 无效。".to_string());
    }
    if !value
        .chars()
        .all(|char| char.is_ascii_alphanumeric() || matches!(char, '_' | '-' | ':'))
    {
        return Err("酷狗歌单 ID 包含不支持的字符。".to_string());
    }

    Ok(value.to_string())
}

fn first_string(value: &Value, keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|key| value_as_string(value, key))
}

fn first_u64(value: &Value, keys: &[&str]) -> Option<u64> {
    keys.iter().find_map(|key| value_as_u64(value, key))
}

fn first_bool(value: &Value, keys: &[&str]) -> Option<bool> {
    keys.iter().find_map(|key| value_as_bool(value, key))
}

fn value_as_string(value: &Value, key: &str) -> Option<String> {
    let value = field_value(value, key)?;
    if let Some(text) = value.as_str() {
        return Some(text.trim().to_string());
    }

    if let Some(number) = value.as_u64() {
        return Some(number.to_string());
    }

    if let Some(number) = value.as_i64() {
        return Some(number.to_string());
    }

    None
}

fn value_as_bool(value: &Value, key: &str) -> Option<bool> {
    match field_value(value, key)? {
        Value::Bool(value) => Some(*value),
        Value::Number(value) => value.as_u64().map(|number| number != 0),
        Value::String(value) => match value.trim().to_lowercase().as_str() {
            "true" | "1" | "yes" => Some(true),
            "false" | "0" | "no" => Some(false),
            _ => None,
        },
        _ => None,
    }
}

fn value_as_u64(value: &Value, key: &str) -> Option<u64> {
    let value = field_value(value, key)?;
    if let Some(number) = value.as_u64() {
        return Some(number);
    }

    if let Some(number) = value.as_i64() {
        return u64::try_from(number).ok();
    }

    value.as_str()?.trim().parse::<u64>().ok()
}

fn value_as_i64(value: &Value, key: &str) -> Option<i64> {
    let value = field_value(value, key)?;
    if let Some(number) = value.as_i64() {
        return Some(number);
    }

    value.as_str()?.trim().parse::<i64>().ok()
}

fn field_value<'a>(value: &'a Value, key: &str) -> Option<&'a Value> {
    value.get(key).or_else(|| {
        value
            .as_object()?
            .iter()
            .find_map(|(current_key, current_value)| {
                if current_key.eq_ignore_ascii_case(key) {
                    Some(current_value)
                } else {
                    None
                }
            })
    })
}

fn strip_html_tags(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    let mut in_tag = false;

    for char in value.chars() {
        match char {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => output.push(char),
            _ => {}
        }
    }

    output
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
}

fn split_artist_from_filename(value: &str) -> Option<String> {
    value
        .split_once(" - ")
        .map(|(artist, _)| strip_html_tags(artist).trim().to_string())
        .filter(|value| !value.is_empty())
}

fn split_artists(value: &str) -> Vec<String> {
    let artists = value
        .split(['/', '、', ',', '，', '&'])
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .take(8)
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();

    if artists.is_empty() {
        vec!["未知歌手".to_string()]
    } else {
        artists
    }
}

fn normalize_cover_url(value: String) -> Option<String> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }

    let normalized = value
        .replace("{size}", "240")
        .replace("http://", "https://");
    if normalized.starts_with("https://") || normalized.starts_with("http://") {
        Some(normalized)
    } else {
        None
    }
}

fn pseudo_uuid() -> String {
    let hex = pseudo_hex(32);
    format!(
        "{}-{}-{}-{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    )
}

fn pseudo_hex(len: usize) -> String {
    const CHARS: &[u8] = b"0123456789abcdef";
    let mut seed = current_epoch_millis() as u64 ^ ((std::process::id() as u64) << 17) ^ len as u64;
    let mut output = String::with_capacity(len);
    for _ in 0..len {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        output.push(CHARS[(seed as usize) % CHARS.len()] as char);
    }
    output
}

fn pseudo_alnum(len: usize) -> String {
    const CHARS: &[u8] = b"1234567890ABCDEFGHIJKLMNOPQRSTUVWXYZ";
    let mut seed = current_epoch_millis() as u64 ^ ((std::process::id() as u64) << 11) ^ len as u64;
    let mut output = String::with_capacity(len);
    for _ in 0..len {
        seed = seed
            .wrapping_mul(2862933555777941757)
            .wrapping_add(3037000493);
        output.push(CHARS[(seed as usize) % CHARS.len()] as char);
    }
    output
}

fn current_epoch_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

fn current_epoch_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
}

fn url_encode_component(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    for byte in value.as_bytes() {
        match *byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                output.push(*byte as char)
            }
            b' ' => output.push_str("%20"),
            byte => output.push_str(&format!("%{byte:02X}")),
        }
    }
    output
}
