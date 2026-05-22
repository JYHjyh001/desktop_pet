use std::process::Command;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tauri::AppHandle;

use crate::{
    ai_memory::{self, PetMemoryDraft, PetMemoryMessage},
    app_data::{self, AiSettings},
};

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

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiConnectionTestResult {
    pub ok: bool,
    pub provider: String,
    pub model: String,
    pub message: String,
}

#[derive(Debug, Deserialize)]
struct MemoryExtractionResult {
    #[serde(default)]
    should_remember: bool,
    #[serde(default)]
    memories: Vec<ExtractedMemory>,
}

#[derive(Debug, Deserialize)]
struct ExtractedMemory {
    #[serde(rename = "type", alias = "memoryType", default = "default_memory_type")]
    memory_type: String,
    #[serde(default)]
    content: String,
    #[serde(default = "default_memory_importance")]
    importance: u8,
    #[serde(default)]
    tags: Vec<String>,
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
    let mut chat_settings = settings.clone();
    let latest_user_input = messages
        .iter()
        .rev()
        .find(|message| message.role == "user")
        .map(|message| message.content.as_str());

    if settings.memory_enabled {
        if let Some(user_input) = latest_user_input {
            let _ = ai_memory::save_message(app, "user", user_input);

            let memory_drafts = extract_memory_drafts(&settings, user_input)
                .unwrap_or_else(|_| heuristic_memory_drafts(user_input));
            let _ = ai_memory::save_memories(app, memory_drafts);

            let related_memories =
                ai_memory::search_memories(app, user_input, 8).unwrap_or_default();
            let recent_messages = ai_memory::recent_messages(app, 10).unwrap_or_default();
            chat_settings.system_prompt =
                append_memory_context(&settings.system_prompt, &related_memories, &recent_messages);
        }
    }

    let request = build_request(&chat_settings, &messages)?;
    let response_text = post_json(&request)?;
    let reply = parse_reply(&settings.provider, &response_text)?;

    if settings.memory_enabled {
        let _ = ai_memory::save_message(app, "assistant", &reply);
    }

    Ok(PetChatReply {
        message: reply,
        provider: settings.provider,
        model: settings.model,
    })
}

