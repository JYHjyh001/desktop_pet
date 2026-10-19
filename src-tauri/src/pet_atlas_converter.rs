use crate::app_data::PetAnimationSet;
use base64::{engine::general_purpose::STANDARD as BASE64_STANDARD, Engine as _};
use image::{
    codecs::gif::{GifEncoder, Repeat},
    Delay, Frame, GenericImageView, RgbaImage,
};
use serde::Serialize;
use serde_json::{json, Value};
use std::{
    fs::{self, File},
    io::BufWriter,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

const CELL_WIDTH: u32 = 192;
const CELL_HEIGHT: u32 = 208;
const ATLAS_WIDTH: u32 = CELL_WIDTH * 8;
const V1_HEIGHT: u32 = CELL_HEIGHT * 9;
const V2_HEIGHT: u32 = CELL_HEIGHT * 11;

const STANDARD_ROWS: [(&str, u32, u32, u32); 9] = [
    ("idle", 0, 6, 12),
    ("dragging-right", 1, 8, 8),
    ("dragging-left", 2, 8, 8),
    ("waving", 3, 4, 12),
    ("jumping", 4, 5, 10),
    ("failed", 5, 8, 12),
    ("waiting", 6, 6, 14),
    ("running", 7, 6, 10),
    ("review", 8, 6, 12),
];

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PetAtlasConversionDraft {
    pub name: String,
    pub source_format: String,
    pub sprite_version_number: u8,
    pub atlas_width: u32,
    pub atlas_height: u32,
    pub structural_check_passed: bool,
    pub visual_qa_status: String,
    pub checks: Vec<String>,
    pub warnings: Vec<String>,
    pub animations: PetAnimationSet,
    pub preview_data_url: String,
    pub temporary_directory: String,
}

pub fn convert(source: &str) -> Result<PetAtlasConversionDraft, String> {
    let source = Path::new(source);
    if !source.is_dir() {
        return Err("请选择有效的 hatch-pet 生成结果目录".to_string());
    }

    let atlas_path = find_atlas(source)?;
    if fs::metadata(&atlas_path)
        .map_err(|_| "无法读取图集信息")?
        .len()
        > 32 * 1024 * 1024
    {
        return Err("宠物图集文件超过 32 MiB 限制".into());
    }
    let dimensions = image::ImageReader::open(&atlas_path)
        .map_err(|_| "无法读取图集")?
        .with_guessed_format()
        .map_err(|_| "图集格式无效")?
        .into_dimensions()
        .map_err(|_| "图集头部无效")?;
    if dimensions.0 != ATLAS_WIDTH || !matches!(dimensions.1, V1_HEIGHT | V2_HEIGHT) {
        return Err("图集头部尺寸不符合标准，未进行全量解码".into());
    }
    let mut reader = image::ImageReader::open(&atlas_path)
        .map_err(|_| "无法读取图集")?
        .with_guessed_format()
        .map_err(|_| "图集格式无效")?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(ATLAS_WIDTH);
    limits.max_image_height = Some(V2_HEIGHT);
    limits.max_alloc = Some(64 * 1024 * 1024);
    reader.limits(limits);
    let image = reader
        .decode()
        .map_err(|_| "图集解码失败或超过资源限制")?
        .into_rgba8();
    let (width, height) = image.dimensions();
    if width != ATLAS_WIDTH || !matches!(height, V1_HEIGHT | V2_HEIGHT) {
        return Err(format!(
            "图集尺寸为 {width} x {height}，需要 1536 x 2288（v2）或 1536 x 1872（旧版兼容）"
        ));
    }

    validate_atlas_structure(&image, height == V2_HEIGHT)?;
    let sprite_version_number = if height == V2_HEIGHT { 2 } else { 1 };
    let mut checks = vec![
        format!("图集尺寸 {width} x {height} 正确"),
        "9 个标准动作行均包含非透明像素".to_string(),
    ];
    let mut warnings = Vec::new();
    if sprite_version_number == 1 {
        warnings.push("这是旧版 8 x 9 图集，不包含 v2 的 16 个注视方向".to_string());
    } else {
        checks.push("已识别 v2 的 2 个注视方向行".to_string());
    }

    let (visual_qa_status, qa_checks, qa_warnings) = inspect_qa(source, sprite_version_number);
    checks.extend(qa_checks);
    warnings.extend(qa_warnings);

    let output_dir = conversion_root().join(format!("conversion_{}", now_millis()));
    fs::create_dir_all(&output_dir).map_err(|err| format!("创建转换临时目录失败：{err}"))?;

    let result = (|| {
        for (name, row, frame_count, delay) in STANDARD_ROWS {
            encode_row_gif(
                &image,
                &output_dir.join(format!("{name}.gif")),
                row,
                frame_count,
                delay,
            )?;
        }

        let animations = mapped_animations(&output_dir);
        let name = detect_name(source).unwrap_or_else(|| "AI 转换宠物".to_string());
        let manifest = json!({
            "id": format!("converted_{}", now_millis()),
            "name": name,
            "animations": relative_animation_manifest(),
            "createdAt": now_millis().to_string()
        });
        fs::write(
            output_dir.join("pet.json"),
            serde_json::to_vec_pretty(&manifest).map_err(|err| err.to_string())?,
        )
        .map_err(|err| format!("写入转换清单失败：{err}"))?;

        let preview_data_url = format!(
            "data:image/gif;base64,{}",
            BASE64_STANDARD
                .encode(fs::read(output_dir.join("idle.gif")).map_err(|err| err.to_string())?)
        );
        Ok(PetAtlasConversionDraft {
            name,
            source_format: atlas_path
                .extension()
                .and_then(|value| value.to_str())
                .unwrap_or("unknown")
                .to_ascii_lowercase(),
            sprite_version_number,
            atlas_width: width,
            atlas_height: height,
            structural_check_passed: true,
            visual_qa_status,
            checks,
            warnings,
            animations,
            preview_data_url,
            temporary_directory: output_dir.to_string_lossy().into_owned(),
        })
    })();

    if result.is_err() {
        let _ = fs::remove_dir_all(&output_dir);
    }
    result
}

pub fn cleanup(path: &str) -> Result<(), String> {
    let root = conversion_root();
    let target = PathBuf::from(path);
    let is_conversion_child = target.parent() == Some(root.as_path())
        && target
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with("conversion_"));
    if !is_conversion_child {
        return Err("拒绝清理非转换器临时目录".to_string());
    }
    if target.exists() {
        fs::remove_dir_all(&target).map_err(|err| format!("清理转换临时文件失败：{err}"))?;
    }
    Ok(())
}

