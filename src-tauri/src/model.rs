use serde::{Deserialize, Serialize};

pub const OFFICIAL_AI_WEBSITE: &str = "https://uptoken.top";
pub const OFFICIAL_AI_CHANNEL_ID: &str = "upfanyi";
pub const OFFICIAL_AI_CHANNEL_NAME: &str = "UP翻译官方 AI";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Settings {
    pub locale: String,
    pub minimize_to_tray: bool,
    pub always_on_top: bool,
    // data-v3 的保留字段只维持原有设置格式，不控制桌面行为。
    pub clipboard_monitor: bool,
    pub clipboard_min_length: usize,
    pub hotkey_show_window: String,
    pub hotkey_clipboard_translate: String,
    pub default_source_lang: String,
    pub default_target_lang: String,
    pub auto_translate: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            locale: "zh_CN".into(),
            minimize_to_tray: true,
            always_on_top: false,
            clipboard_monitor: false,
            clipboard_min_length: 1,
            hotkey_show_window: "Ctrl+Shift+T".into(),
            hotkey_clipboard_translate: "Ctrl+Shift+C".into(),
            default_source_lang: "auto".into(),
            default_target_lang: "en".into(),
            auto_translate: false,
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AiProtocol {
    Openai,
    // 用一个明确模式表达接口组合，保持当前配置的五字段结构。
    #[serde(rename = "openai-responses")]
    OpenaiResponses,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AiConfig {
    pub protocol: AiProtocol,
    pub endpoint: String,
    pub api_key: String,
    pub model: String,
    pub system_prompt: String,
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            protocol: AiProtocol::Openai,
            // 仅新库初始化使用此预设，读取已保存配置时不会回填或覆盖。
            endpoint: OFFICIAL_AI_WEBSITE.into(),
            api_key: String::new(),
            // 模型可用性由服务端和账户决定，首次使用必须明确填写。
            model: String::new(),
            system_prompt: "You are a professional translator. Translate the following text from {source_lang} to {target_lang}. Only output the translation result, no explanations.".into(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AiChannel {
    pub id: String,
    pub name: String,
    pub config: AiConfig,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AiChannels {
    pub active_id: String,
    pub channels: Vec<AiChannel>,
}

impl Default for AiChannels {
    fn default() -> Self {
        Self {
            active_id: OFFICIAL_AI_CHANNEL_ID.into(),
            channels: vec![AiChannel {
                id: OFFICIAL_AI_CHANNEL_ID.into(),
                name: OFFICIAL_AI_CHANNEL_NAME.into(),
                config: AiConfig::default(),
            }],
        }
    }
}

impl AiChannels {
    /// 活动配置只是集合快照的投影，不单独存储，也不回退到任意默认渠道。
    pub fn active_config(&self) -> Result<&AiConfig, String> {
        self.channels
            .iter()
            .find(|channel| channel.id == self.active_id)
            .map(|channel| &channel.config)
            .ok_or_else(|| "活动 AI 渠道不存在".into())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TranslationRequest {
    pub text: String,
    pub source_lang: String,
    pub target_lang: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Translation {
    pub source_text: String,
    pub translated_text: String,
    pub source_lang: String,
    pub target_lang: String,
    pub model: String,
    pub create_time: String,
    pub warning: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryRecord {
    pub id: i64,
    pub source_text: String,
    pub translated_text: String,
    pub source_lang: String,
    pub target_lang: String,
    pub model: String,
    pub create_time: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FavoriteWord {
    pub id: i64,
    pub word: String,
    pub translation: String,
    pub note: Option<String>,
    pub create_time: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Page<T> {
    pub items: Vec<T>,
    pub total: usize,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Bootstrap {
    pub settings: Settings,
    pub ai_channels: AiChannels,
    pub ai_config: AiConfig,
    pub data_dir: String,
    pub warnings: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn translation_request_rejects_removed_engine_selection() {
        let mut request = json!({"text":"Hello", "sourceLang":"en", "targetLang":"zh"});
        assert!(serde_json::from_value::<TranslationRequest>(request.clone()).is_ok());
        request["apiName"] = json!("removed-provider");
        assert!(serde_json::from_value::<TranslationRequest>(request).is_err());
        let mut settings = serde_json::to_value(Settings::default()).unwrap();
        settings["defaultApi"] = json!("removed-provider");
        assert!(serde_json::from_value::<Settings>(settings).is_err());
    }

    #[test]
    fn ai_configuration_has_only_explicit_typed_fields() {
        let value = serde_json::to_value(AiConfig::default()).unwrap();
        assert_eq!(value.as_object().unwrap().len(), 5);
        assert_eq!(value["protocol"], "openai");
        assert_eq!(value["model"], "");
        for protocol in ["openai", "openai-responses"] {
            let mut valid = value.clone();
            valid["protocol"] = json!(protocol);
            assert!(serde_json::from_value::<AiConfig>(valid).is_ok());
        }
        let mut unknown = value.clone();
        unknown["protocol"] = json!("traditional");
        assert!(serde_json::from_value::<AiConfig>(unknown).is_err());
        let mut extra = value.clone();
        extra["priority"] = json!(1);
        assert!(serde_json::from_value::<AiConfig>(extra).is_err());
        for field in ["protocol", "endpoint", "apiKey", "model", "systemPrompt"] {
            let mut missing = value.clone();
            missing.as_object_mut().unwrap().remove(field);
            assert!(serde_json::from_value::<AiConfig>(missing).is_err());
        }
    }
}
