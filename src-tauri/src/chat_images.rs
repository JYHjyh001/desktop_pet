//! Images stay in local message content as Markdown, never in subsequent model context.
use serde_json::Value;

pub const MAX_IMAGE_SOURCE_BYTES: usize = 4 * 1024 * 1024;
pub const MAX_MESSAGE_BYTES: usize = 8 * 1024 * 1024;
const MAX_IMAGES: usize = 8;

pub fn valid_image_source(source: &str) -> bool {
    if crate::chat_media::attachment_id(source).is_some() {
        return true;
    }
    if source.len() > MAX_IMAGE_SOURCE_BYTES {
        return false;
    }
    if let Some(data) = source.strip_prefix("data:image/") {
        let Some((kind, payload)) = data.split_once(";base64,") else {
            return false;
        };
        return matches!(kind, "png" | "jpeg" | "webp" | "gif")
            && !payload.is_empty()
            && payload
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"+/=".contains(&b));
    }
    reqwest::Url::parse(source).is_ok_and(|url| {
        url.scheme() == "https"
            && url.host_str().is_some()
            && url.username().is_empty()
            && url.password().is_none()
    })
}

/// Complete Markdown image ranges. Balanced URL parentheses and angle destinations are supported.
pub fn image_ranges(text: &str) -> Vec<(usize, usize)> {
    let mut ranges = Vec::new();
    let mut offset = 0;
    while let Some(start) = text[offset..].find("![") {
        let start = start + offset;
        let Some(label_end) = text[start + 2..].find("](") else {
            break;
        };
        let target = start + 2 + label_end + 2;
        let mut depth = 1;
        let mut end = None;
        for (index, c) in text[target..].char_indices() {
            match c {
                '(' => depth += 1,
                ')' => {
                    depth -= 1;
                    if depth == 0 {
                        end = Some(target + index + 1);
                        break;
                    }
                }
                _ => {}
            }
        }
        if let Some(end) = end {
            ranges.push((start, end));
            offset = end;
        } else {
            offset = target;
        }
    }
    ranges
}

pub fn text_context(text: &str) -> String {
    let mut output = String::new();
    let mut offset = 0;
    for (start, end) in image_ranges(text) {
        output.push_str(&text[offset..start]);
        output.push_str("[图片]");
        offset = end;
    }
    output.push_str(&text[offset..]);
    output
}

/// Retain whole image tokens, including on export/import; never truncate a Base64 payload.
pub fn stored_content(text: &str, plain_text_limit: usize) -> String {
    if image_ranges(text).is_empty() {
        return text.chars().take(plain_text_limit).collect();
    }
    if text.len() <= MAX_MESSAGE_BYTES {
        return text.to_string();
    }
    format!(
        "{}\n图片消息过大，未保存图片。",
        text_context(text)
            .chars()
            .take(plain_text_limit)
            .collect::<String>()
    )
}

fn push_image(parts: &mut Vec<String>, source: &str) {
    let image_count = parts
        .iter()
        .filter(|part| part.starts_with("![图片]("))
        .count();
    if !valid_image_source(source)
        || image_count >= MAX_IMAGES
        || parts.iter().map(String::len).sum::<usize>() + source.len() > MAX_MESSAGE_BYTES - 1024
    {
        parts.push("图片格式不支持或超过大小限制。".to_string());
        return;
    }
    // Encode Markdown delimiters in remote URLs, while leaving data URLs unchanged.
    let source = source
        .replace('(', "%28")
        .replace(')', "%29")
        .replace('<', "%3C")
        .replace('>', "%3E");
    parts.push(format!("![图片](<{source}>)"));
}

