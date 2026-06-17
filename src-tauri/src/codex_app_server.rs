use std::{
    collections::HashMap,
    fs,
    io::{BufRead, BufReader, Read, Seek, SeekFrom, Write},
    net::{TcpListener, TcpStream},
    path::{Path, PathBuf},
    process::{Child, ChildStderr, ChildStdin, ChildStdout, Command, Stdio},
    sync::{mpsc, Arc, Mutex},
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tauri::{AppHandle, Emitter};
use tungstenite::{stream::MaybeTlsStream, Message, WebSocket};

use crate::app_data::CodexAppServerSettings;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(12);
const SOCKET_READ_TIMEOUT: Duration = Duration::from_millis(250);
const TRANSPORT_POLL_INTERVAL: Duration = Duration::from_millis(50);
const SESSION_LOG_POLL_INTERVAL: Duration = Duration::from_millis(900);
const STATUS_EVENT: &str = "codex-status-updated";
const PET_ANIMATION_EVENT: &str = "pet-animation-state";
const CODEX_TASK_LIMIT: usize = 20;
const CODEX_TASK_RETENTION_SECONDS: u64 = 30 * 60;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexStatusPayload {
    pub state: String,
    pub message: String,
    #[serde(default)]
    pub active: bool,
    pub thread_id: Option<String>,
    pub turn_id: Option<String>,
    pub endpoint: Option<String>,
    pub mode: Option<String>,
    pub last_event: Option<String>,
    pub error: Option<String>,
    pub notify: bool,
    pub updated_at: u64,
    #[serde(default)]
    pub summary: CodexStatusSummaryPayload,
    #[serde(default)]
    pub tasks: Vec<CodexTaskStatusPayload>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexStatusSummaryPayload {
    pub state: String,
    pub message: String,
    pub attention: String,
    pub total_count: usize,
    pub active_count: usize,
    pub running_count: usize,
    pub review_count: usize,
    pub waiting_count: usize,
    pub completed_count: usize,
    pub failed_count: usize,
    pub unread_count: usize,
    pub unread_completed_count: usize,
    pub unread_failed_count: usize,
    pub badge_label: Option<String>,
}

impl Default for CodexStatusSummaryPayload {
    fn default() -> Self {
        Self {
            state: "disconnected".to_string(),
            message: "Codex App Server 未连接".to_string(),
            attention: "none".to_string(),
            total_count: 0,
            active_count: 0,
            running_count: 0,
            review_count: 0,
            waiting_count: 0,
            completed_count: 0,
            failed_count: 0,
            unread_count: 0,
            unread_completed_count: 0,
            unread_failed_count: 0,
            badge_label: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexTaskStatusPayload {
    pub id: String,
    pub label: String,
    pub state: String,
    pub message: String,
    pub mode: Option<String>,
    pub last_event: Option<String>,
    pub updated_at: u64,
    pub unread: bool,
}

impl Default for CodexStatusPayload {
    fn default() -> Self {
        Self {
            state: "disconnected".to_string(),
            message: "Codex App Server 未连接".to_string(),
            active: false,
            thread_id: None,
            turn_id: None,
            endpoint: None,
            mode: None,
            last_event: None,
            error: None,
            notify: false,
            updated_at: now_seconds(),
            summary: CodexStatusSummaryPayload::default(),
            tasks: Vec::new(),
        }
    }
}

#[derive(Debug)]
struct CodexRuntime {
    status: CodexStatusPayload,
    tasks: HashMap<String, CodexTaskStatus>,
    next_task_label: u64,
    control: Option<mpsc::Sender<CodexControl>>,
    worker_id: u64,
}

impl Default for CodexRuntime {
    fn default() -> Self {
        Self {
            status: CodexStatusPayload::default(),
            tasks: HashMap::new(),
            next_task_label: 1,
            control: None,
            worker_id: 0,
        }
    }
}

#[derive(Clone, Default)]
pub struct CodexAppServerState {
    runtime: Arc<Mutex<CodexRuntime>>,
}

#[derive(Debug, Clone)]
struct CodexTaskStatus {
    label: String,
    state: String,
    message: String,
    mode: Option<String>,
    last_event: Option<String>,
    updated_at: u64,
    unread: bool,
}

#[derive(Debug)]
enum CodexControl {
    StartTurn { prompt: String, cwd: Option<String> },
    Stop,
}

#[derive(Debug, Clone)]
struct PendingTurn {
    prompt: String,
    cwd: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ConnectionMode {
    Proxy,
    Managed,
    SessionLog,
}

impl ConnectionMode {
    fn from_settings(settings: &CodexAppServerSettings) -> Self {
        match settings.mode.as_str() {
            "managed" => Self::Managed,
            "sessionLog" => Self::SessionLog,
            _ => Self::Proxy,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Proxy => "proxy",
            Self::Managed => "managed",
            Self::SessionLog => "sessionLog",
        }
    }

    fn endpoint(self, port: u16) -> String {
        match self {
            Self::Proxy => "codex app-server proxy".to_string(),
            Self::Managed => format!("ws://127.0.0.1:{port}"),
            Self::SessionLog => "~/.codex/sessions/*.jsonl".to_string(),
        }
    }

    fn starting_message(self) -> &'static str {
        match self {
            Self::Proxy => "正在连接当前 Codex",
            Self::Managed => "正在启动独立 Codex App Server",
            Self::SessionLog => "正在监听 Codex 会话日志",
        }
    }

    fn failed_message(self) -> &'static str {
        match self {
            Self::Proxy => "当前 Codex 连接失败",
            Self::Managed => "Codex App Server 连接失败",
            Self::SessionLog => "Codex 会话日志监听失败",
        }
    }

    fn stopped_message(self) -> &'static str {
        match self {
            Self::SessionLog => "Codex 会话日志监听已停止",
            _ => "Codex App Server 已停止",
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum RequestKind {
    Initialize,
    ThreadLoadedList,
    ThreadResume,
    ThreadStart,
    TurnStart,
}

enum TransportEvent {
    Message(String),
    Error(String),
    Closed,
}

enum TransportRead {
    Message(String),
    NoMessage,
    Closed,
}

type CodexSocket = WebSocket<MaybeTlsStream<TcpStream>>;

enum CodexConnection {
    WebSocket(CodexSocket),
    Stdio {
        stdin: ChildStdin,
        events: mpsc::Receiver<TransportEvent>,
    },
}

impl CodexConnection {
    fn send_json(&mut self, payload: Value) -> Result<(), String> {
        match self {
            Self::WebSocket(socket) => socket
                .send(Message::Text(payload.to_string()))
                .map_err(|err| err.to_string()),
            Self::Stdio { stdin, .. } => {
                stdin
                    .write_all(payload.to_string().as_bytes())
                    .map_err(|err| err.to_string())?;
                stdin.write_all(b"\n").map_err(|err| err.to_string())?;
                stdin.flush().map_err(|err| err.to_string())
            }
        }
    }

    fn read_event(&mut self) -> Result<TransportRead, String> {
        match self {
            Self::WebSocket(socket) => match socket.read() {
                Ok(message) => match message.into_text() {
                    Ok(text) => Ok(TransportRead::Message(text)),
                    Err(_) => Ok(TransportRead::NoMessage),
                },
                Err(tungstenite::Error::Io(err))
                    if err.kind() == std::io::ErrorKind::WouldBlock
                        || err.kind() == std::io::ErrorKind::TimedOut =>
                {
                    Ok(TransportRead::NoMessage)
                }
                Err(tungstenite::Error::ConnectionClosed) => Ok(TransportRead::Closed),
                Err(err) => Err(err.to_string()),
            },
            Self::Stdio { events, .. } => match events.try_recv() {
                Ok(TransportEvent::Message(text)) => Ok(TransportRead::Message(text)),
                Ok(TransportEvent::Error(err)) => Err(err),
                Ok(TransportEvent::Closed) => Ok(TransportRead::Closed),
                Err(mpsc::TryRecvError::Empty) => Ok(TransportRead::NoMessage),
                Err(mpsc::TryRecvError::Disconnected) => Ok(TransportRead::Closed),
            },
        }
    }

    fn close(&mut self) {
        match self {
            Self::WebSocket(socket) => {
                let _ = socket.close(None);
            }
            Self::Stdio { stdin, .. } => {
                let _ = stdin.flush();
            }
        }
    }
}

pub fn get_status(state: &CodexAppServerState) -> CodexStatusPayload {
    state
        .runtime
        .lock()
        .map(|mut runtime| {
            prune_codex_tasks(&mut runtime);
            rebuild_runtime_status(&mut runtime)
        })
        .unwrap_or_default()
}

pub fn start(
    app: AppHandle,
    state: &CodexAppServerState,
    settings: CodexAppServerSettings,
) -> Result<CodexStatusPayload, String> {
    let command = settings.command.trim();
    let mode = ConnectionMode::from_settings(&settings);
    if mode != ConnectionMode::SessionLog && command.is_empty() {
        return Err("Codex 命令不能为空。".to_string());
    }

    let port = if mode == ConnectionMode::Managed {
        if settings.port == 0 {
            pick_available_port()?
        } else {
            settings.port
        }
    } else {
        0
    };
    let endpoint = mode.endpoint(port);

    let mut runtime = state
        .runtime
        .lock()
        .map_err(|_| "Codex 状态锁定失败".to_string())?;

    if runtime.control.is_some() {
        return Ok(rebuild_runtime_status(&mut runtime));
    }

    let (tx, rx) = mpsc::channel();
    runtime.worker_id = runtime.worker_id.wrapping_add(1);
    let worker_id = runtime.worker_id;
    runtime.control = Some(tx);
    runtime.tasks.clear();
    runtime.next_task_label = 1;
    runtime.status = CodexStatusPayload {
        state: "starting".to_string(),
        message: mode.starting_message().to_string(),
        active: true,
        thread_id: None,
        turn_id: None,
        endpoint: Some(endpoint.clone()),
        mode: Some(mode.as_str().to_string()),
        last_event: Some("start".to_string()),
        error: None,
        notify: true,
        updated_at: now_seconds(),
        summary: CodexStatusSummaryPayload {
            state: "starting".to_string(),
            message: mode.starting_message().to_string(),
            attention: "working".to_string(),
            ..CodexStatusSummaryPayload::default()
        },
        tasks: Vec::new(),
    };
    let payload = rebuild_runtime_status(&mut runtime);
    drop(runtime);
    emit_status(&app, &payload);

    let runtime = Arc::clone(&state.runtime);
    thread::spawn(move || {
        run_worker(app, runtime, worker_id, settings, port, endpoint, rx);
    });

    Ok(payload)
}

pub fn stop(app: AppHandle, state: &CodexAppServerState) -> Result<CodexStatusPayload, String> {
    let (control, current_mode) = {
        let runtime = state
            .runtime
            .lock()
            .map_err(|_| "Codex 状态锁定失败".to_string())?;
        (runtime.control.clone(), runtime.status.mode.clone())
    };

    if let Some(control) = control {
        let _ = control.send(CodexControl::Stop);
    }

    let message = if current_mode.as_deref() == Some("sessionLog") {
        "Codex 会话日志监听已停止"
    } else {
        "Codex App Server 已断开"
    };
    let payload = update_status(
        &app,
        &state.runtime,
        StatusPatch {
            state: Some("disconnected"),
            message: Some(message),
            last_event: Some("stop"),
            error: None,
            clear_thread: false,
            clear_endpoint: true,
            clear_tasks: true,
            animation: Some(("idle", 0, "idle")),
            ..StatusPatch::default()
        },
    );
    Ok(payload)
}

pub fn ack_notifications(
    app: AppHandle,
    state: &CodexAppServerState,
) -> Result<CodexStatusPayload, String> {
    let payload = {
        let mut runtime = state
            .runtime
            .lock()
            .map_err(|_| "Codex 状态锁定失败".to_string())?;
        for task in runtime.tasks.values_mut() {
            task.unread = false;
        }
        runtime.status.notify = false;
        runtime.status.updated_at = now_seconds();
        rebuild_runtime_status(&mut runtime)
    };
    emit_status(&app, &payload);
    Ok(payload)
}

pub fn start_turn(
    app: AppHandle,
    state: &CodexAppServerState,
    settings: CodexAppServerSettings,
    prompt: String,
    cwd: Option<String>,
) -> Result<CodexStatusPayload, String> {
    if ConnectionMode::from_settings(&settings) != ConnectionMode::Managed {
        return Err(
            "测试任务只在独立测试模式可用；监听当前 Codex 或会话日志时，请直接在 Codex 中开始任务。"
                .to_string(),
        );
    }

    let prompt = prompt.trim().to_string();
    if prompt.is_empty() {
        return Err("Codex 任务内容不能为空。".to_string());
    }

    start(app.clone(), state, settings)?;
    let control = {
        let runtime = state
            .runtime
            .lock()
            .map_err(|_| "Codex 状态锁定失败".to_string())?;
        if runtime.status.mode.as_deref() != Some("managed") {
            return Err("请先断开当前 Codex 监听，再切换到独立测试模式。".to_string());
        }
        runtime.control.clone()
    }
    .ok_or_else(|| "Codex App Server 尚未启动。".to_string())?;

    control
        .send(CodexControl::StartTurn {
            prompt,
            cwd: cwd.and_then(|value| {
                let trimmed = value.trim().to_string();
                (!trimmed.is_empty()).then_some(trimmed)
            }),
        })
        .map_err(|_| "Codex App Server 连接已关闭。".to_string())?;

    let payload = update_status(
        &app,
        &state.runtime,
        StatusPatch {
            state: Some("running"),
            message: Some("Codex 任务已提交"),
            last_event: Some("turn/queued"),
            error: None,
            animation: Some(("running", 0, "running")),
            ..StatusPatch::default()
        },
    );
    Ok(payload)
}

fn run_worker(
    app: AppHandle,
    runtime: Arc<Mutex<CodexRuntime>>,
    worker_id: u64,
    settings: CodexAppServerSettings,
    port: u16,
    endpoint: String,
    rx: mpsc::Receiver<CodexControl>,
) {
    let mode = ConnectionMode::from_settings(&settings);
    let mut child: Option<Child> = None;
    let result = run_worker_inner(
        &app, &runtime, &settings, mode, port, &endpoint, &rx, &mut child,
    );

    if let Some(mut child) = child {
        let _ = child.kill();
        let _ = child.wait();
    }

    if let Err(err) = result {
        update_status(
            &app,
            &runtime,
            StatusPatch {
                state: Some("failed"),
                message: Some(mode.failed_message()),
                last_event: Some("worker/error"),
                error: Some(sanitize_message(&err)),
                animation: Some(("failed", 3000, "idle")),
                ..StatusPatch::default()
            },
        );
    }

    let payload = {
        let mut guard = match runtime.lock() {
            Ok(guard) => guard,
            Err(_) => return,
        };
        if guard.worker_id != worker_id {
            return;
        }
        guard.control = None;
        guard.status.active = false;
        guard.tasks.clear();
        if guard.status.state != "failed" && guard.status.state != "disconnected" {
            guard.status.state = "disconnected".to_string();
            guard.status.message = mode.stopped_message().to_string();
            guard.status.last_event = Some("worker/stopped".to_string());
            guard.status.updated_at = now_seconds();
        }
        rebuild_runtime_status(&mut guard)
    };
    emit_status(&app, &payload);
}

fn run_worker_inner(
    app: &AppHandle,
    runtime: &Arc<Mutex<CodexRuntime>>,
    settings: &CodexAppServerSettings,
    mode: ConnectionMode,
    port: u16,
    endpoint: &str,
    rx: &mpsc::Receiver<CodexControl>,
    child: &mut Option<Child>,
) -> Result<(), String> {
    if mode == ConnectionMode::SessionLog {
        return run_session_log_worker(
            app,
            runtime,
            settings.completion_notifications_enabled,
            endpoint,
            rx,
        );
    }

    let mut connection = open_connection(settings, mode, port, endpoint, child)?;

    let mut next_request_id = 1_u64;
    let mut requests: HashMap<u64, RequestKind> = HashMap::new();
    let mut thread_id: Option<String> = None;
    let mut pending_turn: Option<PendingTurn> = None;

    send_request(
        &mut connection,
        &mut next_request_id,
        &mut requests,
        RequestKind::Initialize,
        "initialize",
        json!({
            "clientInfo": {
                "name": "pet-drawer",
                "title": "PetDrawer",
                "version": env!("CARGO_PKG_VERSION")
            },
            "capabilities": {
                "experimentalApi": true
            }
        }),
    )?;

    update_status(
        app,
        runtime,
        StatusPatch {
            state: Some("connected"),
            message: Some(if mode == ConnectionMode::Proxy {
                "已连接当前 Codex，正在初始化监听"
            } else {
                "Codex App Server 已连接"
            }),
            endpoint: Some(endpoint.to_string()),
            mode: Some(mode.as_str()),
            last_event: Some("initialize"),
            error: None,
            animation: Some(("waving", 1600, "idle")),
            ..StatusPatch::default()
        },
    );

    loop {
        while let Ok(control) = rx.try_recv() {
            match control {
                CodexControl::Stop => {
                    connection.close();
                    return Ok(());
                }
                CodexControl::StartTurn { prompt, cwd } => {
                    if mode != ConnectionMode::Managed {
                        continue;
                    }
                    let turn = PendingTurn { prompt, cwd };
                    if let Some(active_thread_id) = thread_id.clone() {
                        send_turn_start(
                            &mut connection,
                            &mut next_request_id,
                            &mut requests,
                            &active_thread_id,
                            turn,
                        )?;
                    } else {
                        pending_turn = Some(turn.clone());
                        send_thread_start(
                            &mut connection,
                            &mut next_request_id,
                            &mut requests,
                            turn.cwd.clone(),
                        )?;
                    }
                }
            }
        }

        match connection.read_event()? {
            TransportRead::Message(text) => {
                handle_server_message(
                    app,
                    runtime,
                    &mut connection,
                    &mut next_request_id,
                    &mut requests,
                    &mut thread_id,
                    &mut pending_turn,
                    mode,
                    settings.completion_notifications_enabled,
                    &text,
                )?;
            }
            TransportRead::NoMessage => {
                thread::sleep(TRANSPORT_POLL_INTERVAL);
            }
            TransportRead::Closed => return Ok(()),
        }
    }
}

#[derive(Debug, Default)]
struct SessionLogFileState {
    offset: u64,
    completed_seen: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SessionLogSignalKind {
    Running,
    Review,
    Waiting,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Copy)]
struct SessionLogSignal {
    kind: SessionLogSignalKind,
    last_event: &'static str,
    message: &'static str,
}

fn run_session_log_worker(
    app: &AppHandle,
    runtime: &Arc<Mutex<CodexRuntime>>,
    completion_notifications_enabled: bool,
    endpoint: &str,
    rx: &mpsc::Receiver<CodexControl>,
) -> Result<(), String> {
    let sessions_dir = default_codex_sessions_dir()?;
    if !sessions_dir.exists() {
        return Err(
            "未找到 Codex 会话日志目录；请先启动一次 Codex Desktop 或 Codex CLI。".to_string(),
        );
    }

    let mut files = HashMap::new();
    seed_session_log_offsets(&sessions_dir, &mut files)?;

    update_status(
        app,
        runtime,
        StatusPatch {
            state: Some("connected"),
            message: Some("已开始监听 Codex 会话日志"),
            endpoint: Some(endpoint.to_string()),
            mode: Some(ConnectionMode::SessionLog.as_str()),
            last_event: Some("sessionLog/connected"),
            error: None,
            animation: Some(("waving", 1600, "idle")),
            ..StatusPatch::default()
        },
    );

    loop {
        while let Ok(control) = rx.try_recv() {
            match control {
                CodexControl::Stop => return Ok(()),
                CodexControl::StartTurn { .. } => {}
            }
        }

        scan_session_logs(
            app,
            runtime,
            &sessions_dir,
            &mut files,
            completion_notifications_enabled,
        )?;
        thread::sleep(SESSION_LOG_POLL_INTERVAL);
    }
}

fn seed_session_log_offsets(
    root: &Path,
    files: &mut HashMap<PathBuf, SessionLogFileState>,
) -> Result<(), String> {
    for path in collect_session_log_files(root)? {
        let offset = fs::metadata(&path)
            .map(|metadata| metadata.len())
            .unwrap_or(0);
        files.insert(
            path,
            SessionLogFileState {
                offset,
                completed_seen: false,
            },
        );
    }
    Ok(())
}

fn scan_session_logs(
    app: &AppHandle,
    runtime: &Arc<Mutex<CodexRuntime>>,
    root: &Path,
    files: &mut HashMap<PathBuf, SessionLogFileState>,
    completion_notifications_enabled: bool,
) -> Result<(), String> {
    for path in collect_session_log_files(root)? {
        let state = files.entry(path.clone()).or_default();
        let lines = read_new_complete_session_log_lines(&path, state)?;
        for line in lines {
            if let Some(signal) = classify_session_log_line(&line) {
                apply_session_log_signal(
                    app,
                    runtime,
                    &path,
                    state,
                    signal,
                    completion_notifications_enabled,
                );
            }
        }
    }
    Ok(())
}

fn collect_session_log_files(root: &Path) -> Result<Vec<PathBuf>, String> {
    let mut files = Vec::new();
    collect_session_log_files_inner(root, &mut files)
        .map_err(|_| "读取 Codex 会话日志目录失败。".to_string())?;
    files.sort();
    Ok(files)
}

fn collect_session_log_files_inner(root: &Path, files: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let path = entry.path();
        if file_type.is_dir() {
            collect_session_log_files_inner(&path, files)?;
        } else if file_type.is_file()
            && path
                .extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| extension.eq_ignore_ascii_case("jsonl"))
        {
            files.push(path);
        }
    }
    Ok(())
}

fn read_new_complete_session_log_lines(
    path: &Path,
    state: &mut SessionLogFileState,
) -> Result<Vec<String>, String> {
    let metadata = fs::metadata(path).map_err(|_| "读取 Codex 会话日志文件失败。".to_string())?;
    let file_len = metadata.len();
    if state.offset > file_len {
        state.offset = 0;
        state.completed_seen = false;
    }
    if state.offset == file_len {
        return Ok(Vec::new());
    }

    let mut file = fs::File::open(path).map_err(|_| "打开 Codex 会话日志文件失败。".to_string())?;
    file.seek(SeekFrom::Start(state.offset))
        .map_err(|_| "读取 Codex 会话日志偏移失败。".to_string())?;

    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)
        .map_err(|_| "读取 Codex 会话日志新增内容失败。".to_string())?;

    let Some(last_newline) = buffer.iter().rposition(|byte| *byte == b'\n') else {
        return Ok(Vec::new());
    };

    let complete = &buffer[..=last_newline];
    state.offset = state.offset.saturating_add(complete.len() as u64);
    let text = String::from_utf8_lossy(complete);
    Ok(text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToString::to_string)
        .collect())
}

fn apply_session_log_signal(
    app: &AppHandle,
    runtime: &Arc<Mutex<CodexRuntime>>,
    path: &Path,
    file_state: &mut SessionLogFileState,
    signal: SessionLogSignal,
    completion_notifications_enabled: bool,
) {
    if signal.kind == SessionLogSignalKind::Completed {
        if file_state.completed_seen {
            return;
        }
        file_state.completed_seen = true;
    } else if matches!(
        signal.kind,
        SessionLogSignalKind::Running
            | SessionLogSignalKind::Review
            | SessionLogSignalKind::Waiting
    ) {
        file_state.completed_seen = false;
    }

    let session_id = session_log_display_id(path);
    let (state, notify, error, animation) = match signal.kind {
        SessionLogSignalKind::Running => ("running", true, None, Some(("running", 0, "running"))),
        SessionLogSignalKind::Review => ("review", true, None, Some(("review", 0, "review"))),
        SessionLogSignalKind::Waiting => ("waiting", true, None, Some(("waiting", 0, "waiting"))),
        SessionLogSignalKind::Completed => (
            "completed",
            completion_notifications_enabled,
            None,
            Some(("jumping", 2600, "idle")),
        ),
        SessionLogSignalKind::Failed => (
            "failed",
            true,
            Some("Codex 会话日志记录了任务中断或失败。".to_string()),
            Some(("failed", 3000, "idle")),
        ),
    };

    update_status(
        app,
        runtime,
        StatusPatch {
            state: Some(state),
            message: Some(signal.message),
            thread_id: Some(session_id),
            turn_id: None,
            last_event: Some(signal.last_event),
            error,
            notify: Some(notify),
            animation,
            ..StatusPatch::default()
        },
    );
}

fn classify_session_log_line(line: &str) -> Option<SessionLogSignal> {
    let value: Value = serde_json::from_str(line).ok()?;
    let root_type = value
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let payload = value.get("payload").unwrap_or(&value);

    classify_session_log_candidate(root_type, payload)
        .or_else(|| {
            payload
                .get("response_item")
                .and_then(|item| classify_session_log_candidate("response_item", item))
        })
        .or_else(|| {
            value
                .get("response_item")
                .and_then(|item| classify_session_log_candidate("response_item", item))
        })
        .or_else(|| {
            payload
                .get("event_msg")
                .and_then(|event| classify_session_log_candidate("event_msg", event))
        })
        .or_else(|| {
            value
                .get("event_msg")
                .and_then(|event| classify_session_log_candidate("event_msg", event))
        })
}

fn classify_session_log_candidate(root_type: &str, candidate: &Value) -> Option<SessionLogSignal> {
    let item_type = candidate
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let phase = candidate
        .get("phase")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let role = candidate
        .get("role")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let status = candidate
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or_default();

    if item_type == "task_complete"
        || (item_type == "agent_message" && phase == "final_answer")
        || (item_type == "message" && role == "assistant" && phase == "final_answer")
        || (root_type == "event_msg" && phase == "final_answer")
        || (role == "assistant" && phase == "final_answer")
    {
        return Some(SessionLogSignal {
            kind: SessionLogSignalKind::Completed,
            last_event: "sessionLog/completed",
            message: "Codex 工作完成",
        });
    }

    if item_type == "turn_aborted"
        || item_type == "task_failed"
        || item_type == "error"
        || status == "failed"
        || status == "interrupted"
    {
        return Some(SessionLogSignal {
            kind: SessionLogSignalKind::Failed,
            last_event: "sessionLog/failed",
            message: "Codex 工作中断",
        });
    }

    let item_type_lower = item_type.to_ascii_lowercase();
    if item_type_lower.contains("approval") || item_type_lower.contains("waiting") {
        return Some(SessionLogSignal {
            kind: SessionLogSignalKind::Waiting,
            last_event: "sessionLog/waiting",
            message: "Codex 需要你处理审批或输入",
        });
    }

    match item_type {
        "task_started" => Some(SessionLogSignal {
            kind: SessionLogSignalKind::Running,
            last_event: "sessionLog/task_started",
            message: "Codex 正在工作",
        }),
        "reasoning" => Some(SessionLogSignal {
            kind: SessionLogSignalKind::Review,
            last_event: "sessionLog/reasoning",
            message: "Codex 正在审查和规划",
        }),
        "function_call" | "function_call_output" | "web_search_call" => Some(SessionLogSignal {
            kind: SessionLogSignalKind::Running,
            last_event: "sessionLog/tool",
            message: "Codex 正在执行任务",
        }),
        "agent_message" if phase == "commentary" => Some(SessionLogSignal {
            kind: SessionLogSignalKind::Running,
            last_event: "sessionLog/commentary",
            message: "Codex 正在工作",
        }),
        "message" if role == "assistant" && phase == "commentary" => Some(SessionLogSignal {
            kind: SessionLogSignalKind::Running,
            last_event: "sessionLog/commentary",
            message: "Codex 正在工作",
        }),
        _ => None,
    }
}

fn session_log_display_id(path: &Path) -> String {
    let stem = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("session");
    let short = if stem.chars().count() > 16 {
        let mut chars = stem.chars().rev().take(12).collect::<Vec<_>>();
        chars.reverse();
        chars.into_iter().collect::<String>()
    } else {
        stem.to_string()
    };
    format!("session:{short}")
}

fn open_connection(
    settings: &CodexAppServerSettings,
    mode: ConnectionMode,
    port: u16,
    endpoint: &str,
    child: &mut Option<Child>,
) -> Result<CodexConnection, String> {
    match mode {
        ConnectionMode::Managed => {
            *child = Some(spawn_codex_app_server(settings, port)?);
            wait_for_port(port)?;

            let (mut socket, _) = tungstenite::connect(endpoint).map_err(|err| err.to_string())?;
            set_socket_timeout(&mut socket);
            Ok(CodexConnection::WebSocket(socket))
        }
        ConnectionMode::Proxy => {
            *child = Some(spawn_codex_proxy(settings)?);
            let child_ref = child
                .as_mut()
                .ok_or_else(|| "Codex proxy 进程启动失败。".to_string())?;
            let stdin = child_ref
                .stdin
                .take()
                .ok_or_else(|| "无法打开 Codex proxy 输入通道。".to_string())?;
            let stdout = child_ref
                .stdout
                .take()
                .ok_or_else(|| "无法打开 Codex proxy 输出通道。".to_string())?;
            let stderr = child_ref.stderr.take();
            let events = spawn_stdio_reader(stdout, stderr);
            Ok(CodexConnection::Stdio { stdin, events })
        }
        ConnectionMode::SessionLog => Err("日志监听模式不需要打开 App Server 连接。".to_string()),
    }
}

fn handle_server_message(
    app: &AppHandle,
    runtime: &Arc<Mutex<CodexRuntime>>,
    connection: &mut CodexConnection,
    next_request_id: &mut u64,
    requests: &mut HashMap<u64, RequestKind>,
    thread_id: &mut Option<String>,
    pending_turn: &mut Option<PendingTurn>,
    mode: ConnectionMode,
    completion_notifications_enabled: bool,
    text: &str,
) -> Result<(), String> {
    let value: Value = serde_json::from_str(text).map_err(|err| err.to_string())?;

    if value.get("result").is_some() || value.get("error").is_some() {
        handle_response(
            app,
            runtime,
            connection,
            next_request_id,
            requests,
            thread_id,
            pending_turn,
            mode,
            &value,
        )?;
        return Ok(());
    }

    let method = value
        .get("method")
        .and_then(Value::as_str)
        .unwrap_or_default();

    if value.get("id").is_some() && method.contains("request") {
        update_status(
            app,
            runtime,
            StatusPatch {
                state: Some("waiting"),
                message: Some("Codex 需要审批或用户输入"),
                last_event: Some(method),
                error: None,
                animation: Some(("waiting", 0, "waiting")),
                ..StatusPatch::default()
            },
        );
        return Ok(());
    }

    handle_notification(
        app,
        runtime,
        thread_id,
        method,
        completion_notifications_enabled,
        &value,
    );
    Ok(())
}

fn handle_response(
    app: &AppHandle,
    runtime: &Arc<Mutex<CodexRuntime>>,
    connection: &mut CodexConnection,
    next_request_id: &mut u64,
    requests: &mut HashMap<u64, RequestKind>,
    thread_id: &mut Option<String>,
    pending_turn: &mut Option<PendingTurn>,
    mode: ConnectionMode,
    value: &Value,
) -> Result<(), String> {
    let id = value.get("id").and_then(Value::as_u64).unwrap_or_default();
    let request_kind = requests.remove(&id);

    if let Some(error) = value.get("error") {
        let message = error
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or("Codex App Server 返回错误");
        update_status(
            app,
            runtime,
            StatusPatch {
                state: Some("failed"),
                message: Some("Codex 请求失败"),
                last_event: Some("response/error"),
                error: Some(sanitize_message(message)),
                animation: Some(("failed", 3000, "idle")),
                ..StatusPatch::default()
            },
        );
        return Ok(());
    }

    match request_kind {
        Some(RequestKind::Initialize) => {
            update_status(
                app,
                runtime,
                StatusPatch {
                    state: Some("connected"),
                    message: Some(if mode == ConnectionMode::Proxy {
                        "已连接当前 Codex，等待任务状态"
                    } else {
                        "Codex App Server 已初始化"
                    }),
                    mode: Some(mode.as_str()),
                    last_event: Some("initialize/complete"),
                    error: None,
                    ..StatusPatch::default()
                },
            );
            if mode == ConnectionMode::Proxy {
                send_thread_loaded_list(connection, next_request_id, requests)?;
            }
        }
        Some(RequestKind::ThreadLoadedList) => {
            let thread_ids = value
                .pointer("/result/data")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            let count = thread_ids.len();
            for id in thread_ids
                .iter()
                .filter_map(Value::as_str)
                .filter(|id| !id.trim().is_empty())
                .take(20)
            {
                send_thread_resume(connection, next_request_id, requests, id)?;
            }
            update_status(
                app,
                runtime,
                StatusPatch {
                    state: Some("connected"),
                    message: Some(if count == 0 {
                        "已监听当前 Codex，等待你开始任务"
                    } else {
                        "已监听当前 Codex，正在同步已加载线程"
                    }),
                    last_event: Some("thread/loaded/list"),
                    error: None,
                    ..StatusPatch::default()
                },
            );
        }
        Some(RequestKind::ThreadResume) => {
            if let Some(next_thread_id) = value
                .pointer("/result/thread/id")
                .and_then(Value::as_str)
                .map(ToString::to_string)
            {
                *thread_id = Some(next_thread_id.clone());
                if let Some(status) = value.pointer("/result/thread/status") {
                    update_from_thread_status(
                        app,
                        runtime,
                        Some(next_thread_id),
                        status,
                        "thread/resume",
                    );
                }
            }
        }
        Some(RequestKind::ThreadStart) => {
            if let Some(next_thread_id) = value
                .pointer("/result/thread/id")
                .and_then(Value::as_str)
                .map(ToString::to_string)
            {
                *thread_id = Some(next_thread_id.clone());
                update_status(
                    app,
                    runtime,
                    StatusPatch {
                        state: Some("connected"),
                        message: Some("Codex 线程已创建"),
                        thread_id: Some(next_thread_id.clone()),
                        last_event: Some("thread/start"),
                        error: None,
                        ..StatusPatch::default()
                    },
                );
                if let Some(turn) = pending_turn.take() {
                    send_turn_start(connection, next_request_id, requests, &next_thread_id, turn)?;
                }
            }
        }
        Some(RequestKind::TurnStart) => {
            let next_turn_id = value
                .pointer("/result/turn/id")
                .and_then(Value::as_str)
                .map(ToString::to_string);
            update_status(
                app,
                runtime,
                StatusPatch {
                    state: Some("running"),
                    message: Some("Codex 正在工作"),
                    turn_id: next_turn_id,
                    last_event: Some("turn/start"),
                    error: None,
                    animation: Some(("running", 0, "running")),
                    ..StatusPatch::default()
                },
            );
        }
        None => {}
    }

    Ok(())
}

fn handle_notification(
    app: &AppHandle,
    runtime: &Arc<Mutex<CodexRuntime>>,
    thread_id: &mut Option<String>,
    method: &str,
    completion_notifications_enabled: bool,
    value: &Value,
) {
    match method {
        "thread/started" => {
            if let Some(next_thread_id) = value
                .pointer("/params/thread/id")
                .and_then(Value::as_str)
                .map(ToString::to_string)
            {
                *thread_id = Some(next_thread_id.clone());
                if let Some(status) = value.pointer("/params/thread/status") {
                    update_from_thread_status(app, runtime, Some(next_thread_id), status, method);
                } else {
                    update_status(
                        app,
                        runtime,
                        StatusPatch {
                            state: Some("connected"),
                            message: Some("Codex 线程已就绪"),
                            thread_id: Some(next_thread_id),
                            last_event: Some(method),
                            error: None,
                            ..StatusPatch::default()
                        },
                    );
                }
            }
        }
        "thread/status/changed" => {
            let next_thread_id = value
                .pointer("/params/threadId")
                .and_then(Value::as_str)
                .map(ToString::to_string);
            if let Some(next_thread_id) = next_thread_id.clone() {
                *thread_id = Some(next_thread_id);
            }
            if let Some(status) = value.pointer("/params/status") {
                update_from_thread_status(app, runtime, next_thread_id, status, method);
            }
        }
        "turn/started" => {
            if let Some(next_thread_id) = value
                .pointer("/params/threadId")
                .and_then(Value::as_str)
                .map(ToString::to_string)
            {
                *thread_id = Some(next_thread_id.clone());
                update_status(
                    app,
                    runtime,
                    StatusPatch {
                        thread_id: Some(next_thread_id),
                        ..StatusPatch::default()
                    },
                );
            }
            update_status(
                app,
                runtime,
                StatusPatch {
                    state: Some("running"),
                    message: Some("Codex 正在工作"),
                    turn_id: value
                        .pointer("/params/turn/id")
                        .and_then(Value::as_str)
                        .map(ToString::to_string),
                    last_event: Some(method),
                    error: None,
                    animation: Some(("running", 0, "running")),
                    ..StatusPatch::default()
                },
            );
        }
        "turn/completed" => {
            if let Some(next_thread_id) = value
                .pointer("/params/threadId")
                .and_then(Value::as_str)
                .map(ToString::to_string)
            {
                *thread_id = Some(next_thread_id.clone());
                update_status(
                    app,
                    runtime,
                    StatusPatch {
                        thread_id: Some(next_thread_id),
                        ..StatusPatch::default()
                    },
                );
            }
            let turn_status = value
                .pointer("/params/turn/status")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let failed = turn_status == "failed" || turn_status == "interrupted";
            let error = value
                .pointer("/params/turn/error/message")
                .and_then(Value::as_str)
                .map(sanitize_message);
            update_status(
                app,
                runtime,
                StatusPatch {
                    state: Some(if failed { "failed" } else { "completed" }),
                    message: Some(if failed {
                        "Codex 工作失败"
                    } else {
                        "Codex 工作完成"
                    }),
                    turn_id: value
                        .pointer("/params/turn/id")
                        .and_then(Value::as_str)
                        .map(ToString::to_string),
                    last_event: Some(method),
                    error,
                    notify: Some(completion_notifications_enabled),
                    animation: Some(if failed {
                        ("failed", 3000, "idle")
                    } else {
                        ("jumping", 2600, "idle")
                    }),
                    ..StatusPatch::default()
                },
            );
        }
        "item/started" | "item/completed" => {
            if let Some(next_thread_id) = value
                .pointer("/params/threadId")
                .and_then(Value::as_str)
                .map(ToString::to_string)
            {
                *thread_id = Some(next_thread_id.clone());
                update_status(
                    app,
                    runtime,
                    StatusPatch {
                        thread_id: Some(next_thread_id),
                        turn_id: value
                            .pointer("/params/turnId")
                            .and_then(Value::as_str)
                            .map(ToString::to_string),
                        ..StatusPatch::default()
                    },
                );
            }
            let item_type = value
                .pointer("/params/item/type")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let (state, message, animation) = match item_type {
                "reasoning" | "plan" | "enteredReviewMode" => {
                    ("review", "Codex 正在审查和规划", ("review", 0, "review"))
                }
                "commandExecution" | "fileChange" | "mcpToolCall" | "dynamicToolCall" => {
                    ("running", "Codex 正在执行任务", ("running", 0, "running"))
                }
                _ => ("running", "Codex 正在工作", ("running", 0, "running")),
            };
            update_status(
                app,
                runtime,
                StatusPatch {
                    state: Some(state),
                    message: Some(message),
                    last_event: Some(method),
                    error: None,
                    animation: Some(animation),
                    ..StatusPatch::default()
                },
            );
        }
        "item/autoApprovalReview/started" => {
            update_status(
                app,
                runtime,
                StatusPatch {
                    state: Some("review"),
                    message: Some("Codex 正在审查审批请求"),
                    last_event: Some(method),
                    error: None,
                    animation: Some(("review", 0, "review")),
                    ..StatusPatch::default()
                },
            );
        }
        "error" => {
            let message = value
                .pointer("/params/error/message")
                .and_then(Value::as_str)
                .unwrap_or("Codex 返回错误");
            update_status(
                app,
                runtime,
                StatusPatch {
                    state: Some("failed"),
                    message: Some("Codex 工作失败"),
                    last_event: Some(method),
                    error: Some(sanitize_message(message)),
                    animation: Some(("failed", 3000, "idle")),
                    ..StatusPatch::default()
                },
            );
        }
        _ => {}
    }
}

fn update_from_thread_status(
    app: &AppHandle,
    runtime: &Arc<Mutex<CodexRuntime>>,
    thread_id: Option<String>,
    status: &Value,
    method: &str,
) {
    let state_type = status
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let flags = status
        .get("activeFlags")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let waiting = flags.iter().any(|flag| {
        matches!(
            flag.as_str(),
            Some("waitingOnApproval") | Some("waitingOnUserInput")
        )
    });
    let (state, message, animation) = if waiting {
        (
            "waiting",
            "Codex 需要你处理审批或输入",
            ("waiting", 0, "waiting"),
        )
    } else if state_type == "active" {
        ("running", "Codex 正在工作", ("running", 0, "running"))
    } else if state_type == "systemError" {
        ("failed", "Codex 状态异常", ("failed", 3000, "idle"))
    } else {
        ("connected", "Codex 当前空闲", ("idle", 0, "idle"))
    };

    update_status(
        app,
        runtime,
        StatusPatch {
            state: Some(state),
            message: Some(message),
            thread_id,
            last_event: Some(method),
            error: None,
            animation: Some(animation),
            ..StatusPatch::default()
        },
    );
}

fn send_thread_loaded_list(
    connection: &mut CodexConnection,
    next_request_id: &mut u64,
    requests: &mut HashMap<u64, RequestKind>,
) -> Result<(), String> {
    send_request(
        connection,
        next_request_id,
        requests,
        RequestKind::ThreadLoadedList,
        "thread/loaded/list",
        json!({
            "limit": 20
        }),
    )?;
    Ok(())
}

fn send_thread_resume(
    connection: &mut CodexConnection,
    next_request_id: &mut u64,
    requests: &mut HashMap<u64, RequestKind>,
    thread_id: &str,
) -> Result<(), String> {
    send_request(
        connection,
        next_request_id,
        requests,
        RequestKind::ThreadResume,
        "thread/resume",
        json!({
            "threadId": thread_id,
            "excludeTurns": true
        }),
    )?;
    Ok(())
}

fn send_thread_start(
    connection: &mut CodexConnection,
    next_request_id: &mut u64,
    requests: &mut HashMap<u64, RequestKind>,
    cwd: Option<String>,
) -> Result<(), String> {
    let mut params = json!({
        "ephemeral": true
    });
    if let Some(cwd) = cwd {
        params["cwd"] = json!(cwd);
    }
    send_request(
        connection,
        next_request_id,
        requests,
        RequestKind::ThreadStart,
        "thread/start",
        params,
    )?;
    Ok(())
}

fn send_turn_start(
    connection: &mut CodexConnection,
    next_request_id: &mut u64,
    requests: &mut HashMap<u64, RequestKind>,
    thread_id: &str,
    turn: PendingTurn,
) -> Result<(), String> {
    let mut params = json!({
        "threadId": thread_id,
        "input": [{
            "type": "text",
            "text": turn.prompt,
            "text_elements": []
        }]
    });
    if let Some(cwd) = turn.cwd {
        params["cwd"] = json!(cwd);
    }
    send_request(
        connection,
        next_request_id,
        requests,
        RequestKind::TurnStart,
        "turn/start",
        params,
    )?;
    Ok(())
}

fn send_request(
    connection: &mut CodexConnection,
    next_request_id: &mut u64,
    requests: &mut HashMap<u64, RequestKind>,
    request_kind: RequestKind,
    method: &str,
    params: Value,
) -> Result<u64, String> {
    let id = *next_request_id;
    *next_request_id = next_request_id.wrapping_add(1);
    let payload = json!({
        "id": id,
        "method": method,
        "params": params
    });
    connection.send_json(payload)?;
    requests.insert(id, request_kind);
    Ok(id)
}

fn spawn_codex_app_server(settings: &CodexAppServerSettings, port: u16) -> Result<Child, String> {
    let endpoint = format!("ws://127.0.0.1:{port}");
    let mut command = Command::new(settings.command.trim());
    command
        .args(["app-server", "--listen", endpoint.as_str()])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    #[cfg(windows)]
    {
        command.creation_flags(CREATE_NO_WINDOW);
    }

    command
        .spawn()
        .map_err(|err| format!("启动 Codex 命令失败：{err}"))
}

fn spawn_codex_proxy(settings: &CodexAppServerSettings) -> Result<Child, String> {
    let mut command = Command::new(settings.command.trim());
    command.args(["app-server", "proxy"]);
    let socket_path = settings.socket_path.trim();
    let socket_path = if socket_path.is_empty() {
        default_codex_control_socket_path()?
    } else {
        PathBuf::from(socket_path)
    };

    if !socket_path.exists() {
        return Err(missing_control_socket_message(&socket_path));
    }

    command.arg("--sock").arg(&socket_path);
    command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    #[cfg(windows)]
    {
        command.creation_flags(CREATE_NO_WINDOW);
    }

    command
        .spawn()
        .map_err(|err| format!("启动 Codex proxy 失败：{err}"))
}

fn default_codex_control_socket_path() -> Result<PathBuf, String> {
    Ok(default_codex_home_path()?
        .join("app-server-control")
        .join("app-server-control.sock"))
}

fn default_codex_sessions_dir() -> Result<PathBuf, String> {
    Ok(default_codex_home_path()?.join("sessions"))
}

fn default_codex_home_path() -> Result<PathBuf, String> {
    std::env::var_os("CODEX_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("USERPROFILE").map(|home| PathBuf::from(home).join(".codex")))
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".codex")))
        .ok_or_else(|| "无法定位 CODEX_HOME，不能自动查找 Codex 本机会话数据。".to_string())
}

