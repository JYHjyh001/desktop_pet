use std::process::Command;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tauri::AppHandle;

use crate::app_data::{self, AiSettings};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PetChatMessageDraft {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PetChatReply {
    pub message: String,
    pub provider: String,
    pub model: String,
}

pub fn send_pet_chat_message(
    app: &AppHandle,
    messages: Vec<PetChatMessageDraft>,
) -> Result<PetChatReply, String> {
    let config = app_data::read_config(app)?;
    let settings = config.ai;

    if !settings.enabled {
        return Err("请先在抽屉设置的“AI 接口”中启用宠物聊天 API。".to_string());
    }

    if settings.model.trim().is_empty() {
        return Err("请先在“AI 接口”中填写模型名称。".to_string());
    }

    let messages = normalize_messages(messages)?;
    let request = build_request(&settings, &messages)?;
    let response_text = post_json(&request)?;
    let reply = parse_reply(&settings.provider, &response_text)?;

    Ok(PetChatReply {
        message: reply,
        provider: settings.provider,
        model: settings.model,
    })
}

struct AiHttpRequest {
    provider: String,
    url: String,
    api_key: String,
    body: String,
}

fn normalize_messages(
    messages: Vec<PetChatMessageDraft>,
) -> Result<Vec<PetChatMessageDraft>, String> {
    let normalized = messages
        .into_iter()
        .filter_map(|message| {
            let role = normalize_role(&message.role)?;
            let content = message.content.trim().to_string();
            if content.is_empty() {
                return None;
            }

            Some(PetChatMessageDraft { role, content })
        })
        .collect::<Vec<_>>();

    if normalized.is_empty() {
        return Err("请输入要和宠物说的话。".to_string());
    }

    Ok(normalized)
}

fn normalize_role(role: &str) -> Option<String> {
    match role.trim().to_lowercase().as_str() {
        "user" => Some("user".to_string()),
        "assistant" => Some("assistant".to_string()),
        _ => None,
    }
}

fn build_request(
    settings: &AiSettings,
    messages: &[PetChatMessageDraft],
) -> Result<AiHttpRequest, String> {
    if settings.base_url.trim().is_empty() {
        return Err("请先在“AI 接口”中填写 Base URL。".to_string());
    }

    let provider = settings.provider.trim().to_lowercase();
    match provider.as_str() {
        "anthropic" => build_anthropic_request(settings, messages),
        "gemini" => build_gemini_request(settings, messages),
        "ollama" => build_ollama_request(settings, messages),
        _ => build_openai_compatible_request(settings, messages),
    }
}

fn build_openai_compatible_request(
    settings: &AiSettings,
    messages: &[PetChatMessageDraft],
) -> Result<AiHttpRequest, String> {
    let provider = settings.provider.trim().to_lowercase();
    let api_key = settings.api_key.trim().to_string();

    if matches!(provider.as_str(), "openai" | "deepseek") && api_key.is_empty() {
        return Err("请先在“AI 接口”中填写 API Key。".to_string());
    }

    let mut request_messages = Vec::new();
    let system_prompt = settings.system_prompt.trim();
    if !system_prompt.is_empty() {
        request_messages.push(json!({
            "role": "system",
            "content": system_prompt,
        }));
    }

    request_messages.extend(messages.iter().map(|message| {
        json!({
            "role": message.role.as_str(),
            "content": message.content.as_str(),
        })
    }));

    let body = json!({
        "model": settings.model.trim(),
        "messages": request_messages,
        "temperature": settings.temperature,
        "max_tokens": settings.max_tokens,
    });

    Ok(AiHttpRequest {
        provider,
        url: chat_completions_url(&settings.base_url),
        api_key,
        body: body.to_string(),
    })
}

fn build_anthropic_request(
    settings: &AiSettings,
    messages: &[PetChatMessageDraft],
) -> Result<AiHttpRequest, String> {
    let api_key = settings.api_key.trim().to_string();
    if api_key.is_empty() {
        return Err("请先在“AI 接口”中填写 API Key。".to_string());
    }

    let body_messages = messages
        .iter()
        .map(|message| {
            json!({
                "role": message.role.as_str(),
                "content": message.content.as_str(),
            })
        })
        .collect::<Vec<_>>();

    let body = json!({
        "model": settings.model.trim(),
        "system": settings.system_prompt.trim(),
        "messages": body_messages,
        "temperature": settings.temperature,
        "max_tokens": settings.max_tokens,
    });

    Ok(AiHttpRequest {
        provider: "anthropic".to_string(),
        url: anthropic_messages_url(&settings.base_url),
        api_key,
        body: body.to_string(),
    })
}

fn build_gemini_request(
    settings: &AiSettings,
    messages: &[PetChatMessageDraft],
) -> Result<AiHttpRequest, String> {
    let api_key = settings.api_key.trim().to_string();
    if api_key.is_empty() {
        return Err("请先在“AI 接口”中填写 API Key。".to_string());
    }

    let contents = messages
        .iter()
        .map(|message| {
            let role = if message.role == "assistant" {
                "model"
            } else {
                "user"
            };
            json!({
                "role": role,
                "parts": [{ "text": message.content.as_str() }],
            })
        })
        .collect::<Vec<_>>();

    let body = json!({
        "contents": contents,
        "systemInstruction": {
            "parts": [{ "text": settings.system_prompt.trim() }]
        },
        "generationConfig": {
            "temperature": settings.temperature,
            "maxOutputTokens": settings.max_tokens,
        }
    });

    Ok(AiHttpRequest {
        provider: "gemini".to_string(),
        url: gemini_generate_url(&settings.base_url, &settings.model, &api_key),
        api_key: String::new(),
        body: body.to_string(),
    })
}

fn build_ollama_request(
    settings: &AiSettings,
    messages: &[PetChatMessageDraft],
) -> Result<AiHttpRequest, String> {
    let mut request_messages = Vec::new();
    let system_prompt = settings.system_prompt.trim();
    if !system_prompt.is_empty() {
        request_messages.push(json!({
            "role": "system",
            "content": system_prompt,
        }));
    }

    request_messages.extend(messages.iter().map(|message| {
        json!({
            "role": message.role.as_str(),
            "content": message.content.as_str(),
        })
    }));

    let body = json!({
        "model": settings.model.trim(),
        "messages": request_messages,
        "stream": false,
        "options": {
            "temperature": settings.temperature,
            "num_predict": settings.max_tokens,
        }
    });

    Ok(AiHttpRequest {
        provider: "ollama".to_string(),
        url: ollama_chat_url(&settings.base_url),
        api_key: settings.api_key.trim().to_string(),
        body: body.to_string(),
    })
}

fn chat_completions_url(base_url: &str) -> String {
    let base_url = base_url.trim().trim_end_matches('/');
    if base_url.ends_with("/chat/completions") {
        base_url.to_string()
    } else {
        format!("{base_url}/chat/completions")
    }
}

fn anthropic_messages_url(base_url: &str) -> String {
    let base_url = base_url.trim().trim_end_matches('/');
    if base_url.ends_with("/messages") {
        base_url.to_string()
    } else {
        format!("{base_url}/v1/messages")
    }
}

fn ollama_chat_url(base_url: &str) -> String {
    let base_url = base_url.trim().trim_end_matches('/');
    if base_url.ends_with("/api/chat") {
        base_url.to_string()
    } else {
        format!("{base_url}/api/chat")
    }
}

fn gemini_generate_url(base_url: &str, model: &str, api_key: &str) -> String {
    let base_url = base_url.trim().trim_end_matches('/');
    if base_url.contains(":generateContent") {
        return if base_url.contains('?') {
            base_url.to_string()
        } else {
            format!("{base_url}?key={api_key}")
        };
    }

    format!(
        "{base_url}/v1beta/models/{}:generateContent?key={api_key}",
        model.trim()
    )
}

#[cfg(target_os = "windows")]
fn post_json(request: &AiHttpRequest) -> Result<String, String> {
    use std::os::windows::process::CommandExt;

    const CREATE_NO_WINDOW: u32 = 0x08000000;
    const SCRIPT: &str = r#"
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
$ProgressPreference = 'SilentlyContinue'
Add-Type -AssemblyName System.Net.Http

$Url = $env:PETDRAWER_AI_URL
$Body = $env:PETDRAWER_AI_BODY
$Provider = $env:PETDRAWER_AI_PROVIDER
$ApiKey = $env:PETDRAWER_AI_API_KEY

if ([string]::IsNullOrWhiteSpace($Url)) {
  throw 'AI 请求地址为空'
}

$client = [System.Net.Http.HttpClient]::new()
$client.Timeout = [TimeSpan]::FromSeconds(60)
$client.DefaultRequestHeaders.UserAgent.ParseAdd('PetDrawer-AI')

if ($Provider -eq 'anthropic') {
  if (![string]::IsNullOrWhiteSpace($ApiKey)) {
    $client.DefaultRequestHeaders.Add('x-api-key', $ApiKey)
  }
  $client.DefaultRequestHeaders.Add('anthropic-version', '2023-06-01')
} elseif (![string]::IsNullOrWhiteSpace($ApiKey)) {
  $client.DefaultRequestHeaders.Authorization = [System.Net.Http.Headers.AuthenticationHeaderValue]::new('Bearer', $ApiKey)
}

$content = [System.Net.Http.StringContent]::new($Body, [System.Text.Encoding]::UTF8, 'application/json')
$response = $client.PostAsync($Url, $content).GetAwaiter().GetResult()
$text = $response.Content.ReadAsStringAsync().GetAwaiter().GetResult()

if (-not $response.IsSuccessStatusCode) {
  [Console]::Error.Write($text)
  exit 1
}

[Console]::Out.Write($text)
"#;

    let output = Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            SCRIPT,
        ])
        .env("PETDRAWER_AI_URL", &request.url)
        .env("PETDRAWER_AI_BODY", &request.body)
        .env("PETDRAWER_AI_PROVIDER", &request.provider)
        .env("PETDRAWER_AI_API_KEY", &request.api_key)
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|err| format!("无法启动 AI 请求：{err}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if stderr.is_empty() {
            "AI 请求失败，但服务没有返回错误内容。".to_string()
        } else {
            format!("AI 请求失败：{}", readable_api_error(&stderr))
        });
    }

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if stdout.is_empty() {
        return Err("AI 服务返回了空内容。".to_string());
    }

    Ok(stdout)
}

