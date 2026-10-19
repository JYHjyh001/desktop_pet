//! Validated local attachments and credential-free bounded image downloads.
use base64::{engine::general_purpose::STANDARD, Engine};
use image::{AnimationDecoder, ImageDecoder};
use md5::{Digest, Md5};
use std::{
    fs,
    io::{Cursor, Write},
    net::IpAddr,
    path::Path,
    sync::Mutex,
    time::Duration,
};
use tauri::AppHandle;

pub const MAX_IMAGE_BYTES: usize = 4 * 1024 * 1024;
const ATTACHMENT_QUOTA: u64 = 128 * 1024 * 1024;
const MAX_DECODE_BYTES: u64 = 64 * 1024 * 1024;
const ATTACHMENT_PREFIX: &str = "pet-attachment:";
static WRITE_LOCK: Mutex<()> = Mutex::new(());
static DOWNLOADS: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(2);
static DECODERS: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(2);

pub fn attachment_id(source: &str) -> Option<&str> {
    source
        .strip_prefix(ATTACHMENT_PREFIX)
        .filter(|id| id.len() == 32 && id.bytes().all(|b| b.is_ascii_hexdigit()))
}

fn limits() -> image::Limits {
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(4096);
    limits.max_image_height = Some(4096);
    limits.max_alloc = Some(MAX_DECODE_BYTES);
    limits
}

pub fn validate_image(bytes: &[u8]) -> Result<&'static str, String> {
    let _slot = DECODERS
        .try_acquire()
        .map_err(|_| "图片解码繁忙，请稍后重试")?;
    if bytes.len() > MAX_IMAGE_BYTES {
        return Err("图片超过 4 MiB 限制。".into());
    }
    let format = image::guess_format(bytes).map_err(|_| "图片格式无效。")?;
    let mime = match format {
        image::ImageFormat::Png => "image/png",
        image::ImageFormat::Jpeg => "image/jpeg",
        image::ImageFormat::WebP => "image/webp",
        image::ImageFormat::Gif => "image/gif",
        _ => return Err("图片格式不支持。".into()),
    };
    let dimensions = image::ImageReader::with_format(Cursor::new(bytes), format)
        .into_dimensions()
        .map_err(|_| "图片头部无效。")?;
    if dimensions.0 > 4096
        || dimensions.1 > 4096
        || u64::from(dimensions.0) * u64::from(dimensions.1) > 16_000_000
    {
        return Err("图片尺寸或像素超过限制。".into());
    }
    // Animation has a cumulative decoded budget, not merely a first-frame check.
    match format {
        image::ImageFormat::Gif => {
            let mut decoder = image::codecs::gif::GifDecoder::new(Cursor::new(bytes))
                .map_err(|_| "GIF 头部无效")?;
            decoder
                .set_limits(limits())
                .map_err(|_| "图片超出解码限制")?;
            validate_frames(decoder.into_frames())?;
        }
        image::ImageFormat::WebP => {
            let mut decoder = image::codecs::webp::WebPDecoder::new(Cursor::new(bytes))
                .map_err(|_| "WebP 头部无效")?;
            decoder
                .set_limits(limits())
                .map_err(|_| "图片超出解码限制")?;
            if decoder.has_animation() {
                validate_frames(decoder.into_frames())?;
            } else {
                image::DynamicImage::from_decoder(decoder).map_err(|_| "图片解码失败或超出限制")?;
            }
        }
        image::ImageFormat::Png => {
            let mut decoder = image::codecs::png::PngDecoder::new(Cursor::new(bytes))
                .map_err(|_| "PNG 头部无效")?;
            decoder
                .set_limits(limits())
                .map_err(|_| "图片超出解码限制")?;
            if decoder.is_apng().map_err(|_| "PNG 动画头部无效")? {
                validate_frames(decoder.apng().map_err(|_| "PNG 动画无效")?.into_frames())?;
            } else {
                let mut reader = image::ImageReader::with_format(Cursor::new(bytes), format);
                reader.limits(limits());
                reader.decode().map_err(|_| "图片解码失败或超出限制")?;
            }
        }
        _ => {
            let mut reader = image::ImageReader::with_format(Cursor::new(bytes), format);
            reader.limits(limits());
            reader.decode().map_err(|_| "图片解码失败或超出限制")?;
        }
    }
    Ok(mime)
}

