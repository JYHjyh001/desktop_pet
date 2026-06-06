use std::{
    collections::HashMap,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex, OnceLock,
    },
    thread::{self, JoinHandle},
    time::Duration,
};

use serde::{Deserialize, Serialize};
use tauri::AppHandle;

use crate::{
    ai_chat::{self, PetChatMessageDraft},
    app_data::{self, WechatClawbotSettings},
};

const MAX_REQUEST_BYTES: usize = 128 * 1024;

static BRIDGE_SERVER: OnceLock<Mutex<Option<BridgeServerHandle>>> = OnceLock::new();

struct BridgeServerHandle {
    stop: Arc<AtomicBool>,
    join: Option<JoinHandle<()>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ClawbotChatRequest {
    #[serde(default)]
    message: String,
    #[serde(default)]
    sender: String,
    #[serde(default)]
    session_id: String,
    #[serde(default)]
    messages: Vec<PetChatMessageDraft>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ClawbotChatResponse {
    ok: bool,
    reply: String,
    message: String,
    provider: String,
    model: String,
}

pub fn restart_bridge_server(app: &AppHandle) -> Result<(), String> {
    let settings = app_data::read_config(app)?.wechat_clawbot;
    let state = BRIDGE_SERVER.get_or_init(|| Mutex::new(None));
    let mut guard = state
        .lock()
        .map_err(|_| "ClawBot HTTP Bridge 状态锁定失败".to_string())?;

    stop_bridge_server(&mut guard);

    if !settings.bridge_enabled {
        return Ok(());
    }

    let bind_addr = format!("{}:{}", settings.bridge_host.trim(), settings.bridge_port);
    let listener =
        TcpListener::bind(&bind_addr).map_err(|err| format!("启动 ClawBot HTTP Bridge 失败：{err}"))?;
    listener
        .set_nonblocking(true)
        .map_err(|err| format!("设置 ClawBot HTTP Bridge 非阻塞模式失败：{err}"))?;

    let stop = Arc::new(AtomicBool::new(false));
    let thread_stop = Arc::clone(&stop);
    let app_handle = app.clone();
    let join = thread::spawn(move || run_bridge_server(listener, app_handle, settings, thread_stop));
    *guard = Some(BridgeServerHandle {
        stop,
        join: Some(join),
    });

    Ok(())
}

fn stop_bridge_server(handle: &mut Option<BridgeServerHandle>) {
    if let Some(mut server) = handle.take() {
        server.stop.store(true, Ordering::SeqCst);
        if let Some(join) = server.join.take() {
            let _ = join.join();
        }
    }
}

fn run_bridge_server(
    listener: TcpListener,
    app: AppHandle,
    settings: WechatClawbotSettings,
    stop: Arc<AtomicBool>,
) {
    while !stop.load(Ordering::SeqCst) {
        match listener.accept() {
            Ok((stream, _)) => {
                let app = app.clone();
                let settings = settings.clone();
                thread::spawn(move || {
                    if let Err(err) = handle_connection(stream, app, settings) {
                        eprintln!("ClawBot HTTP Bridge 请求失败：{err}");
                    }
                });
            }
            Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(100));
            }
            Err(err) => {
                eprintln!("ClawBot HTTP Bridge 监听失败：{err}");
                thread::sleep(Duration::from_millis(250));
            }
        }
    }
}

fn handle_connection(
    mut stream: TcpStream,
    app: AppHandle,
    settings: WechatClawbotSettings,
) -> Result<(), String> {
    stream
        .set_read_timeout(Some(Duration::from_secs(10)))
        .map_err(|err| err.to_string())?;

    let request = read_http_request(&mut stream)?;
    let response = route_request(request, &app, &settings);
    write_http_response(&mut stream, response)
}

struct HttpRequest {
    method: String,
    path: String,
    headers: HashMap<String, String>,
    body: Vec<u8>,
}

struct HttpResponse {
    status: u16,
    reason: &'static str,
    body: serde_json::Value,
}

fn read_http_request(stream: &mut TcpStream) -> Result<HttpRequest, String> {
    let mut buffer = Vec::new();
    let mut temp = [0_u8; 4096];
    let mut header_end = None;
    let mut content_length = 0_usize;

    loop {
        let bytes_read = stream.read(&mut temp).map_err(|err| err.to_string())?;
        if bytes_read == 0 {
            break;
        }
        buffer.extend_from_slice(&temp[..bytes_read]);
        if buffer.len() > MAX_REQUEST_BYTES {
            return Err("请求体过大".to_string());
        }

        if header_end.is_none() {
            header_end = find_header_end(&buffer);
            if let Some(index) = header_end {
                let headers_text = String::from_utf8_lossy(&buffer[..index]);
                content_length = parse_content_length(&headers_text)?;
            }
        }

        if let Some(index) = header_end {
            let body_start = index + 4;
            if buffer.len().saturating_sub(body_start) >= content_length {
                break;
            }
        }
    }

    let header_end = header_end.ok_or_else(|| "HTTP 请求头不完整".to_string())?;
    let headers_text = String::from_utf8_lossy(&buffer[..header_end]);
    let mut lines = headers_text.lines();
    let request_line = lines
        .next()
        .ok_or_else(|| "HTTP 请求行缺失".to_string())?;
    let mut request_parts = request_line.split_whitespace();
    let method = request_parts.next().unwrap_or_default().to_string();
    let path = request_parts.next().unwrap_or_default().to_string();

    let headers = lines
        .filter_map(|line| {
            let (name, value) = line.split_once(':')?;
            Some((name.trim().to_ascii_lowercase(), value.trim().to_string()))
        })
        .collect::<HashMap<_, _>>();

    let body_start = header_end + 4;
    let body_end = body_start + content_length;
    let body = buffer
        .get(body_start..body_end)
        .ok_or_else(|| "HTTP 请求体不完整".to_string())?
        .to_vec();

    Ok(HttpRequest {
        method,
        path,
        headers,
        body,
    })
}