fn missing_control_socket_message(socket_path: &std::path::Path) -> String {
    format!(
        "未找到 Codex control socket：{}。当前 Codex App/扩展没有暴露 app-server-control.sock；请切换到“独立测试模式”，启动后使用设置页显示的 codex --remote ws://127.0.0.1:<端口> 命令开始任务，或填写 Codex 侧实际提供的 --sock 路径。",
        socket_path.display()
    )
}

fn spawn_stdio_reader(
    stdout: ChildStdout,
    stderr: Option<ChildStderr>,
) -> mpsc::Receiver<TransportEvent> {
    let (tx, rx) = mpsc::channel();
    let stdout_tx = tx.clone();
    thread::spawn(move || {
        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            match line {
                Ok(line) => {
                    if stdout_tx.send(TransportEvent::Message(line)).is_err() {
                        return;
                    }
                }
                Err(err) => {
                    let _ = stdout_tx.send(TransportEvent::Error(err.to_string()));
                    return;
                }
            }
        }
        let _ = stdout_tx.send(TransportEvent::Closed);
    });

    if let Some(stderr) = stderr {
        thread::spawn(move || {
            let reader = BufReader::new(stderr);
            for line in reader.lines().map_while(Result::ok) {
                let line = line.trim();
                if line.is_empty() || line == "Error:" {
                    continue;
                }
                let _ = tx.send(TransportEvent::Error(friendly_proxy_error(line)));
                return;
            }
        });
    }

    rx
}

