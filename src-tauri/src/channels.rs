use crate::{ai, model::*};
use std::collections::HashSet;

/// 只有保存输入可规范化用户名称；读取配置时不改写原始数据。
pub(crate) fn normalize(channels: &AiChannels) -> Result<AiChannels, String> {
    let mut value = channels.clone();
    for channel in &mut value.channels {
        if channel.name.chars().any(char::is_control) {
            return Err("渠道名称不能包含控制字符".into());
        }
        if channel.id != OFFICIAL_AI_CHANNEL_ID {
            channel.name = channel.name.trim().to_owned();
        }
    }
    validate(&value)?;
    Ok(value)
}

pub(crate) fn validate(value: &AiChannels) -> Result<(), String> {
    let mut ids = HashSet::new();
    let mut names = HashSet::new();
    let mut official = false;
    for channel in &value.channels {
        if !ids.insert(channel.id.as_str()) {
            return Err("渠道身份不能重复".into());
        }
        let name = channel.name.trim();
        if name.is_empty()
            || name != channel.name
            || name.chars().count() > 60
            || name.chars().any(char::is_control)
        {
            return Err("渠道名称须为不超过 60 字符的非空文本，且不能包含控制字符".into());
        }
        let folded = name.to_lowercase();
        if !names.insert(folded.clone()) {
            return Err("渠道名称不能重复".into());
        }
        if channel.id == OFFICIAL_AI_CHANNEL_ID {
            official = true;
            // 官方身份与固定服务绑定，不能仅依赖前端隐藏地址或禁用选项。
            if channel.name != OFFICIAL_AI_CHANNEL_NAME
                || channel.config.endpoint != OFFICIAL_AI_WEBSITE
            {
                return Err("官方渠道的名称和地址不可修改".into());
            }
        } else {
            if !uuid::Uuid::parse_str(&channel.id).is_ok_and(|id| id.to_string() == channel.id) {
                return Err("用户渠道身份必须是规范 UUID".into());
            }
            if folded == OFFICIAL_AI_CHANNEL_NAME.to_lowercase() || folded == "upfanyi official ai"
            {
                return Err("渠道名称不能使用官方保留名称".into());
            }
        }
        ai::validate_config(&channel.config)?;
    }
    if !official {
        return Err("官方 AI 渠道缺失".into());
    }
    value.active_config()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_normalize_only_at_save_and_keep_identity_and_config() {
        let mut value = crate::test_support::channel_collection(&AiConfig::default());
        value.channels[1].name = "  服务 😀  ".into();
        assert!(validate(&value).is_err());
        let saved = normalize(&value).unwrap();
        assert_eq!(saved.channels[1].name, "服务 😀");
        assert_eq!(saved.active_id, value.active_id);
        assert_eq!(saved.channels[1].config, value.channels[1].config);
        assert_eq!(value.channels[1].name, "  服务 😀  ");
        saved.active_config().unwrap();
    }

    #[test]
    fn names_use_unicode_character_limits_and_case_insensitive_uniqueness() {
        let mut value = crate::test_support::channel_collection(&AiConfig::default());
        value.channels[1].name = "😀".repeat(60);
        assert!(normalize(&value).is_ok());
        value.channels[1].name.push('😀');
        assert!(normalize(&value).is_err());
        for name in [
            "",
            "  ",
            "a\nb",
            "\ta",
            "UPfanyi official AI",
            "up翻译官方 ai",
        ] {
            value.channels[1].name = name.into();
            assert!(normalize(&value).is_err(), "invalid fixture name accepted");
        }
        value.channels[1].name = "Relay".into();
        let mut other = value.channels[1].clone();
        other.id = uuid::Uuid::new_v4().to_string();
        other.name = "rELAY".into();
        value.channels.push(other);
        assert!(normalize(&value).is_err());
    }
}
