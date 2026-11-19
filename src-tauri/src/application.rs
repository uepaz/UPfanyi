use crate::{ai, model::*, storage::Store, validation::validate_settings};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

// 应用流程不依赖 Tauri 句柄，便于用真实数据库和受控系统副作用验证失败补偿。
pub(crate) trait SettingsEffects {
    fn replace_shortcuts(&self, settings: &Settings) -> Result<(), String>;
    fn set_always_on_top(&self, enabled: bool) -> Result<(), String>;
}

pub(crate) fn save_settings(
    store: &Store,
    effects: &impl SettingsEffects,
    settings: &Settings,
    ai_channels: &AiChannels,
) -> Result<(), String> {
    validate_settings(settings)?;
    let ai_channels = crate::channels::normalize(ai_channels)?;
    let previous = store.settings()?;
    // 快捷键注册内部负责失败回滚；置顶或事务失败则恢复两项桌面状态。
    effects.replace_shortcuts(settings)?;
    let save = effects
        .set_always_on_top(settings.always_on_top)
        .and_then(|_| store.save_configuration(settings, &ai_channels));
    if let Err(error) = save {
        let shortcuts_rollback = effects.replace_shortcuts(&previous);
        let window_rollback = effects.set_always_on_top(previous.always_on_top);
        if shortcuts_rollback.is_err() || window_rollback.is_err() {
            return Err(format!(
                "设置未保存：{error}；桌面状态恢复失败，请重新启动应用"
            ));
        }
        return Err(error);
    }
    Ok(())
}

pub(crate) fn record_translation(store: &Store, mut translation: Translation) -> Translation {
    if let Err(error) = store.save_history(&translation) {
        translation.warning = Some(format!("历史记录未保存：{error}"));
    }
    translation
}

pub(crate) async fn translate(
    store: &Store,
    ai: &ai::AiService,
    request: &TranslationRequest,
    on_delta: impl FnMut(&str) + Send,
) -> Result<Translation, String> {
    let (_, channels) = store.configuration_snapshot()?;
    translate_with_config(store, ai, request, channels.active_config()?, on_delta).await
}

/// 调用方已取得本次配置快照，网络往返期间释放数据库锁。
pub(crate) async fn translate_with_config(
    store: &Store,
    ai: &ai::AiService,
    request: &TranslationRequest,
    config: &AiConfig,
    on_delta: impl FnMut(&str) + Send,
) -> Result<Translation, String> {
    let result = ai.translate_stream(request, config, on_delta).await?;
    Ok(record_translation(store, result))
}

pub(crate) fn write_export(path: &Path, content: &str) -> Result<String, String> {
    let directory = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let pending =
        PendingExport(directory.join(format!(".upfanyi-export-{}.tmp", uuid::Uuid::new_v4())));
    let write = || -> std::io::Result<()> {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&pending.0)?;
        file.write_all(content.as_bytes())?;
        file.sync_all()?;
        drop(file);
        // 导出允许用户覆盖所选文件，但完整写入临时文件后才替换，避免写入失败先截断旧导出。
        fs::rename(&pending.0, path)
    };
    write().map_err(|error| format!("导出失败：{error}"))?;
    Ok(path.to_string_lossy().into_owned())
}

struct PendingExport(PathBuf);

impl Drop for PendingExport {
    fn drop(&mut self) {
        // 只清理本次创建的精确文件；替换成功后该临时路径已不存在。
        let _ = fs::remove_file(&self.0);
    }
}

#[cfg(test)]
mod tests;