fn friendly_proxy_error(message: &str) -> String {
    if message.contains("failed to connect to socket") {
        return "无法连接 Codex control socket。当前 Codex App/扩展没有暴露 app-server-control.sock；请确认 Codex 侧已启动支持 control socket 的 app-server，或在桌宠中切换到“独立测试模式”，再用 codex --remote ws://127.0.0.1:<端口> 连接桌宠启动的 App Server。".to_string();
    }
    if message.contains("control socket path exists and is not a socket") {
        return "Codex control socket 路径存在但不是可连接的 socket，请重启 Codex 后再试。"
            .to_string();
    }
    message.to_string()
}

fn set_socket_timeout(socket: &mut CodexSocket) {
    if let MaybeTlsStream::Plain(stream) = socket.get_mut() {
        let _ = stream.set_read_timeout(Some(SOCKET_READ_TIMEOUT));
        let _ = stream.set_write_timeout(Some(Duration::from_secs(5)));
    }
}

fn wait_for_port(port: u16) -> Result<(), String> {
    let started = std::time::Instant::now();
    loop {
        if TcpStream::connect(("127.0.0.1", port)).is_ok() {
            return Ok(());
        }
        if started.elapsed() >= CONNECT_TIMEOUT {
            return Err("等待 Codex App Server 监听端口超时。".to_string());
        }
        thread::sleep(Duration::from_millis(200));
    }
}