fn validate_frames(frames: image::Frames<'_>) -> Result<(), String> {
    let mut total = 0u64;
    for (index, frame) in frames.enumerate() {
        if index >= 32 {
            return Err("图片动画超过 32 帧限制。".into());
        }
        let frame = frame.map_err(|_| "图片动画解码失败")?;
        total += frame.buffer().as_raw().len() as u64;
        if total > MAX_DECODE_BYTES {
            return Err("图片动画超过解码内存限制。".into());
        }
    }
    Ok(())
}

fn inline_bytes(source: &str) -> Result<Vec<u8>, String> {
    if source.len() > crate::chat_images::MAX_IMAGE_SOURCE_BYTES {
        return Err("图片编码超过大小限制。".into());
    }
    let (kind, payload) = source.split_once(";base64,").ok_or("图片编码无效")?;
    if !matches!(
        kind,
        "data:image/png" | "data:image/jpeg" | "data:image/gif" | "data:image/webp"
    ) {
        return Err("图片格式不支持".into());
    }
    STANDARD.decode(payload).map_err(|_| "图片编码无效".into())
}

fn store_at(dir: &Path, bytes: &[u8]) -> Result<String, String> {
    validate_image(bytes)?;
    let _guard = WRITE_LOCK.lock().map_err(|_| "附件写入锁不可用")?;
    fs::create_dir_all(dir).map_err(|_| "无法创建附件目录")?;
    let id = format!("{:x}", Md5::digest(bytes));
    let path = dir.join(&id);
    if path.exists() {
        if fs::read(&path).map_err(|_| "无法校验附件")? != bytes {
            return Err("附件标识冲突，未覆盖原文件".into());
        }
        return Ok(format!("{ATTACHMENT_PREFIX}{id}"));
    }
    let total = fs::read_dir(dir)
        .map_err(|_| "无法检查附件配额")?
        .try_fold(0u64, |total, entry| {
            entry
                .and_then(|entry| entry.metadata())
                .map(|metadata| total.saturating_add(metadata.len()))
        })
        .map_err(|_| "无法检查附件配额")?;
    if total.saturating_add(bytes.len() as u64) > ATTACHMENT_QUOTA {
        return Err("本机聊天附件已达到 128 MiB 配额，未删除已有附件。".into());
    }
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .map_err(|_| "无法写入附件")?;
    if file.write_all(bytes).and_then(|_| file.sync_all()).is_err() {
        drop(file);
        let _ = fs::remove_file(&path); // Only our incomplete newly created file.
        return Err("附件保存失败".into());
    }
    Ok(format!("{ATTACHMENT_PREFIX}{id}"))
}

/// Only new inline images are externalized; old history is never rewritten.
pub fn externalize_inline(app: &AppHandle, text: &str) -> Result<String, String> {
    let dir = crate::app_data::memory_dir(app)?.join("chat-attachments");
    let mut output = String::new();
    let mut offset = 0;
    for (start, end) in crate::chat_images::image_ranges(text) {
        output.push_str(&text[offset..start]);
        let token = &text[start..end];
        let source = token
            .split_once("](")
            .map(|(_, target)| target[..target.len() - 1].trim().trim_matches(['<', '>']))
            .unwrap_or("");
        if source.starts_with("data:image/") {
            let reference = store_at(&dir, &inline_bytes(source)?)?;
            output.push_str(&format!("![图片]({reference})"));
        } else {
            output.push_str(token);
        }
        offset = end;
    }
    output.push_str(&text[offset..]);
    Ok(output)
}

