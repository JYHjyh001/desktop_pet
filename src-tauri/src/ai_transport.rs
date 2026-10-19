//! Bounded, pooled AI HTTP transport. Never include remote bodies/URLs in errors.
use std::{
    cell::RefCell,
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, Mutex, OnceLock,
    },
    time::Duration,
};
use tokio::sync::Semaphore;

pub const MAX_RESPONSE_BYTES: usize = 12 * 1024 * 1024;
const MAX_CONCURRENT: usize = 4;
static CLIENT: OnceLock<Result<reqwest::Client, String>> = OnceLock::new();
static SLOTS: Semaphore = Semaphore::const_new(MAX_CONCURRENT);
type RequestRegistry = Mutex<HashMap<u64, (String, Arc<AtomicBool>)>>;
static REQUESTS: OnceLock<RequestRegistry> = OnceLock::new();
static NEXT_ID: AtomicU64 = AtomicU64::new(1);
thread_local! { static CANCELLATION: RefCell<Option<Arc<AtomicBool>>> = const { RefCell::new(None) }; }

fn requests() -> &'static RequestRegistry {
    REQUESTS.get_or_init(Mutex::default)
}

/// One UI operation may perform several requests (reply, then memory extraction).
/// Cancelling its window prevents subsequent requests in that operation too.
pub fn with_scope<T>(owner: &str, operation: impl FnOnce() -> T) -> T {
    struct Scope {
        id: u64,
        previous: Option<Arc<AtomicBool>>,
    }
    impl Drop for Scope {
        fn drop(&mut self) {
            if let Ok(mut entries) = requests().lock() {
                entries.remove(&self.id);
            }
            CANCELLATION.with(|cell| {
                cell.replace(self.previous.take());
            });
        }
    }
    let token = Arc::new(AtomicBool::new(false));
    let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    if let Ok(mut entries) = requests().lock() {
        entries.insert(id, (owner.to_owned(), token.clone()));
    }
    let _scope = Scope {
        id,
        previous: CANCELLATION.with(|cell| cell.replace(Some(token))),
    };
    operation()
}

pub fn cancel_owner(owner: &str) {
    if let Ok(entries) = requests().lock() {
        for (label, token) in entries.values() {
            if label == owner {
                token.store(true, Ordering::Release);
            }
        }
    }
}

fn client() -> Result<&'static reqwest::Client, String> {
    CLIENT
        .get_or_init(|| {
            let builder = reqwest::Client::builder()
                .user_agent("PetDrawer-AI")
                .connect_timeout(Duration::from_secs(10))
                .timeout(Duration::from_secs(60))
                .pool_max_idle_per_host(2)
                .pool_idle_timeout(Duration::from_secs(30))
                .redirect(reqwest::redirect::Policy::none());
            // Local mock tests must not use the developer's system proxy.
            #[cfg(test)]
            let builder = builder.no_proxy();
            builder
                .build()
                .map_err(|_| "无法初始化 AI 安全连接。".into())
        })
        .as_ref()
        .map_err(Clone::clone)
}

pub fn block_on_cancellable<T>(
    operation: impl std::future::Future<Output = Result<T, String>>,
) -> Result<T, String> {
    let token = CANCELLATION
        .with(|cell| cell.borrow().clone())
        .unwrap_or_else(|| Arc::new(AtomicBool::new(false)));
    tauri::async_runtime::block_on(async {
        if token.load(Ordering::Acquire) {
            return Err("请求已取消。".into());
        }
        tokio::select! { result = operation => result, _ = cancelled(&token) => Err("请求已取消。".into()) }
    })
}

pub fn post_json(provider: &str, url: &str, api_key: &str, body: &str) -> Result<String, String> {
    let token = CANCELLATION
        .with(|cell| cell.borrow().clone())
        .unwrap_or_else(|| Arc::new(AtomicBool::new(false)));
    tauri::async_runtime::block_on(async {
        let _slot = SLOTS
            .try_acquire()
            .map_err(|_| "AI 请求繁忙，请稍后重试。")?;
        if token.load(Ordering::Acquire) {
            return Err("AI 请求已取消。".into());
        }
        if body.len() > MAX_RESPONSE_BYTES {
            return Err("AI 请求内容超过大小限制。".into());
        }
        let mut request = client()?
            .post(url)
            .header(reqwest::header::CONTENT_TYPE, "application/json");
        match provider {
            "anthropic" => {
                request = request.header("anthropic-version", "2023-06-01");
                if !api_key.is_empty() {
                    request = request.header("x-api-key", api_key);
                }
            }
            "gemini" => {
                if !api_key.is_empty() {
                    request = request.header("x-goog-api-key", api_key);
                }
            }
            _ => {
                if !api_key.is_empty() {
                    request = request.bearer_auth(api_key);
                }
            }
        }
        tokio::select! {
            result = receive(request.body(body.to_owned()), MAX_RESPONSE_BYTES) => result,
            _ = cancelled(&token) => Err("AI 请求已取消。".into()),
        }
    })
}