fn pick_available_port() -> Result<u16, String> {
    let listener =
        TcpListener::bind(("127.0.0.1", 0)).map_err(|err| format!("无法分配本机端口：{err}"))?;
    let port = listener
        .local_addr()
        .map_err(|err| format!("无法读取本机端口：{err}"))?
        .port();
    drop(listener);
    Ok(port)
}

#[derive(Default)]
struct StatusPatch<'a> {
    state: Option<&'a str>,
    message: Option<&'a str>,
    thread_id: Option<String>,
    turn_id: Option<String>,
    endpoint: Option<String>,
    mode: Option<&'a str>,
    last_event: Option<&'a str>,
    error: Option<String>,
    notify: Option<bool>,
    clear_thread: bool,
    clear_endpoint: bool,
    clear_tasks: bool,
    animation: Option<(&'a str, u32, &'a str)>,
}

fn rebuild_runtime_status(runtime: &mut CodexRuntime) -> CodexStatusPayload {
    runtime.status.active = runtime.control.is_some();
    let tasks = codex_task_payloads(runtime);
    let summary = summarize_codex_status(&runtime.status, &tasks);
    runtime.status.tasks = tasks;
    runtime.status.summary = summary;
    runtime.status.state = runtime.status.summary.state.clone();
    runtime.status.message = runtime.status.summary.message.clone();
    runtime.status.clone()
}

