use std::{
    collections::BTreeMap,
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};
use tauri::AppHandle;

use crate::{
    ai_chat::{self, PetChatMessageDraft},
    ai_memory,
    app_data::{self, AiSettings, Companion},
};

const STORY_SAVES_FILE_NAME: &str = "story-saves.json";
const MAX_RECENT_STORY_MESSAGES: usize = 24;
const MAX_STORY_SUMMARY_CHARS: usize = 2400;
const STORY_TYPES: &[&str] = &[
    "日常恋爱",
    "校园生活",
    "奇幻冒险",
    "都市异能",
    "悬疑推理",
    "末日生存",
    "赛博朋克",
    "古风江湖",
    "魔法学院",
    "偶像养成",
    "治愈陪伴",
    "轻喜剧",
    "黑暗童话",
    "异世界旅行",
    "未来科幻",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorySave {
    pub id: String,
    pub title: String,
    pub story_type: String,
    pub mode: String,
    pub created_at: u64,
    pub updated_at: u64,
    pub user_role: String,
    pub current_chapter: u32,
    pub current_scene: String,
    #[serde(default)]
    pub current_location: String,
    #[serde(default)]
    pub current_time: String,
    #[serde(default)]
    pub characters: Vec<StoryCharacter>,
    #[serde(default)]
    pub active_character_ids: Vec<String>,
    #[serde(default)]
    pub relationship_values: BTreeMap<String, RelationshipState>,
    #[serde(default)]
    pub emotion_states: BTreeMap<String, String>,
    #[serde(default)]
    pub important_choices: Vec<StoryChoice>,
    #[serde(default)]
    pub unlocked_events: Vec<String>,
    #[serde(default)]
    pub hidden_flags: BTreeMap<String, bool>,
    #[serde(default)]
    pub inventory: Vec<String>,
    #[serde(default)]
    pub clues: Vec<String>,
    #[serde(default)]
    pub story_summary: String,
    #[serde(default)]
    pub recent_messages: Vec<StoryMessage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoryCharacter {
    pub id: String,
    pub name: String,
    pub source: String,
    #[serde(default)]
    pub avatar_id: Option<String>,
    #[serde(default)]
    pub role_in_story: String,
    #[serde(default)]
    pub personality: String,
    #[serde(default)]
    pub appearance: String,
    #[serde(default)]
    pub speaking_style: String,
    #[serde(default)]
    pub relationship_to_user: String,
    #[serde(default)]
    pub relationship_to_others: String,
    #[serde(default)]
    pub hidden_setting: String,
    #[serde(default = "default_true")]
    pub is_interactable: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RelationshipState {
    #[serde(default)]
    pub affection: i32,
    #[serde(default)]
    pub trust: i32,
    #[serde(default)]
    pub tension: i32,
    #[serde(default)]
    pub suspicion: i32,
    #[serde(default)]
    pub stage: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoryChoice {
    pub id: String,
    pub chapter: u32,
    pub scene: String,
    pub user_action: String,
    pub result_summary: String,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoryMessage {
    pub role: String,
    pub content: String,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoryCreateDraft {
    #[serde(default = "default_story_mode")]
    pub mode: String,
    #[serde(default)]
    pub story_type: String,
    #[serde(default)]
    pub tone: String,
    #[serde(default)]
    pub premise: String,
    #[serde(default)]
    pub companion_ids: Vec<String>,
    #[serde(default)]
    pub companion_role: String,
    #[serde(default)]
    pub temporary_characters: Vec<StoryCharacterDraft>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoryCharacterDraft {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub gender: String,
    #[serde(default)]
    pub age_stage: String,
    #[serde(default)]
    pub identity: String,
    #[serde(default)]
    pub appearance: String,
    #[serde(default)]
    pub personality: String,
    #[serde(default)]
    pub relationship_to_user: String,
    #[serde(default)]
    pub relationship_to_others: String,
    #[serde(default)]
    pub role_in_story: String,
    #[serde(default)]
    pub speaking_style: String,
    #[serde(default)]
    pub hidden_setting: String,
    #[serde(default = "default_true")]
    pub is_interactable: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StoryTurnReply {
    pub story: StorySave,
    pub reply: String,
}

pub fn list_story_saves(app: &AppHandle) -> Result<Vec<StorySave>, String> {
    let mut saves = read_story_saves(app)?;
    saves.sort_by(|left, right| right.updated_at.cmp(&left.updated_at));
    Ok(saves)
}

pub fn get_story_save(app: &AppHandle, story_id: &str) -> Result<StorySave, String> {
    read_story_saves(app)?
        .into_iter()
        .find(|story| story.id == story_id)
        .ok_or_else(|| "未找到故事存档".to_string())
}

pub fn create_story(app: &AppHandle, draft: StoryCreateDraft) -> Result<StoryTurnReply, String> {
    let mut config = app_data::read_config(app)?;
    let companions = selected_companions(app, &draft)?;
    apply_companion_model_override(&mut config.ai, companions.first());
    let story_type = resolved_story_type(&draft);
    let characters = story_characters_from_draft(&companions, &draft);
    let prompt = story_create_prompt(&draft, &story_type, &characters);
    let reply = send_story_request(config.ai, prompt, Vec::new())?;
    let now = now_seconds();
    let title =
        parse_story_field(&reply, &["故事标题"]).unwrap_or_else(|| "未命名故事".to_string());
    let current_scene = parse_story_field(&reply, &["当前场景", "初始场景"])
        .unwrap_or_else(|| "故事开局".to_string());
    let story = StorySave {
        id: format!("story_{}", now_millis()),
        title: clean_title(&title),
        story_type,
        mode: normalize_mode(&draft.mode),
        created_at: now,
        updated_at: now,
        user_role: parse_story_field(&reply, &["用户身份"])
            .unwrap_or_else(|| "故事参与者".to_string()),
        current_chapter: 1,
        current_scene,
        current_location: String::new(),
        current_time: String::new(),
        active_character_ids: characters
            .iter()
            .map(|character| character.id.clone())
            .collect(),
        characters,
        relationship_values: BTreeMap::new(),
        emotion_states: BTreeMap::new(),
        important_choices: Vec::new(),
        unlocked_events: Vec::new(),
        hidden_flags: BTreeMap::new(),
        inventory: Vec::new(),
        clues: Vec::new(),
        story_summary: truncate_chars(&reply, MAX_STORY_SUMMARY_CHARS),
        recent_messages: vec![StoryMessage {
            role: "assistant".to_string(),
            content: reply.clone(),
            timestamp: now,
        }],
    };

    let mut saves = read_story_saves(app)?;
    saves.push(story.clone());
    write_story_saves(app, &saves)?;
    Ok(StoryTurnReply { story, reply })
}

pub fn advance_story(
    app: &AppHandle,
    story_id: &str,
    user_input: &str,
) -> Result<StoryTurnReply, String> {
    let user_input = user_input.trim();
    if user_input.is_empty() {
        return Err("请输入你的行动或选择".to_string());
    }

    let mut saves = read_story_saves(app)?;
    let index = saves
        .iter()
        .position(|story| story.id == story_id)
        .ok_or_else(|| "未找到故事存档".to_string())?;
    let mut story = saves[index].clone();
    let mut config = app_data::read_config(app)?;
    let companion = story
        .characters
        .iter()
        .find(|character| character.source == "existing_avatar")
        .and_then(|character| character.avatar_id.as_deref())
        .and_then(|id| {
            ai_memory::list_companions(app)
                .ok()?
                .into_iter()
                .find(|item| item.id == id)
        });
    apply_companion_model_override(&mut config.ai, companion.as_ref());

    let prompt = story_turn_prompt(&story, user_input)?;
    let messages = story_messages_for_request(&story, user_input);
    let reply = send_story_request(config.ai, prompt, messages)?;
    update_story_after_turn(&mut story, user_input, &reply);
    saves[index] = story.clone();
    write_story_saves(app, &saves)?;
    Ok(StoryTurnReply { story, reply })
}

pub fn delete_story_save(app: &AppHandle, story_id: &str) -> Result<(), String> {
    let mut saves = read_story_saves(app)?;
    let before = saves.len();
    saves.retain(|story| story.id != story_id);
    if saves.len() == before {
        return Err("未找到要删除的故事存档".to_string());
    }
    write_story_saves(app, &saves)
}

pub fn rename_story_save(
    app: &AppHandle,
    story_id: &str,
    title: &str,
) -> Result<StorySave, String> {
    let title = title.trim();
    if title.is_empty() {
        return Err("故事标题不能为空".to_string());
    }
    let mut saves = read_story_saves(app)?;
    let story = saves
        .iter_mut()
        .find(|story| story.id == story_id)
        .ok_or_else(|| "未找到要重命名的故事存档".to_string())?;
    story.title = truncate_chars(title, 80);
    story.updated_at = now_seconds();
    let updated = story.clone();
    write_story_saves(app, &saves)?;
    Ok(updated)
}

fn read_story_saves(app: &AppHandle) -> Result<Vec<StorySave>, String> {
    let path = story_saves_file(app)?;
    if !path.exists() {
        return Ok(Vec::new());
    }
    let content = fs::read_to_string(&path).map_err(|err| format!("读取故事存档失败：{err}"))?;
    if content.trim().is_empty() {
        return Ok(Vec::new());
    }
    serde_json::from_str(&content).map_err(|err| format!("故事存档格式错误：{err}"))
}

fn write_story_saves(app: &AppHandle, saves: &[StorySave]) -> Result<(), String> {
    let path = story_saves_file(app)?;
    let content = serde_json::to_vec_pretty(saves).map_err(|err| err.to_string())?;
    app_data::atomic_write_file(&path, &content).map_err(|err| format!("写入故事存档失败：{err}"))
}

fn story_saves_file(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app_data::memory_dir(app)?;
    fs::create_dir_all(&dir).map_err(|err| format!("创建故事存档目录失败：{err}"))?;
    Ok(dir.join(STORY_SAVES_FILE_NAME))
}

fn selected_companions(
    app: &AppHandle,
    draft: &StoryCreateDraft,
) -> Result<Vec<Companion>, String> {
    let companions = ai_memory::list_companions(app)?;
    let selected = draft
        .companion_ids
        .iter()
        .filter_map(|id| {
            companions
                .iter()
                .find(|companion| companion.id == *id)
                .cloned()
        })
        .collect::<Vec<_>>();
    Ok(selected)
}

fn story_characters_from_draft(
    companions: &[Companion],
    draft: &StoryCreateDraft,
) -> Vec<StoryCharacter> {
    let companion_role = if draft.companion_role.trim().is_empty() {
        "重要角色 / 搭档".to_string()
    } else {
        draft.companion_role.trim().to_string()
    };
    let mut characters = companions
        .iter()
        .map(|companion| StoryCharacter {
            id: companion.id.clone(),
            name: companion.name.clone(),
            source: "existing_avatar".to_string(),
            avatar_id: Some(companion.id.clone()),
            role_in_story: companion_role.clone(),
            personality: join_non_empty(&[
                companion.personality.as_str(),
                companion.persona_prompt.as_str(),
            ]),
            appearance: companion.avatar.clone().unwrap_or_default(),
            speaking_style: companion.system_prompt.clone(),
            relationship_to_user: companion.scenario.clone(),
            relationship_to_others: String::new(),
            hidden_setting: companion.creator_notes.clone(),
            is_interactable: true,
        })
        .collect::<Vec<_>>();

    for (index, character) in draft.temporary_characters.iter().enumerate() {
        if let Some(character) = temporary_story_character(character, companions.len() + index + 1)
        {
            characters.push(character);
        }
    }

    characters
}

fn temporary_story_character(
    draft: &StoryCharacterDraft,
    fallback_index: usize,
) -> Option<StoryCharacter> {
    let name = draft.name.trim();
    if name.is_empty()
        && draft.identity.trim().is_empty()
        && draft.personality.trim().is_empty()
        && draft.role_in_story.trim().is_empty()
    {
        return None;
    }

    Some(StoryCharacter {
        id: format!("temporary_{fallback_index}"),
        name: if name.is_empty() {
            format!("临时角色{fallback_index}")
        } else {
            name.to_string()
        },
        source: "temporary".to_string(),
        avatar_id: None,
        role_in_story: join_non_empty(&[
            draft.role_in_story.as_str(),
            draft.identity.as_str(),
            draft.gender.as_str(),
            draft.age_stage.as_str(),
        ]),
        personality: draft.personality.trim().to_string(),
        appearance: draft.appearance.trim().to_string(),
        speaking_style: draft.speaking_style.trim().to_string(),
        relationship_to_user: draft.relationship_to_user.trim().to_string(),
        relationship_to_others: draft.relationship_to_others.trim().to_string(),
        hidden_setting: draft.hidden_setting.trim().to_string(),
        is_interactable: draft.is_interactable,
    })
}

fn send_story_request(
    mut settings: AiSettings,
    system_prompt: String,
    messages: Vec<PetChatMessageDraft>,
) -> Result<String, String> {
    settings.max_tokens = settings.max_tokens.clamp(600, 2400);
    ai_chat::send_story_text_request(settings, system_prompt, messages)
}

fn apply_companion_model_override(settings: &mut AiSettings, companion: Option<&Companion>) {
    if let Some(companion) = companion {
        if !companion.model.trim().is_empty() {
            settings.model = companion.model.clone();
        }
    }
}

fn story_create_prompt(
    draft: &StoryCreateDraft,
    story_type: &str,
    characters: &[StoryCharacter],
) -> String {
    let mode_label = if normalize_mode(&draft.mode) == "custom" {
        "自定义设定模式"
    } else {
        "随机模式"
    };
    format!(
        "{}\n\n[故事创建任务]\n创建方式：{}\n故事类型：{}\n故事氛围：{}\n用户设定：{}\n\n当前角色设定：\n{}\n\n请生成故事开局。随机模式必须给出明确冲突；自定义模式必须尊重用户设定。\n\n输出格式：\n故事标题：\n故事类型：\n故事基调：\n世界观背景：\n当前场景：\n用户身份：\nAI 角色身份：\n其他主要角色：\n当前剧情冲突：\n当前可选行动：\n1.\n2.\n3.\n4. 自由输入你的行动",
        story_system_prompt(),
        mode_label,
        story_type,
        empty_as_random(&draft.tone),
        empty_as_random(&draft.premise),
        character_profiles_text(characters)
    )
}

fn story_turn_prompt(story: &StorySave, user_input: &str) -> Result<String, String> {
    let state = serde_json::to_string_pretty(story).map_err(|err| err.to_string())?;
    Ok(format!(
        "{}\n\n当前故事状态：\n{}\n\n故事摘要：\n{}\n\n当前角色设定：\n{}\n\n用户本轮输入：\n{}\n\n请生成下一轮故事内容。\n\n输出格式：\n【剧情描写】\n\n【角色对话】\n\n【状态变化】\n\n【可选行动】\n1.\n2.\n3.\n4. 自由输入你的行动",
        story_system_prompt(),
        state,
        story.story_summary,
        character_profiles_text(&story.characters),
        user_input
    ))
}

fn story_system_prompt() -> &'static str {
    r#"你是一个互动故事模式引擎，需要根据用户选择、角色设定、故事状态和历史摘要，持续生成可互动的剧情。

你需要同时扮演：
1. 故事旁白
2. 当前场景中的角色
3. 剧情管理器
4. 状态更新器

你必须遵守以下规则：
- 根据当前故事设定继续剧情。
- 保持角色性格一致。
- 保持故事世界观一致。
- 不要替用户做关键决定。
- 用户可以通过对话、行动、选择影响剧情。
- 每次回复都要给出自然的后续互动空间。
- 如果用户自由输入行动，需要合理解释结果。
- 重要选择需要记录到故事变量。
- 关系变化需要循序渐进。
- 不要暴露隐藏变量，除非剧情中自然揭示。
- 不要一次性跳过太多剧情，也不要让故事过快完结。
- 生成选项时必须允许用户自由输入。"#
}

fn story_messages_for_request(story: &StorySave, user_input: &str) -> Vec<PetChatMessageDraft> {
    let mut messages = story
        .recent_messages
        .iter()
        .rev()
        .take(12)
        .collect::<Vec<_>>();
    messages.reverse();
    let mut request = messages
        .into_iter()
        .map(|message| PetChatMessageDraft {
            role: message.role.clone(),
            content: message.content.clone(),
            created_at: message.timestamp.to_string(),
            time_context: String::new(),
        })
        .collect::<Vec<_>>();
    request.push(PetChatMessageDraft {
        role: "user".to_string(),
        content: user_input.to_string(),
        created_at: now_seconds().to_string(),
        time_context: String::new(),
    });
    request
}

fn update_story_after_turn(story: &mut StorySave, user_input: &str, reply: &str) {
    let now = now_seconds();
    story.updated_at = now;
    story.current_scene = parse_story_section_first_line(reply, "剧情描写")
        .or_else(|| parse_story_field(reply, &["当前场景", "初始场景"]))
        .unwrap_or_else(|| story.current_scene.clone());
    story.recent_messages.push(StoryMessage {
        role: "user".to_string(),
        content: user_input.to_string(),
        timestamp: now,
    });
    story.recent_messages.push(StoryMessage {
        role: "assistant".to_string(),
        content: reply.to_string(),
        timestamp: now,
    });
    if story.recent_messages.len() > MAX_RECENT_STORY_MESSAGES {
        let drain_count = story.recent_messages.len() - MAX_RECENT_STORY_MESSAGES;
        story.recent_messages.drain(0..drain_count);
    }
    story.important_choices.push(StoryChoice {
        id: format!("choice_{}", now_millis()),
        chapter: story.current_chapter,
        scene: story.current_scene.clone(),
        user_action: truncate_chars(user_input, 400),
        result_summary: truncate_chars(reply, 500),
        timestamp: now,
    });
    if story.important_choices.len() > 80 {
        let drain_count = story.important_choices.len() - 80;
        story.important_choices.drain(0..drain_count);
    }
    story.story_summary = append_summary_line(&story.story_summary, user_input, reply);
}

fn append_summary_line(summary: &str, user_input: &str, reply: &str) -> String {
    let line = format!(
        "用户行动：{}；结果：{}",
        truncate_chars(user_input, 120),
        truncate_chars(reply, 220)
    );
    let next = if summary.trim().is_empty() {
        line
    } else {
        format!("{}\n{}", summary.trim(), line)
    };
    let chars = next.chars().collect::<Vec<_>>();
    if chars.len() <= MAX_STORY_SUMMARY_CHARS {
        next
    } else {
        chars[chars.len() - MAX_STORY_SUMMARY_CHARS..]
            .iter()
            .collect()
    }
}

fn resolved_story_type(draft: &StoryCreateDraft) -> String {
    let value = draft.story_type.trim();
    if !value.is_empty() && value != "随机" {
        return truncate_chars(value, 40);
    }
    STORY_TYPES[(now_millis() as usize) % STORY_TYPES.len()].to_string()
}

fn normalize_mode(value: &str) -> String {
    if value.trim().eq_ignore_ascii_case("custom") {
        "custom".to_string()
    } else {
        "random".to_string()
    }
}

fn default_story_mode() -> String {
    "random".to_string()
}

fn default_true() -> bool {
    true
}

fn character_profiles_text(characters: &[StoryCharacter]) -> String {
    if characters.is_empty() {
        return "不指定，由故事模式临时生成角色。".to_string();
    }
    characters
        .iter()
        .map(|character| {
            format!(
                "角色名称：{}\n来源：{}\n剧情定位：{}\n性格特点：{}\n与用户的关系：{}\n说话风格：{}\n隐藏设定：{}",
                character.name,
                character.source,
                character.role_in_story,
                character.personality,
                character.relationship_to_user,
                character.speaking_style,
                character.hidden_setting
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn join_non_empty(values: &[&str]) -> String {
    values
        .iter()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

fn empty_as_random(value: &str) -> String {
    if value.trim().is_empty() {
        "随机生成".to_string()
    } else {
        value.trim().to_string()
    }
}

fn parse_story_field(text: &str, labels: &[&str]) -> Option<String> {
    for line in text.lines() {
        let line = line.trim();
        for label in labels {
            for delimiter in ["：", ":"] {
                let prefix = format!("{label}{delimiter}");
                if let Some(value) = line.strip_prefix(&prefix) {
                    let value = value.trim();
                    if !value.is_empty() {
                        return Some(value.to_string());
                    }
                }
            }
        }
    }
    None
}

fn parse_story_section_first_line(text: &str, label: &str) -> Option<String> {
    let heading = format!("【{label}】");
    let mut in_section = false;
    for line in text.lines() {
        let line = line.trim();
        if line == heading {
            in_section = true;
            continue;
        }
        if in_section {
            if line.starts_with('【') {
                return None;
            }
            if !line.is_empty() {
                return Some(truncate_chars(line, 160));
            }
        }
    }
    None
}

fn clean_title(title: &str) -> String {
    let title = title.trim().trim_start_matches('《').trim_end_matches('》');
    if title.is_empty() {
        "未命名故事".to_string()
    } else {
        truncate_chars(title, 80)
    }
}

fn truncate_chars(value: &str, limit: usize) -> String {
    value.chars().take(limit).collect()
}

fn now_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn now_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn random_story_type_falls_back_to_known_list() {
        let draft = StoryCreateDraft {
            mode: "random".to_string(),
            story_type: String::new(),
            tone: String::new(),
            premise: String::new(),
            companion_ids: Vec::new(),
            companion_role: String::new(),
            temporary_characters: Vec::new(),
        };

        assert!(STORY_TYPES.contains(&resolved_story_type(&draft).as_str()));
    }

    #[test]
    fn story_summary_is_kept_bounded() {
        let summary = "旧剧情。".repeat(1000);
        let next = append_summary_line(&summary, "选择 1", "剧情继续。");

        assert!(next.chars().count() <= MAX_STORY_SUMMARY_CHARS);
        assert!(next.contains("选择 1"));
    }
}