pub fn public_address(address: IpAddr) -> bool {
    match address {
        IpAddr::V4(ip) => {
            let [a, b, c, _] = ip.octets();
            !ip.is_private()
                && !ip.is_loopback()
                && !ip.is_link_local()
                && !ip.is_broadcast()
                && !ip.is_documentation()
                && !ip.is_unspecified()
                && !ip.is_multicast()
                && a != 0
                && a < 240
                && !(a == 100 && (64..=127).contains(&b))
                && !(a == 198 && matches!(b, 18 | 19))
                && !(a == 192 && b == 0 && c == 0)
        }
        IpAddr::V6(ip) => {
            if let Some(v4) = ip.to_ipv4_mapped() {
                return public_address(IpAddr::V4(v4));
            }
            // Global unicast only; exclude documentation, transition and tunnelling ranges.
            let s = ip.segments();
            (s[0] & 0xe000) == 0x2000
                && s[0] != 0x2002
                && !(s[0] == 0x2001 && (s[1] <= 0x1ff || s[1] == 0xdb8))
        }
    }
}

async fn download(source: &str) -> Result<Vec<u8>, String> {
    let _slot = DOWNLOADS
        .try_acquire()
        .map_err(|_| "图片加载繁忙，请稍后重试")?;
    if source.len() > 8192 {
        return Err("图片地址超过长度限制".into());
    }
    let url = reqwest::Url::parse(source).map_err(|_| "图片地址无效")?;
    if url.scheme() != "https" || !url.username().is_empty() || url.password().is_some() {
        return Err("图片只允许 HTTPS 地址".into());
    }
    let host = url
        .host_str()
        .ok_or("图片主机无效")?
        .trim_matches(['[', ']']);
    let port = url.port_or_known_default().ok_or("图片端口无效")?;
    let addresses: Vec<_> = tokio::net::lookup_host((host, port))
        .await
        .map_err(|_| "无法解析图片主机")?
        .collect();
    if addresses.is_empty()
        || addresses
            .iter()
            .any(|address| !public_address(address.ip()))
    {
        return Err("图片不允许访问本机、内网或保留地址".into());
    }
    // Pin DNS results and bypass proxies so later resolution/proxying cannot bypass the address policy.
    let client = reqwest::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .resolve_to_addrs(host, &addresses)
        .timeout(Duration::from_secs(15))
        .connect_timeout(Duration::from_secs(5))
        .build()
        .map_err(|_| "图片连接初始化失败")?;
    let mut response = client.get(url).send().await.map_err(|_| "图片下载失败")?;
    if !response.status().is_success() {
        return Err("图片服务返回错误或重定向，未继续下载".into());
    }
    if response
        .content_length()
        .is_some_and(|length| length > MAX_IMAGE_BYTES as u64)
    {
        return Err("图片超过大小限制".into());
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|_| "图片下载失败")? {
        if chunk.len() > MAX_IMAGE_BYTES.saturating_sub(bytes.len()) {
            return Err("图片超过大小限制".into());
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}

pub fn hydrate_attachments(app: &AppHandle, text: &str) -> Result<String, String> {
    let mut output = String::new();
    let mut offset = 0;
    for (start, end) in crate::chat_images::image_ranges(text) {
        output.push_str(&text[offset..start]);
        let token = &text[start..end];
        let source = token
            .split_once("](")
            .map(|(_, target)| target[..target.len() - 1].trim().trim_matches(['<', '>']))
            .unwrap_or("");
        if attachment_id(source).is_some() {
            output.push_str(&format!("![图片](<{}>)", load_image(app, source)?));
        } else {
            output.push_str(token);
        }
        if output.len() > 32 * 1024 * 1024 {
            return Err("附件导出超过大小限制".into());
        }
        offset = end;
    }
    output.push_str(&text[offset..]);
    Ok(output)
}

pub fn load_image(app: &AppHandle, source: &str) -> Result<String, String> {
    let bytes = if let Some(id) = attachment_id(source) {
        let path = crate::app_data::memory_dir(app)?
            .join("chat-attachments")
            .join(id);
        if fs::metadata(&path).map_err(|_| "附件不存在")?.len() > MAX_IMAGE_BYTES as u64 {
            return Err("附件超过大小限制".into());
        }
        fs::read(path).map_err(|_| "无法读取附件")?
    } else if source.starts_with("data:") {
        inline_bytes(source)?
    } else {
        crate::ai_transport::block_on_cancellable(async {
            tokio::time::timeout(Duration::from_secs(15), download(source))
                .await
                .map_err(|_| "图片下载超时".to_owned())?
        })?
    };
    let mime = validate_image(&bytes)?;
    Ok(format!("data:{mime};base64,{}", STANDARD.encode(bytes)))
}

#[cfg(test)]
mod tests {
    use super::*;
    static TEST_LOCK: Mutex<()> = Mutex::new(());
    #[test]
    fn blocks_local_private_reserved_and_mapped_addresses() {
        for ip in [
            "127.0.0.1",
            "10.0.0.1",
            "172.16.0.1",
            "192.168.1.1",
            "169.254.1.1",
            "100.64.0.1",
            "0.0.0.0",
            "224.0.0.1",
            "198.18.0.1",
            "192.0.2.1",
            "::1",
            "fc00::1",
            "fe80::1",
            "::ffff:127.0.0.1",
            "2001:db8::1",
            "2002:7f00:1::1",
        ] {
            assert!(!public_address(ip.parse().unwrap()), "{ip}");
        }
        assert!(public_address(IpAddr::V4(std::net::Ipv4Addr::new(
            8, 8, 8, 8
        ))));
    }
    #[test]
    fn attachment_references_cannot_escape_the_store() {
        let _guard = TEST_LOCK.lock().unwrap();
        assert!(attachment_id("pet-attachment:../../secret").is_none());
        assert!(attachment_id("pet-attachment:0123456789abcdef0123456789abcdef").is_some());
        assert!(validate_image(&vec![0; MAX_IMAGE_BYTES + 1]).is_err());
        assert!(validate_image(b"invalid image").is_err());
    }

    fn png(width: u32) -> Vec<u8> {
        let mut bytes = Cursor::new(Vec::new());
        image::DynamicImage::new_rgba8(width, 1)
            .write_to(&mut bytes, image::ImageFormat::Png)
            .unwrap();
        bytes.into_inner()
    }

    #[test]
    fn headers_are_checked_before_decode_and_attachments_are_deduplicated() {
        let _guard = TEST_LOCK.lock().unwrap();
        assert!(validate_image(&png(4097)).unwrap_err().contains("尺寸"));
        let dir = std::env::temp_dir().join(format!(
            "pet-media-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let bytes = png(1);
        let first = store_at(&dir, &bytes).unwrap();
        assert_eq!(store_at(&dir, &bytes).unwrap(), first);
        assert_eq!(fs::read_dir(&dir).unwrap().count(), 1);
        let quota = fs::File::create(dir.join("synthetic-quota")).unwrap();
        quota.set_len(ATTACHMENT_QUOTA).unwrap();
        assert!(store_at(&dir, &png(2)).unwrap_err().contains("配额"));
        assert!(dir.join(attachment_id(&first).unwrap()).exists());
        drop(quota);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn animations_have_a_total_frame_budget() {
        let _guard = TEST_LOCK.lock().unwrap();
        let mut bytes = Vec::new();
        {
            let mut encoder = image::codecs::gif::GifEncoder::new(&mut bytes);
            for _ in 0..33 {
                encoder
                    .encode_frame(image::Frame::new(image::RgbaImage::new(1, 1)))
                    .unwrap();
            }
        }
        assert!(validate_image(&bytes).unwrap_err().contains("32 帧"));
    }

    #[test]
    fn static_webp_is_decoded_and_decoder_concurrency_is_bounded() {
        let _guard = TEST_LOCK.lock().unwrap();
        let mut bytes = Cursor::new(Vec::new());
        image::DynamicImage::new_rgba8(2, 2)
            .write_to(&mut bytes, image::ImageFormat::WebP)
            .unwrap();
        assert_eq!(validate_image(bytes.get_ref()).unwrap(), "image/webp");
        let slots = DECODERS.try_acquire_many(2).unwrap();
        assert!(validate_image(bytes.get_ref())
            .unwrap_err()
            .contains("繁忙"));
        drop(slots);
        assert!(validate_image(bytes.get_ref()).is_ok());
    }
}