fn update_runtime_task(
    runtime: &mut CodexRuntime,
    state: &str,
    message: &str,
    updated_at: u64,
    notify: bool,
) {
    if !is_trackable_codex_task_state(state) {
        return;
    }

    let key = runtime
        .status
        .thread_id
        .clone()
        .or_else(|| runtime.status.turn_id.clone());
    let Some(key) = key else {
        return;
    };

    let label = if let Some(task) = runtime.tasks.get(&key) {
        task.label.clone()
    } else {
        let label = format!("任务 {}", runtime.next_task_label);
        runtime.next_task_label = runtime.next_task_label.saturating_add(1);
        label
    };
    let unread = match runtime.tasks.get(&key) {
        Some(task) if task.state == state && matches!(state, "completed" | "failed") => task.unread,
        _ => matches!(state, "failed" | "waiting") || (state == "completed" && notify),
    };

    runtime.tasks.insert(
        key,
        CodexTaskStatus {
            label,
            state: state.to_string(),
            message: message.to_string(),
            mode: runtime.status.mode.clone(),
            last_event: runtime.status.last_event.clone(),
            updated_at,
            unread,
        },
    );
    prune_codex_tasks(runtime);
}

fn is_trackable_codex_task_state(state: &str) -> bool {
    matches!(
        state,
        "connected" | "running" | "review" | "waiting" | "completed" | "failed"
    )
}