fn find_header_end(buffer: &[u8]) -> Option<usize> {
    buffer.windows(4).position(|window| window == b"\r\n\r\n")
}

fn parse_content_length(headers_text: &str) -> Result<usize, String> {
    for line in headers_text.lines().skip(1) {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        if name.trim().eq_ignore_ascii_case("content-length") {
            return value
                .trim()
                .parse::<usize>()
                .map_err(|_| "Content-Length 无效".to_string());
        }
    }

    Ok(0)
}

fn route_request(
    request: HttpRequest,
    app: &AppHandle,
    settings: &WechatClawbotSettings,
) -> HttpResponse {
    let bridge_path = normalize_bridge_path(&settings.bridge_path);

    if request.method == "OPTIONS" {
        return json_response(204, "No Content", serde_json::Value::Null);
    }

    if request.method != "POST" || request.path != bridge_path {
        return json_response(
            404,
            "Not Found",
            serde_json::json!({ "ok": false, "error": "接口不存在" }),
        );
    }

    if !authorized(&request.headers, &settings.bridge_token) {
        return json_response(
            401,
            "Unauthorized",
            serde_json::json!({ "ok": false, "error": "ClawBot Bridge Token 不正确" }),
        );
    }

    let chat_request = match serde_json::from_slice::<ClawbotChatRequest>(&request.body) {
        Ok(value) => value,
        Err(err) => {
            return json_response(
                400,
                "Bad Request",
                serde_json::json!({ "ok": false, "error": format!("JSON 格式错误：{err}") }),
            )
        }
    };

    match handle_chat_request(app, chat_request) {
        Ok(response) => json_response(
            200,
            "OK",
            serde_json::to_value(response).unwrap_or_else(|_| serde_json::json!({ "ok": true })),
        ),
        Err(err) => json_response(
            500,
            "Internal Server Error",
            serde_json::json!({ "ok": false, "error": err }),
        ),
    }
}

fn handle_chat_request(
    app: &AppHandle,
    request: ClawbotChatRequest,
) -> Result<ClawbotChatResponse, String> {
    let mut messages = request.messages;
    let incoming = request.message.trim().to_string();

    if !incoming.is_empty() {
        messages.push(PetChatMessageDraft {
            role: "user".to_string(),
            content: incoming,
            created_at: app_data::now_seconds(),
            time_context: build_time_context(&request.sender, &request.session_id),
        });
    }

    if messages.is_empty() {
        return Err("请求中缺少 message 或 messages".to_string());
    }

    let reply = ai_chat::send_pet_chat_message(app, messages)?;
    Ok(ClawbotChatResponse {
        ok: true,
        reply: reply.message.clone(),
        message: reply.message,
        provider: reply.provider,
        model: reply.model,
    })
}

fn build_time_context(sender: &str, session_id: &str) -> String {
    let mut parts = Vec::new();
    if !sender.trim().is_empty() {
        parts.push(format!("微信发送者：{}", sender.trim()));
    }
    if !session_id.trim().is_empty() {
        parts.push(format!("微信会话：{}", session_id.trim()));
    }

    parts.join("；")
}

fn authorized(headers: &HashMap<String, String>, token: &str) -> bool {
    let token = token.trim();
    if token.is_empty() {
        return true;
    }

    let expected = format!("Bearer {token}");
    headers
        .get("authorization")
        .map(|value| value.trim() == expected)
        .unwrap_or(false)
        || headers
            .get("x-petdrawer-token")
            .map(|value| value.trim() == token)
            .unwrap_or(false)
}

fn normalize_bridge_path(path: &str) -> String {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return "/clawbot/chat".to_string();
    }

    if trimmed.starts_with('/') {
        trimmed.to_string()
    } else {
        format!("/{trimmed}")
    }
}

fn json_response(status: u16, reason: &'static str, body: serde_json::Value) -> HttpResponse {
    HttpResponse {
        status,
        reason,
        body,
    }
}

fn write_http_response(stream: &mut TcpStream, response: HttpResponse) -> Result<(), String> {
    let body = if response.status == 204 {
        Vec::new()
    } else {
        serde_json::to_vec(&response.body).map_err(|err| err.to_string())?
    };
    let headers = format!(
        "HTTP/1.1 {} {}\r\nContent-Type: application/json; charset=utf-8\r\nContent-Length: {}\r\nAccess-Control-Allow-Origin: *\r\nAccess-Control-Allow-Headers: Authorization, Content-Type, X-PetDrawer-Token\r\nAccess-Control-Allow-Methods: POST, OPTIONS\r\nConnection: close\r\n\r\n",
        response.status,
        response.reason,
        body.len()
    );

    stream
        .write_all(headers.as_bytes())
        .and_then(|_| stream.write_all(&body))
        .map_err(|err| err.to_string())
}