fn conversion_root() -> PathBuf {
    std::env::temp_dir()
        .join("PetDrawer")
        .join("pet-atlas-conversions")
}

fn now_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

fn find_atlas(source: &Path) -> Result<PathBuf, String> {
    let candidates = [
        "final/spritesheet-extended.webp",
        "final/spritesheet-extended.png",
        "spritesheet.webp",
        "spritesheet.png",
        "final/spritesheet.webp",
        "final/spritesheet.png",
    ];
    candidates
        .iter()
        .map(|relative| source.join(relative))
        .find(|path| path.is_file())
        .ok_or_else(|| "目录中未找到支持的 spritesheet.webp 或 spritesheet.png".to_string())
}

fn validate_atlas_structure(image: &RgbaImage, require_v2: bool) -> Result<(), String> {
    for (name, row, frame_count, _) in STANDARD_ROWS {
        for column in 0..frame_count {
            let has_visible_pixel = image
                .view(
                    column * CELL_WIDTH,
                    row * CELL_HEIGHT,
                    CELL_WIDTH,
                    CELL_HEIGHT,
                )
                .pixels()
                .any(|(_, _, pixel)| pixel.0[3] > 0);
            if !has_visible_pixel {
                return Err(format!("动作 {name} 的第 {} 帧为空", column + 1));
            }
        }
        for column in frame_count..8 {
            let has_visible_pixel = image
                .view(
                    column * CELL_WIDTH,
                    row * CELL_HEIGHT,
                    CELL_WIDTH,
                    CELL_HEIGHT,
                )
                .pixels()
                .any(|(_, _, pixel)| pixel.0[3] > 0);
            if has_visible_pixel {
                return Err(format!("动作 {name} 的未使用单元格 {} 不透明", column + 1));
            }
        }
    }
    if require_v2 {
        for row in 9..=10 {
            for column in 0..8 {
                let has_visible_pixel = image
                    .view(
                        column * CELL_WIDTH,
                        row * CELL_HEIGHT,
                        CELL_WIDTH,
                        CELL_HEIGHT,
                    )
                    .pixels()
                    .any(|(_, _, pixel)| pixel.0[3] > 0);
                if !has_visible_pixel {
                    return Err(format!(
                        "v2 注视方向第 {} 行第 {} 帧为空",
                        row + 1,
                        column + 1
                    ));
                }
            }
        }
    }
    Ok(())
}

fn encode_row_gif(
    atlas: &RgbaImage,
    output: &Path,
    row: u32,
    frame_count: u32,
    delay_centiseconds: u32,
) -> Result<(), String> {
    let file = File::create(output).map_err(|err| format!("创建动画失败：{err}"))?;
    let mut encoder = GifEncoder::new_with_speed(BufWriter::new(file), 10);
    encoder
        .set_repeat(Repeat::Infinite)
        .map_err(|err| format!("设置 GIF 循环失败：{err}"))?;
    let frames = (0..frame_count).map(|column| {
        let cell = image::imageops::crop_imm(
            atlas,
            column * CELL_WIDTH,
            row * CELL_HEIGHT,
            CELL_WIDTH,
            CELL_HEIGHT,
        )
        .to_image();
        Frame::from_parts(
            cell,
            0,
            0,
            Delay::from_numer_denom_ms(delay_centiseconds * 10, 1),
        )
    });
    encoder
        .encode_frames(frames)
        .map_err(|err| format!("编码 GIF 动画失败：{err}"))
}