fn prune_codex_tasks(runtime: &mut CodexRuntime) {
    let now = now_seconds();
    runtime.tasks.retain(|_, task| {
        !matches!(task.state.as_str(), "completed" | "failed")
            || now.saturating_sub(task.updated_at) <= CODEX_TASK_RETENTION_SECONDS
    });

    if runtime.tasks.len() <= CODEX_TASK_LIMIT {
        return;
    }

    let mut by_age = runtime
        .tasks
        .iter()
        .map(|(key, task)| (key.clone(), task.updated_at))
        .collect::<Vec<_>>();
    by_age.sort_by_key(|(_, updated_at)| *updated_at);
    let remove_count = runtime.tasks.len().saturating_sub(CODEX_TASK_LIMIT);
    for (key, _) in by_age.into_iter().take(remove_count) {
        runtime.tasks.remove(&key);
    }
}

fn codex_task_payloads(runtime: &CodexRuntime) -> Vec<CodexTaskStatusPayload> {
    let mut tasks = runtime
        .tasks
        .values()
        .map(|task| CodexTaskStatusPayload {
            id: task.label.clone(),
            label: task.label.clone(),
            state: task.state.clone(),
            message: task.message.clone(),
            mode: task.mode.clone(),
            last_event: task.last_event.clone(),
            updated_at: task.updated_at,
            unread: task.unread,
        })
        .collect::<Vec<_>>();
    tasks.sort_by(|left, right| right.updated_at.cmp(&left.updated_at));
    tasks
}