async fn cancelled(token: &AtomicBool) {
    while !token.load(Ordering::Acquire) {
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
}

async fn receive(request: reqwest::RequestBuilder, limit: usize) -> Result<String, String> {
    let mut response = request.send().await.map_err(network_error)?;
    if !response.status().is_success() {
        return Err(format!(
            "AI 服务请求失败（HTTP {}）。",
            response.status().as_u16()
        ));
    }
    if response
        .content_length()
        .is_some_and(|length| length > limit as u64)
    {
        return Err("AI 响应超过大小限制。".into());
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(network_error)? {
        if chunk.len() > limit.saturating_sub(bytes.len()) {
            return Err("AI 响应超过大小限制。".into());
        }
        bytes.extend_from_slice(&chunk);
    }
    let text = String::from_utf8(bytes).map_err(|_| "AI 响应编码无效。")?;
    if text.trim().is_empty() {
        return Err("AI 服务返回了空内容。".into());
    }
    Ok(text)
}

fn network_error(error: reqwest::Error) -> String {
    if error.is_timeout() {
        "AI 请求超时，请稍后重试。"
    } else if error.is_connect() {
        "无法连接 AI 服务，请检查地址、代理或证书。"
    } else {
        "AI 请求传输失败，请检查连接设置。"
    }
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    static TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    use std::{
        io::{Read, Write},
        net::TcpListener,
    };

    fn mock(response: Vec<u8>) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/", listener.local_addr().unwrap());
        std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            let _ = stream.read(&mut [0; 4096]);
            let _ = stream.write_all(&response);
        });
        url
    }

    #[test]
    fn rejects_declared_chunked_and_unframed_oversize_before_parse() {
        let _serial = TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner());
        let cases = [
            b"HTTP/1.1 200 OK\r\nContent-Length: 1000\r\nConnection: close\r\n\r\n".to_vec(),
            b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n10\r\n0123456789abcdef\r\n0\r\n\r\n".to_vec(),
            b"HTTP/1.1 200 OK\r\nConnection: close\r\n\r\n0123456789abcdef".to_vec(),
        ];
        for response in cases {
            let url = mock(response);
            let error = tauri::async_runtime::block_on(async {
                receive(client().unwrap().post(url), 8).await
            })
            .unwrap_err();
            assert_eq!(error, "AI 响应超过大小限制。");
        }
    }

    #[test]
    fn errors_and_redirects_never_include_remote_private_content() {
        let _serial = TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner());
        for status in [302, 401, 500] {
            let url = mock(format!("HTTP/1.1 {status} Error\r\nLocation: http://127.0.0.1:1/\r\nContent-Length: 19\r\nConnection: close\r\n\r\nprivate-placeholder").into_bytes());
            let error = post_json("openai", &url, "synthetic-key", "{}").unwrap_err();
            assert!(error.contains(&status.to_string()));
            assert!(!error.contains("private-placeholder"));
            assert!(!error.contains("synthetic-key"));
        }
    }

    #[test]
    fn cancellation_survives_between_requests_and_is_owner_scoped() {
        let _serial = TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner());
        with_scope("test-cancel", || {
            cancel_owner("another-owner");
            assert!(!CANCELLATION.with(|cell| cell
                .borrow()
                .as_ref()
                .unwrap()
                .load(Ordering::Acquire)));
            cancel_owner("test-cancel");
            assert_eq!(
                post_json("openai", "http://127.0.0.1:1/", "", "{}").unwrap_err(),
                "AI 请求已取消。"
            );
        });
        assert!(CANCELLATION.with(|cell| cell.borrow().is_none()));
    }

    #[test]
    fn cancelling_a_stalled_response_drops_reception_and_releases_the_slot() {
        let _serial = TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner());
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/", listener.local_addr().unwrap());
        let (accepted, waiting) = std::sync::mpsc::channel();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let _ = stream.read(&mut [0; 4096]);
            stream
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 1000\r\n\r\n")
                .unwrap();
            accepted.send(()).unwrap();
            std::thread::sleep(Duration::from_millis(400));
        });
        let worker = std::thread::spawn(move || {
            with_scope("stalled-unique", || post_json("openai", &url, "", "{}"))
        });
        waiting.recv_timeout(Duration::from_secs(3)).unwrap();
        let start = std::time::Instant::now();
        cancel_owner("stalled-unique");
        assert_eq!(worker.join().unwrap().unwrap_err(), "AI 请求已取消。");
        assert!(start.elapsed() < Duration::from_millis(350));
        server.join().unwrap();
        let url =
            mock(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}".to_vec());
        assert_eq!(post_json("openai", &url, "", "{}").unwrap(), "{}");
    }

    #[test]
    fn timeout_is_bounded_and_does_not_expose_a_private_url() {
        let _serial = TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner());
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!(
            "http://{}/private-placeholder",
            listener.local_addr().unwrap()
        );
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let _ = stream.read(&mut [0; 4096]);
            std::thread::sleep(Duration::from_millis(200));
        });
        let error = tauri::async_runtime::block_on(async {
            let client = reqwest::Client::builder()
                .no_proxy()
                .timeout(Duration::from_millis(50))
                .build()
                .unwrap();
            receive(client.post(url), 100).await.unwrap_err()
        });
        assert_eq!(error, "AI 请求超时，请稍后重试。");
        server.join().unwrap();
    }

    #[test]
    fn fifth_request_is_rejected_before_network_and_recovers_after_release() {
        let _serial = TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner());
        let slots = SLOTS.try_acquire_many(MAX_CONCURRENT as u32).unwrap();
        assert_eq!(
            post_json("openai", "http://127.0.0.1:1/", "", "{}").unwrap_err(),
            "AI 请求繁忙，请稍后重试。"
        );
        drop(slots);
        assert_eq!(SLOTS.available_permits(), MAX_CONCURRENT);
    }
}
