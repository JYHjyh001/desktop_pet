//! Translation connection routing. No companion, memory or story state.
use crate::{
    ai_chat,
    app_data::{AiSettings, PetDrawerConfig, TranslationSettings},
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranslationSettingsDraft {
    pub use_global_api: bool,
    pub provider: String,
    pub base_url: String,
    pub model: String,
    pub key_action: KeyAction,
    #[serde(default)]
    pub api_key: String,
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum KeyAction {
    Keep,
    Replace,
    Clear,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TranslationServiceSummary {
    pub use_global_api: bool,
    pub enabled: bool,
    pub provider: String,
    pub model: String,
    pub host: String,
    pub problem: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TranslationSettingsView {
    pub use_global_api: bool,
    pub provider: String,
    pub base_url: String,
    pub model: String,
    pub has_api_key: bool,
    pub effective: TranslationServiceSummary,
}

pub fn effective_settings(config: &PetDrawerConfig) -> AiSettings {
    let mut settings = if config.translation.use_global_api {
        config.ai.clone()
    } else {
        let connection = &config.translation;
        AiSettings {
            enabled: true,
            provider: connection.provider.clone(),
            base_url: connection.base_url.clone(),
            api_key: connection.api_key.clone(),
            model: connection.model.clone(),
            ..AiSettings::default()
        }
    };
    // Translate uses its own prompt. Never propagate unrelated profiles/persona.
    settings.system_prompt.clear();
    settings.profiles.clear();
    settings.active_profile_id.clear();
    settings.memory_enabled = false;
    settings.short_memory_summary_enabled = false;
    settings
}

pub fn validate(settings: &AiSettings) -> Result<(), String> {
    if !settings.enabled {
        return Err("共用的全局 AI 接口未启用，请启用全局接口或设置独立翻译 API。".into());
    }
    if !matches!(
        settings.provider.as_str(),
        "openai" | "deepseek" | "anthropic" | "gemini" | "ollama" | "custom"
    ) {
        return Err("不支持的翻译接口类型。".into());
    }
    if settings.model.trim().is_empty() {
        return Err("请填写翻译使用的模型名称。".into());
    }
    if settings.base_url.trim().is_empty() {
        return Err("请填写翻译使用的 Base URL。".into());
    }
    let url = reqwest::Url::parse(&settings.base_url).map_err(|_| "翻译 Base URL 格式无效。")?;
    if !url.username().is_empty() || url.password().is_some() || url.fragment().is_some() {
        return Err("翻译 Base URL 不得包含账号、密码或片段。".into());
    }
    ai_chat::validate_ai_transport(settings)?;
    if matches!(
        settings.provider.as_str(),
        "openai" | "deepseek" | "anthropic" | "gemini"
    ) && settings.api_key.trim().is_empty()
    {
        return Err("该翻译接口需要 API Key，请在翻译设置中检查密钥。".into());
    }
    Ok(())
}

pub fn summary(config: &PetDrawerConfig) -> TranslationServiceSummary {
    let settings = effective_settings(config);
    let problem = validate(&settings).err();
    TranslationServiceSummary {
        use_global_api: config.translation.use_global_api,
        enabled: problem.is_none(),
        provider: settings.provider,
        model: settings.model,
        host: reqwest::Url::parse(&settings.base_url)
            .ok()
            .and_then(|url| url.host_str().map(str::to_string))
            .unwrap_or_default(),
        problem,
    }
}

pub fn view(config: &PetDrawerConfig) -> TranslationSettingsView {
    let stored = &config.translation;
    TranslationSettingsView {
        use_global_api: stored.use_global_api,
        provider: stored.provider.clone(),
        base_url: stored.base_url.clone(),
        model: stored.model.clone(),
        has_api_key: !stored.api_key.is_empty(),
        effective: summary(config),
    }
}

pub fn apply_draft(
    config: &PetDrawerConfig,
    draft: TranslationSettingsDraft,
) -> Result<TranslationSettings, String> {
    let api_key = match draft.key_action {
        KeyAction::Keep => config.translation.api_key.clone(),
        KeyAction::Replace => {
            let key = draft.api_key.trim();
            if key.is_empty() {
                return Err("替换密钥时请输入新密钥，清除请使用明确清除选项。".into());
            }
            key.to_string()
        }
        KeyAction::Clear => String::new(),
    };
    if !matches!(
        draft.provider.as_str(),
        "openai" | "deepseek" | "anthropic" | "gemini" | "ollama" | "custom"
    ) {
        return Err("不支持的翻译接口类型。".into());
    }
    let settings = TranslationSettings {
        use_global_api: draft.use_global_api,
        provider: draft.provider,
        base_url: draft.base_url.trim().trim_end_matches('/').to_string(),
        api_key,
        model: draft.model.trim().to_string(),
    };
    // Permit explicitly clearing a key and incomplete configurations to be saved.
    // Reject unsafe URLs even while global mode is selected; usability is checked
    // separately before translating/testing.
    if !settings.base_url.is_empty() {
        let mut candidate = config.clone();
        candidate.translation = settings.clone();
        candidate.translation.use_global_api = false;
        let connection = effective_settings(&candidate);
        let url =
            reqwest::Url::parse(&connection.base_url).map_err(|_| "翻译 Base URL 格式无效。")?;
        if !url.username().is_empty() || url.password().is_some() || url.fragment().is_some() {
            return Err("翻译 Base URL 不得包含账号、密码或片段。".into());
        }
        ai_chat::validate_ai_transport(&connection)?;
    }
    Ok(settings)
}

pub fn draft_config(
    config: &PetDrawerConfig,
    draft: TranslationSettingsDraft,
) -> Result<PetDrawerConfig, String> {
    let mut candidate = config.clone();
    candidate.translation = apply_draft(config, draft)?;
    Ok(candidate)
}

// External providers may echo credentials/URLs in error bodies or test replies.
// Translation intentionally returns local, fixed diagnostics for network errors.
pub fn safe_request_error(summary: &TranslationServiceSummary) -> String {
    format!(
        "{}翻译请求失败，请检查连接、模型及服务权限；未切换到其他接口。",
        if summary.use_global_api {
            "全局接口"
        } else {
            "独立接口"
        }
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn draft(use_global_api: bool, key_action: KeyAction) -> TranslationSettingsDraft {
        TranslationSettingsDraft {
            use_global_api,
            provider: "custom".into(),
            base_url: "https://translation.example.test/v1".into(),
            model: "test-model".into(),
            key_action,
            api_key: String::new(),
        }
    }

    #[test]
    fn old_config_defaults_to_global_without_copying_its_key() {
        let mut json = serde_json::to_value(PetDrawerConfig::default()).unwrap();
        json.as_object_mut().unwrap().remove("translation");
        json["ai"]["apiKey"] = "global-placeholder".into();
        let config: PetDrawerConfig = serde_json::from_value(json).unwrap();
        assert!(config.translation.use_global_api);
        assert!(config.translation.api_key.is_empty());
        assert_eq!(effective_settings(&config).api_key, "global-placeholder");
        assert!(!summary(&config).enabled);
    }

    #[test]
    fn key_actions_and_mode_switch_preserve_global_and_saved_connection() {
        let mut config = PetDrawerConfig::default();
        config.ai.api_key = "global-placeholder".into();
        config.translation.api_key = "translation-placeholder".into();
        let original = serde_json::to_value(&config.ai).unwrap();
        let kept = draft_config(&config, draft(false, KeyAction::Keep)).unwrap();
        assert_eq!(kept.translation.api_key, "translation-placeholder");
        assert!(summary(&kept).enabled);
        assert_eq!(serde_json::to_value(&kept.ai).unwrap(), original);
        let shared = draft_config(&kept, draft(true, KeyAction::Keep)).unwrap();
        let independent = draft_config(&shared, draft(false, KeyAction::Keep)).unwrap();
        assert_eq!(independent.translation.api_key, kept.translation.api_key);
        let mut replace = draft(false, KeyAction::Replace);
        assert!(draft_config(&config, replace.clone()).is_err());
        replace.api_key = " replacement-placeholder ".into();
        let replaced = draft_config(&config, replace).unwrap();
        assert_eq!(replaced.translation.api_key, "replacement-placeholder");
        let cleared = draft_config(&replaced, draft(false, KeyAction::Clear)).unwrap();
        assert!(cleared.translation.api_key.is_empty());
        assert_eq!(config.translation.api_key, "translation-placeholder");
    }

    #[test]
    fn view_never_returns_secret_and_summary_only_returns_host() {
        let mut config =
            draft_config(&PetDrawerConfig::default(), draft(false, KeyAction::Keep)).unwrap();
        config.translation.api_key = "translation-placeholder".into();
        let value = serde_json::to_string(&view(&config)).unwrap();
        assert!(!value.contains("translation-placeholder"));
        assert!(view(&config).has_api_key);
        assert_eq!(summary(&config).host, "translation.example.test");
    }

    #[test]
    fn unsafe_transport_is_rejected_without_echoing_value() {
        for address in [
            "file:///private",
            "https://account:private@translation.example.test/v1",
            "https://translation.example.test/v1?api_key=private",
            "http://translation.example.test/v1",
        ] {
            let mut settings = draft(false, KeyAction::Replace);
            settings.api_key = "placeholder".into();
            settings.base_url = address.into();
            let error = draft_config(&PetDrawerConfig::default(), settings)
                .err()
                .unwrap();
            assert!(!error.contains(address));
            assert!(!error.contains("placeholder"));
        }
        let mut local = draft(false, KeyAction::Replace);
        local.api_key = "placeholder".into();
        local.base_url = "http://127.0.0.1:11434".into();
        assert!(draft_config(&PetDrawerConfig::default(), local).is_ok());
    }
}