fn summarize_codex_status(
    base: &CodexStatusPayload,
    tasks: &[CodexTaskStatusPayload],
) -> CodexStatusSummaryPayload {
    let running_count = tasks.iter().filter(|task| task.state == "running").count();
    let review_count = tasks.iter().filter(|task| task.state == "review").count();
    let waiting_count = tasks.iter().filter(|task| task.state == "waiting").count();
    let completed_count = tasks
        .iter()
        .filter(|task| task.state == "completed")
        .count();
    let failed_count = tasks.iter().filter(|task| task.state == "failed").count();
    let unread_completed_count = tasks
        .iter()
        .filter(|task| task.state == "completed" && task.unread)
        .count();
    let unread_failed_count = tasks
        .iter()
        .filter(|task| task.state == "failed" && task.unread)
        .count();
    let active_count = running_count + review_count + waiting_count;
    let unread_count = waiting_count + unread_completed_count + unread_failed_count;

    let (state, message, attention) = if waiting_count > 0 {
        (
            "waiting",
            count_message(
                waiting_count,
                "Codex 需要你处理",
                "Codex：{} 个任务需要处理",
            ),
            "waiting",
        )
    } else if unread_failed_count > 0 {
        (
            "failed",
            count_message(
                unread_failed_count,
                "Codex 工作失败",
                "Codex：{} 个任务失败",
            ),
            "failed",
        )
    } else if unread_completed_count > 0 {
        (
            "completed",
            count_message(
                unread_completed_count,
                "Codex 工作完成",
                "Codex：{} 个任务已完成",
            ),
            "completed",
        )
    } else if running_count + review_count > 0 {
        let busy_count = running_count + review_count;
        let state = if running_count > 0 {
            "running"
        } else {
            "review"
        };
        let singular = if state == "review" {
            "Codex 正在审查"
        } else {
            "Codex 正在工作"
        };
        (
            state,
            count_message(busy_count, singular, "Codex：{} 个任务进行中"),
            "working",
        )
    } else if tasks.is_empty() {
        let attention = if base.state == "failed" {
            "failed"
        } else if base.state == "starting" {
            "working"
        } else if base.active {
            "idle"
        } else {
            "none"
        };
        (base.state.as_str(), base.message.clone(), attention)
    } else if base.active {
        ("connected", "Codex 当前空闲".to_string(), "idle")
    } else {
        (
            "disconnected",
            "Codex App Server 未连接".to_string(),
            "none",
        )
    };

    CodexStatusSummaryPayload {
        state: state.to_string(),
        message,
        attention: attention.to_string(),
        total_count: tasks.len(),
        active_count,
        running_count,
        review_count,
        waiting_count,
        completed_count,
        failed_count,
        unread_count,
        unread_completed_count,
        unread_failed_count,
        badge_label: codex_badge_label(
            waiting_count,
            unread_failed_count,
            unread_completed_count,
            active_count,
        ),
    }
}