fn push_content(parts: &mut Vec<String>, content: &Value) {
    if let Some(text) = content.as_str() {
        parts.push(text.to_string());
    } else if let Some(blocks) = content.as_array() {
        for block in blocks {
            if let Some(text) = block.get("text").and_then(Value::as_str) {
                parts.push(text.to_string());
            }
            let source = block
                .get("image_url")
                .and_then(|image| {
                    image
                        .as_str()
                        .or_else(|| image.get("url").and_then(Value::as_str))
                })
                .or_else(|| {
                    if block.get("type").and_then(Value::as_str) == Some("image") {
                        block.get("url").and_then(Value::as_str)
                    } else {
                        None
                    }
                });
            if let Some(source) = source {
                push_image(parts, source);
            }
            if let Some(inline) = block.get("inlineData").or_else(|| block.get("inline_data")) {
                if let (Some(mime), Some(data)) = (
                    inline
                        .get("mimeType")
                        .or_else(|| inline.get("mime_type"))
                        .and_then(Value::as_str),
                    inline.get("data").and_then(Value::as_str),
                ) {
                    push_image(parts, &format!("data:{mime};base64,{data}"));
                }
            }
            if let Some(source) = block.get("source") {
                if let (Some(mime), Some(data)) = (
                    source.get("media_type").and_then(Value::as_str),
                    source.get("data").and_then(Value::as_str),
                ) {
                    push_image(parts, &format!("data:{mime};base64,{data}"));
                } else if let Some(url) = source.get("url").and_then(Value::as_str) {
                    push_image(parts, url);
                }
            }
        }
    }
}

pub fn parse_response(provider: &str, value: &Value) -> String {
    let mut parts = Vec::new();
    match provider {
        "anthropic" => push_content(&mut parts, &value["content"]),
        "gemini" => push_content(&mut parts, &value["candidates"][0]["content"]["parts"]),
        "ollama" => {
            push_content(&mut parts, &value["message"]["content"]);
            if parts.is_empty() {
                push_content(&mut parts, &value["response"]);
            }
        }
        _ => {
            let message = &value["choices"][0]["message"];
            push_content(&mut parts, &message["content"]);
            push_content(&mut parts, &message["images"]);
        }
    }
    parts.join("\n\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn accepts_mixed_and_image_only_replies() {
        let value = json!({"choices":[{"message":{"content":[{"type":"text","text":"你好"},{"type":"image_url","image_url":{"url":"https://example.test/a.png"}}]}}]});
        let reply = parse_response("custom", &value);
        assert!(reply.starts_with("你好"));
        assert!(reply.contains("![图片](<https://example.test/a.png>)"));
        let images = json!({"choices":[{"message":{"content":null,"images":[{"image_url":{"url":"data:image/png;base64,AAAA"}}]}}]});
        assert!(parse_response("openai", &images).contains("data:image/png;base64,AAAA"));
        let gemini = json!({"candidates":[{"content":{"parts":[{"inlineData":{"mimeType":"image/png","data":"AAAA"}}]}}]});
        assert!(parse_response("gemini", &gemini).contains("data:image/png;base64,AAAA"));
    }

    #[test]
    fn blocks_unsafe_images_and_limits_payloads() {
        for url in [
            "file:///secret.png",
            "javascript:alert(1)",
            "http://example.test/a.png",
            "data:image/svg+xml;base64,AAAA",
            "https://fixture@example.test/a.png",
        ] {
            assert!(!valid_image_source(url));
        }
        assert!(!valid_image_source(&format!(
            "data:image/png;base64,{}",
            "A".repeat(MAX_IMAGE_SOURCE_BYTES)
        )));
    }

    #[test]
    fn preserves_history_but_removes_images_from_context() {
        let text = format!(
            "你好 ![图片](data:image/png;base64,{}) 再见",
            "A".repeat(3000)
        );
        assert_eq!(stored_content(&text, 1200), text);
        assert_eq!(text_context(&text), "你好 [图片] 再见");
        assert_eq!(
            text_context("![图](https://example.test/a(1).png)![图](<https://example.test/b.png>)"),
            "[图片][图片]"
        );
        assert_eq!(stored_content("普通文字", 2), "普通");
    }
}
