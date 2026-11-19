//! 数据层与应用流程共享的纯校验，不访问文件、网络或桌面状态。

use crate::model::Settings;
use std::str::FromStr;
use tauri_plugin_global_shortcut::Shortcut;

pub(crate) fn validate_settings(settings: &Settings) -> Result<(), String> {
    if !["zh_CN", "en_US"].contains(&settings.locale.as_str()) {
        return Err("不支持的界面语言".into());
    }
    if !(1..=10000).contains(&settings.clipboard_min_length) {
        return Err("本版设置中的保留数值无效".into());
    }
    validate_languages(&settings.default_source_lang, &settings.default_target_lang)?;
    shortcuts(settings)?;
    Ok(())
}

pub(crate) fn validate_languages(source: &str, target: &str) -> Result<(), String> {
    for lang in [source, target] {
        if lang.is_empty()
            || lang.len() > 32
            || !lang
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        {
            return Err("语言代码无效".into());
        }
    }
    if target.eq_ignore_ascii_case("auto") {
        return Err("目标语言不能为自动检测".into());
    }
    Ok(())
}

pub(crate) fn shortcuts(settings: &Settings) -> Result<[Shortcut; 2], String> {
    fn parse(text: &str) -> Result<Shortcut, ()> {
        // 只接受设置页展示的加号分隔格式，不转换空格或丢弃空组合键。
        if text.chars().any(char::is_whitespace) || text.split('+').any(str::is_empty) {
            return Err(());
        }
        Shortcut::from_str(text).map_err(|_| ())
    }
    let show = parse(&settings.hotkey_show_window).map_err(|_| "显示窗口快捷键格式无效")?;
    let clipboard =
        parse(&settings.hotkey_clipboard_translate).map_err(|_| "选中文字翻译快捷键格式无效")?;
    if show.id() == clipboard.id() {
        return Err("两个全局快捷键不能相同".into());
    }
    Ok([show, clipboard])
}