fn count_message(count: usize, singular: &str, plural_template: &str) -> String {
    if count <= 1 {
        singular.to_string()
    } else {
        plural_template.replace("{}", &count.to_string())
    }
}

fn codex_badge_label(
    waiting_count: usize,
    unread_failed_count: usize,
    unread_completed_count: usize,
    active_count: usize,
) -> Option<String> {
    if waiting_count > 0 {
        return Some(count_badge(waiting_count, "待处理"));
    }
    if unread_failed_count > 0 {
        return Some(count_badge(unread_failed_count, "失败"));
    }
    if unread_completed_count > 0 {
        return Some(count_badge(unread_completed_count, "完成"));
    }
    if active_count > 1 {
        return Some(count_badge(active_count, "进行中"));
    }
    None
}

fn count_badge(count: usize, label: &str) -> String {
    if count <= 1 {
        label.to_string()
    } else {
        format!("{label} {count}")
    }
}

fn animation_fallback_for_summary(summary: &CodexStatusSummaryPayload, fallback: &str) -> String {
    if summary.waiting_count > 0 {
        "waiting".to_string()
    } else if summary.running_count > 0 {
        "running".to_string()
    } else if summary.review_count > 0 {
        "review".to_string()
    } else {
        fallback.to_string()
    }
}

fn update_status(
    app: &AppHandle,
    runtime: &Arc<Mutex<CodexRuntime>>,
    patch: StatusPatch<'_>,
) -> CodexStatusPayload {
    let payload = {
        let mut guard = match runtime.lock() {
            Ok(guard) => guard,
            Err(_) => return CodexStatusPayload::default(),
        };
        let updated_at = now_seconds();
        let should_update_task = patch.state.is_some();
        if let Some(state) = patch.state {
            guard.status.state = state.to_string();
        }
        guard.status.active = guard.control.is_some();
        if let Some(message) = patch.message {
            guard.status.message = message.to_string();
        }
        if patch.clear_thread {
            guard.status.thread_id = None;
            guard.status.turn_id = None;
        } else {
            if let Some(thread_id) = patch.thread_id {
                guard.status.thread_id = Some(thread_id);
            }
            if let Some(turn_id) = patch.turn_id {
                guard.status.turn_id = Some(turn_id);
            }
        }
        if patch.clear_endpoint {
            guard.status.endpoint = None;
            guard.status.mode = None;
        } else {
            if let Some(endpoint) = patch.endpoint {
                guard.status.endpoint = Some(endpoint);
            }
            if let Some(mode) = patch.mode {
                guard.status.mode = Some(mode.to_string());
            }
        }
        if let Some(last_event) = patch.last_event {
            guard.status.last_event = Some(last_event.to_string());
        }
        guard.status.error = patch.error;
        let notify = patch.notify.unwrap_or(true);
        guard.status.notify = notify;
        guard.status.updated_at = updated_at;
        if patch.clear_tasks {
            guard.tasks.clear();
        }
        if should_update_task {
            let state = guard.status.state.clone();
            let message = guard.status.message.clone();
            update_runtime_task(&mut guard, &state, &message, updated_at, notify);
        }
        rebuild_runtime_status(&mut guard)
    };
    emit_status(app, &payload);
    if payload.notify {
        if let Some((state, duration_ms, fallback)) = patch.animation {
            let fallback = animation_fallback_for_summary(&payload.summary, fallback);
            let _ = app.emit(
                PET_ANIMATION_EVENT,
                json!({
                    "state": state,
                    "durationMs": duration_ms,
                    "fallback": fallback
                }),
            );
        }
    }
    payload
}

fn emit_status(app: &AppHandle, payload: &CodexStatusPayload) {
    let _ = app.emit(STATUS_EVENT, payload);
}

fn sanitize_message(message: &str) -> String {
    let compact = message
        .replace(['\r', '\n', '\t'], " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    if compact.chars().count() > 240 {
        compact.chars().take(240).collect::<String>() + "..."
    } else {
        compact
    }
}

fn now_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}