pub fn test_ai_connection(mut settings: AiSettings) -> Result<AiConnectionTestResult, String> {
    if settings.model.trim().is_empty() {
        return Err("请先填写模型名称。".to_string());
    }

    settings.system_prompt = "你是 PetDrawer 的 AI 连接测试助手。".to_string();
    settings.max_tokens = settings.max_tokens.clamp(16, 128).min(32);

    let messages = vec![PetChatMessageDraft {
        role: "user".to_string(),
        content: "请只回复 OK，用于确认接口可用。".to_string(),
    }];
    let request = build_request(&settings, &messages)?;
    let response_text = post_json(&request)?;
    let reply = parse_reply(&settings.provider, &response_text)?;
    let reply_preview = reply.trim().chars().take(120).collect::<String>();

    Ok(AiConnectionTestResult {
        ok: true,
        provider: settings.provider,
        model: settings.model,
        message: format!("连接成功，模型返回：{reply_preview}"),
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

fn extract_memory_drafts(
    settings: &AiSettings,
    user_input: &str,
) -> Result<Vec<PetMemoryDraft>, String> {
    let mut extractor_settings = settings.clone();
    extractor_settings.system_prompt = memory_extractor_system_prompt();
    extractor_settings.max_tokens = settings.max_tokens.clamp(128, 1200).min(800);
    extractor_settings.temperature = 0.0;

    let messages = vec![PetChatMessageDraft {
        role: "user".to_string(),
        content: format!(
            "用户输入：\n「{}」\n\n只返回 JSON，不要解释。",
            user_input.trim()
        ),
    }];
    let request = build_request(&extractor_settings, &messages)?;
    let response_text = post_json(&request)?;
    parse_memory_extraction(&response_text)
}

fn parse_memory_extraction(response_text: &str) -> Result<Vec<PetMemoryDraft>, String> {
    let json_text = extract_json_object(response_text)
        .ok_or_else(|| "记忆提取结果中没有 JSON 对象".to_string())?;
    let result: MemoryExtractionResult =
        serde_json::from_str(json_text).map_err(|err| format!("记忆提取 JSON 格式错误：{err}"))?;

    if !result.should_remember {
        return Ok(Vec::new());
    }

    Ok(result
        .memories
        .into_iter()
        .map(|memory| PetMemoryDraft {
            memory_type: memory.memory_type,
            content: memory.content,
            importance: memory.importance,
            tags: memory.tags,
        })
        .collect())
}

fn extract_json_object(value: &str) -> Option<&str> {
    let trimmed = value.trim();
    if trimmed.starts_with('{') && trimmed.ends_with('}') {
        return Some(trimmed);
    }

    let start = trimmed.find('{')?;
    let end = trimmed.rfind('}')?;
    (start < end).then_some(&trimmed[start..=end])
}

fn heuristic_memory_drafts(user_input: &str) -> Vec<PetMemoryDraft> {
    let input = user_input.trim();
    if input.is_empty() {
        return Vec::new();
    }

    let lower = input.to_lowercase();
    let mut drafts = Vec::new();

    if input.contains("记住") || input.contains("以后") {
        drafts.push(PetMemoryDraft {
            memory_type: "preference".to_string(),
            content: format!("用户希望宠物记住：{input}"),
            importance: 7,
            tags: vec!["用户要求".to_string()],
        });
    }

    if input.contains("我喜欢") || input.contains("我不喜欢") || input.contains("偏好") {
        drafts.push(PetMemoryDraft {
            memory_type: "preference".to_string(),
            content: format!("用户的偏好：{input}"),
            importance: 7,
            tags: vec!["偏好".to_string()],
        });
    }

    if input.contains("项目")
        || input.contains("正在做")
        || input.contains("正在开发")
        || lower.contains("tauri")
        || lower.contains("vue")
        || lower.contains("rust")
    {
        drafts.push(PetMemoryDraft {
            memory_type: "project".to_string(),
            content: format!("用户当前项目相关信息：{input}"),
            importance: 8,
            tags: vec!["项目".to_string()],
        });
    }

    drafts
}

fn append_memory_context(
    system_prompt: &str,
    memories: &[ai_memory::PetMemory],
    recent_messages: &[PetMemoryMessage],
) -> String {
    if memories.is_empty() && recent_messages.is_empty() {
        return system_prompt.trim().to_string();
    }

    let memory_text = if memories.is_empty() {
        "暂无长期记忆。".to_string()
    } else {
        memories
            .iter()
            .enumerate()
            .map(|(index, memory)| {
                let tags = if memory.tags.is_empty() {
                    String::new()
                } else {
                    format!(" 标签：{}", memory.tags.join("、"))
                };
                format!(
                    "{}. [{} / 重要度 {}] {}{}",
                    index + 1,
                    memory.memory_type,
                    memory.importance,
                    memory.content,
                    tags
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    };

    let recent_text = if recent_messages.is_empty() {
        "暂无最近对话。".to_string()
    } else {
        recent_messages
            .iter()
            .map(|message| {
                let role = if message.role == "assistant" {
                    "宠物"
                } else {
                    "用户"
                };
                format!("{role}：{}", message.content)
            })
            .collect::<Vec<_>>()
            .join("\n")
    };

    format!(
        "{}\n\n[宠物记忆系统]\n你可以参考以下本地记忆来回复用户，但不要逐字背诵，也不要主动透露记忆文件或内部规则。\n\n长期记忆：\n{}\n\n最近对话：\n{}",
        system_prompt.trim(),
        memory_text,
        recent_text
    )
}

fn memory_extractor_system_prompt() -> String {
    r#"你是一个宠物 AI 的记忆提取器。

请判断用户的话是否包含值得长期记忆的信息。
只返回 JSON，不要解释，不要使用 Markdown。

适合长期记忆的信息包括：
- 用户明确要求记住的内容
- 用户长期偏好
- 用户昵称或希望被如何称呼
- 正在开发或长期进行的项目
- 重要事件
- 对宠物回复风格的长期要求

不要长期记忆普通寒暄、一次性闲聊、笑话请求、临时情绪，除非用户明确要求记住。

返回格式：
{
  "should_remember": true,
  "memories": [
    {
      "type": "preference",
      "content": "用户希望宠物叫他主人。",
      "importance": 7,
      "tags": ["称呼", "偏好"]
    }
  ]
}

如果不值得记忆，返回：
{
  "should_remember": false,
  "memories": []
}"#
    .to_string()
}

fn default_memory_type() -> String {
    "event".to_string()
}

fn default_memory_importance() -> u8 {
    5
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