#[cfg(not(target_os = "windows"))]
fn post_json(_request: &AiHttpRequest) -> Result<String, String> {
    Err("当前宠物聊天 API 请求仅支持 Windows 版本。".to_string())
}

fn parse_reply(provider: &str, response_text: &str) -> Result<String, String> {
    let value: Value = serde_json::from_str(response_text)
        .map_err(|err| format!("AI 返回内容不是有效 JSON：{err}"))?;

    let message = match provider {
        "anthropic" => parse_anthropic_reply(&value),
        "gemini" => parse_gemini_reply(&value),
        "ollama" => parse_ollama_reply(&value),
        _ => parse_openai_compatible_reply(&value),
    }
    .map(str::trim)
    .filter(|content| !content.is_empty())
    .map(str::to_string)
    .ok_or_else(|| "AI 返回内容中没有可显示的回复。".to_string())?;

    Ok(message)
}

fn parse_openai_compatible_reply(value: &Value) -> Option<&str> {
    value
        .get("choices")?
        .as_array()?
        .first()?
        .get("message")?
        .get("content")?
        .as_str()
}

fn parse_anthropic_reply(value: &Value) -> Option<&str> {
    value
        .get("content")?
        .as_array()?
        .iter()
        .find_map(|item| item.get("text")?.as_str())
}

fn parse_gemini_reply(value: &Value) -> Option<&str> {
    value
        .get("candidates")?
        .as_array()?
        .first()?
        .get("content")?
        .get("parts")?
        .as_array()?
        .iter()
        .find_map(|item| item.get("text")?.as_str())
}

fn parse_ollama_reply(value: &Value) -> Option<&str> {
    value
        .get("message")
        .and_then(|message| message.get("content"))
        .and_then(Value::as_str)
        .or_else(|| value.get("response").and_then(Value::as_str))
}

fn readable_api_error(stderr: &str) -> String {
    serde_json::from_str::<Value>(stderr)
        .ok()
        .and_then(|value| {
            value
                .get("error")
                .and_then(|error| {
                    error
                        .get("message")
                        .and_then(Value::as_str)
                        .or_else(|| error.as_str())
                })
                .map(str::to_string)
        })
        .unwrap_or_else(|| stderr.to_string())
}