fn mapped_animations(output_dir: &Path) -> PetAnimationSet {
    let path = |name: &str| Some(output_dir.join(name).to_string_lossy().into_owned());
    PetAnimationSet {
        idle: path("idle.gif"),
        hover: path("waving.gif"),
        click: path("jumping.gif"),
        dragging: path("running.gif"),
        dragging_left: path("dragging-left.gif"),
        dragging_right: path("dragging-right.gif"),
        waving: path("waving.gif"),
        jumping: path("jumping.gif"),
        waiting: path("waiting.gif"),
        running: path("running.gif"),
        review: path("review.gif"),
        failed: path("failed.gif"),
    }
}

fn relative_animation_manifest() -> Value {
    json!({
        "idle": "idle.gif", "hover": "waving.gif", "click": "jumping.gif",
        "dragging": "running.gif", "draggingLeft": "dragging-left.gif",
        "draggingRight": "dragging-right.gif", "waving": "waving.gif",
        "jumping": "jumping.gif", "waiting": "waiting.gif", "running": "running.gif",
        "review": "review.gif", "failed": "failed.gif"
    })
}

fn detect_name(source: &Path) -> Option<String> {
    for file in ["pet_request.json", "pet.json"] {
        let Ok(bytes) = fs::read(source.join(file)) else {
            continue;
        };
        let Ok(value) = serde_json::from_slice::<Value>(&bytes) else {
            continue;
        };
        for key in ["display_name", "displayName", "name", "pet_name"] {
            if let Some(name) = value
                .get(key)
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|name| !name.is_empty())
            {
                return Some(name.to_string());
            }
        }
    }
    source
        .file_name()
        .and_then(|name| name.to_str())
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(ToString::to_string)
}

fn inspect_qa(source: &Path, version: u8) -> (String, Vec<String>, Vec<String>) {
    if version != 2 {
        return ("legacy".to_string(), Vec::new(), Vec::new());
    }
    let required = [
        ("final/validation-extended.json", "v2 图集校验"),
        ("qa/chroma-despill-extended.json", "透明边缘校验"),
        ("qa/direction-blind-validation.json", "注视方向盲测"),
        ("qa/direction-semantics.json", "注视方向语义检查"),
        ("qa/review.json", "标准动作帧检查"),
    ];
    let mut checks = Vec::new();
    let mut warnings = Vec::new();
    let mut complete = true;
    for (relative, label) in required {
        let path = source.join(relative);
        match fs::read(&path)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
        {
            Some(value) if qa_value_failed(&value) => {
                complete = false;
                warnings.push(format!("{label}报告存在失败项，请先回到生成流程修复"));
            }
            Some(_) => checks.push(format!("已读取{label}证据")),
            None => {
                complete = false;
                warnings.push(format!("缺少{label}文件，视觉质量未完整验证"));
            }
        }
    }
    (
        if complete { "verified" } else { "unverified" }.to_string(),
        checks,
        warnings,
    )
}

fn qa_value_failed(value: &Value) -> bool {
    value.get("ok").and_then(Value::as_bool) == Some(false)
        || value
            .get("errors")
            .and_then(Value::as_array)
            .is_some_and(|errors| !errors.is_empty())
        || value.to_string().contains("\"verdict\":\"fail\"")
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;

    #[test]
    fn validates_v2_standard_cells() {
        let mut atlas = RgbaImage::new(ATLAS_WIDTH, V2_HEIGHT);
        for (_, row, count, _) in STANDARD_ROWS {
            for column in 0..count {
                atlas.put_pixel(column * CELL_WIDTH, row * CELL_HEIGHT, Rgba([1, 2, 3, 255]));
            }
        }
        for row in 9..=10 {
            for column in 0..8 {
                atlas.put_pixel(column * CELL_WIDTH, row * CELL_HEIGHT, Rgba([1, 2, 3, 255]));
            }
        }
        assert!(validate_atlas_structure(&atlas, true).is_ok());
    }

    #[test]
    fn rejects_blank_required_frame() {
        let atlas = RgbaImage::new(ATLAS_WIDTH, V1_HEIGHT);
        assert!(validate_atlas_structure(&atlas, false)
            .unwrap_err()
            .contains("idle"));
    }

    #[test]
    fn rejects_cleanup_outside_converter_root() {
        assert!(cleanup("C:\\Windows").is_err());
    }

    #[test]
    fn encodes_a_transparent_animation_gif() {
        let mut atlas = RgbaImage::new(ATLAS_WIDTH, CELL_HEIGHT);
        for column in 0..6 {
            atlas.put_pixel(column * CELL_WIDTH, 0, Rgba([10, 20, 30, 255]));
        }
        let output =
            std::env::temp_dir().join(format!("pet-drawer-atlas-test-{}.gif", now_millis()));
        encode_row_gif(&atlas, &output, 0, 6, 12).unwrap();
        let bytes = fs::read(&output).unwrap();
        assert!(bytes.starts_with(b"GIF89a") || bytes.starts_with(b"GIF87a"));
        assert!(bytes.len() > 100);
        fs::remove_file(output).unwrap();
    }
}
