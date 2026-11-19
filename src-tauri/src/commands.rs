use crate::{application, desktop::popup, model::*, AppState};
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_clipboard_manager::ClipboardExt;
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_opener::OpenerExt;

pub fn snapshot(state: &AppState) -> Result<Bootstrap, String> {
    let (settings, ai_channels) = state.store.configuration_snapshot()?;
    let ai_config = ai_channels.active_config()?.clone();
    let warnings = state
        .desktop
        .warnings
        .lock()
        .map_err(|_| "启动状态不可用")?
        .clone();
    Ok(Bootstrap {
        settings,
        ai_config,
        ai_channels,
        data_dir: state.store.data_dir.to_string_lossy().into(),
        warnings,
    })
}

#[tauri::command]
pub fn bootstrap(app: AppHandle) -> Result<Bootstrap, String> {
    if let Some(failure) = app.try_state::<crate::StartupFailure>() {
        return Err(format!(
            "启动失败：{}。请检查数据目录并重新启动应用。",
            failure.0
        ));
    }
    let state = app.try_state::<AppState>().ok_or("应用尚未初始化")?;
    snapshot(&state)
}

#[tauri::command]
pub fn get_target_language(state: State<'_, AppState>) -> Result<TargetLanguage, String> {
    state.store.target_language()
}

#[tauri::command]
pub fn set_target_language(
    language: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<TargetLanguage, String> {
    let saved = state.store.set_target_language(&language)?;
    // 两个 WebView 只接收已提交的语言代码及版本，窗口重载后可直接读取最新值。
    // 正在关闭的窗口可能收不到事件，重新进入时会核对快照；提交成功仍应回执成功。
    let _ = app.emit("target-language-changed", &saved);
    Ok(saved)
}

#[tauri::command]
pub async fn translate(
    request: TranslationRequest,
    on_delta: tauri::ipc::Channel<String>,
    state: State<'_, AppState>,
) -> Result<Translation, String> {
    application::translate(&state.store, &state.ai, &request, |text| {
        // 此通道只回到发起请求的 WebView。窗口关闭后，正式请求仍按原规则完成入库。
        let _ = on_delta.send(text.to_owned());
    })
    .await
}

#[tauri::command]
pub async fn test_ai_connection(
    config: AiConfig,
    state: State<'_, AppState>,
) -> Result<Translation, String> {
    // 只对传入快照发起测试，既不写配置，也不保存测试翻译历史。
    state.ai.test(&config).await
}

#[tauri::command]
pub async fn list_ai_models(
    config: AiConfig,
    state: State<'_, AppState>,
) -> Result<Vec<String>, String> {
    // 草稿模型发现不要求先选模型，也不进入配置保存或翻译入库流程。
    state.ai.list_models(&config).await
}

// 只打开编译时固定官网，不接受前端 URL、路径或进程参数，也不传递配置凭据。
fn open_official_website_with(open: impl FnOnce(&str) -> Result<(), ()>) -> Result<(), String> {
    open(OFFICIAL_AI_WEBSITE).map_err(|()| "OFFICIAL_WEBSITE_OPEN_FAILED".to_owned())
}

#[tauri::command]
pub async fn open_official_website(app: AppHandle) -> Result<(), String> {
    // 操作系统打开浏览器可能阻塞，不占用 WebView 的异步任务线程。
    tauri::async_runtime::spawn_blocking(move || {
        open_official_website_with(|url| app.opener().open_url(url, None::<&str>).map_err(|_| ()))
    })
    .await
    .map_err(|_| "OFFICIAL_WEBSITE_OPEN_FAILED".to_owned())?
}

struct DesktopSettings<'a> {
    app: &'a AppHandle,
    runtime: &'a crate::desktop::DesktopRuntime,
    window: tauri::WebviewWindow,
}

impl application::SettingsEffects for DesktopSettings<'_> {
    fn replace_shortcuts(&self, settings: &Settings) -> Result<(), String> {
        self.runtime.replace_shortcuts(self.app, settings)
    }

    fn set_always_on_top(&self, enabled: bool) -> Result<(), String> {
        self.window
            .set_always_on_top(enabled)
            .map_err(|e| e.to_string())
    }
}

#[tauri::command]
pub fn save_settings(
    settings: Settings,
    ai_channels: AiChannels,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Bootstrap, String> {
    let window = app.get_webview_window("main").ok_or("主窗口不可用")?;
    let effects = DesktopSettings {
        app: &app,
        runtime: &state.desktop,
        window,
    };
    application::save_settings(&state.store, &effects, &settings, &ai_channels)?;
    if let Err(error) = state.desktop.init_tray(&app, &settings) {
        if let Ok(mut warnings) = state.desktop.warnings.lock() {
            warnings.push(error);
        }
    }
    snapshot(&state)
}

#[tauri::command]
pub fn list_history(
    keyword: String,
    limit: usize,
    offset: usize,
    state: State<'_, AppState>,
) -> Result<Page<HistoryRecord>, String> {
    state.store.list_history(&keyword, limit, offset)
}
#[tauri::command]
pub fn delete_history(id: i64, state: State<'_, AppState>) -> Result<(), String> {
    state.store.delete_history(id)
}
#[tauri::command]
pub fn clear_history(state: State<'_, AppState>) -> Result<(), String> {
    state.store.clear_history()
}
#[tauri::command]
pub fn list_favorites(
    keyword: String,
    limit: usize,
    offset: usize,
    state: State<'_, AppState>,
) -> Result<Page<FavoriteWord>, String> {
    state.store.list_favorites(&keyword, limit, offset)
}
#[tauri::command]
pub fn save_favorite(
    word: String,
    translation: String,
    note: Option<String>,
    state: State<'_, AppState>,
) -> Result<FavoriteWord, String> {
    state
        .store
        .save_favorite(&word, &translation, note.as_deref())
}
#[tauri::command]
pub fn delete_favorite(id: i64, state: State<'_, AppState>) -> Result<(), String> {
    state.store.delete_favorite(id)
}
#[tauri::command]
pub fn copy_text(text: String, app: AppHandle) -> Result<(), String> {
    app.clipboard()
        .write_text(text)
        .map_err(|error| format!("复制失败：{error}"))
}

#[tauri::command]
pub fn popup_snapshot(state: State<'_, AppState>) -> popup::Snapshot {
    state.desktop.popup.snapshot()
}

#[tauri::command]
pub async fn popup_ready(app: AppHandle, request_id: u64) -> Result<(), String> {
    popup::on_ui(app, move |app, state| {
        state.desktop.popup.ready(app, request_id);
        Ok(())
    })
    .await
}

#[tauri::command]
pub async fn popup_retranslate(
    app: AppHandle,
    request_id: u64,
    request: TranslationRequest,
) -> Result<popup::Snapshot, String> {
    let config = popup::saved_channel(&app).await;
    popup::on_ui(app, move |app, state| {
        state
            .desktop
            .popup
            .retranslate(app, request_id, request, config)
    })
    .await
}

#[tauri::command]
pub async fn popup_pin(app: AppHandle, request_id: u64, pinned: bool) -> Result<(), String> {
    popup::on_ui(app, move |app, state| {
        state.desktop.popup.pin(app, request_id, pinned)
    })
    .await
}

#[tauri::command]
pub async fn popup_copy(app: AppHandle, request_id: u64) -> Result<(), String> {
    popup::on_ui(app, move |app, state| {
        state.desktop.popup.copy(app, request_id)
    })
    .await
}

#[tauri::command]
pub async fn popup_start_drag(window: tauri::Window) -> Result<(), String> {
    // 调用窗口由宿主注入，WebView 无需取得可指定其它窗口的通用权限。
    window
        .start_dragging()
        .map_err(|_| "POPUP_WINDOW_FAILED".into())
}

#[tauri::command]
pub async fn popup_dismiss(app: AppHandle, request_id: u64) -> Result<(), String> {
    popup::on_ui(app, move |app, state| {
        state.desktop.popup.dismiss(app, Some(request_id))
    })
    .await
}

#[tauri::command]
pub async fn export_favorites(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Option<String>, String> {
    let content = state.store.export_favorites()?;
    // 原生文件选择和写入交给后台任务，避免阻塞 WebView 线程。
    tauri::async_runtime::spawn_blocking(move || {
        let Some(file) = app
            .dialog()
            .file()
            .set_file_name("favorites.txt")
            .blocking_save_file()
        else {
            return Ok(None);
        };
        let path = file.into_path().map_err(|e| e.to_string())?;
        application::write_export(&path, &content).map(Some)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{application::record_translation, storage::Store, validation::validate_settings};

    #[test]
    fn official_website_opener_receives_only_the_fixed_https_address() {
        let mut calls = Vec::new();
        let result = open_official_website_with(|url| {
            calls.push(url.to_owned());
            Ok(())
        });
        assert!(result.is_ok());
        assert_eq!(calls, ["https://uptoken.top"]);
    }

    #[test]
    fn official_website_opener_failure_is_a_stable_error_without_system_details() {
        let result = open_official_website_with(|_| Err(()));
        assert_eq!(result.unwrap_err(), "OFFICIAL_WEBSITE_OPEN_FAILED");
    }

    fn result() -> Translation {
        Translation {
            source_text: "hello".into(),
            translated_text: "你好".into(),
            source_lang: "en".into(),
            target_lang: "zh".into(),
            model: "fixture-model".into(),
            create_time: "2026-10-03T12:00:00.000Z".into(),
            warning: None,
        }
    }

    #[test]
    fn persistence_failure_preserves_translation_and_returns_warning() {
        let directory = tempfile::tempdir().unwrap();
        let store = Store::open(directory.path()).unwrap();
        rusqlite::Connection::open(directory.path().join("upfanyi.sqlite3"))
            .unwrap()
            .execute("DROP TABLE translation_history", [])
            .unwrap();
        let translation = record_translation(&store, result());
        assert_eq!(translation.translated_text, "你好");
        assert!(translation.warning.is_some());
    }

    #[test]
    fn application_snapshot_and_successful_history_use_real_store() {
        let directory = tempfile::tempdir().unwrap();
        let state = AppState {
            store: std::sync::Arc::new(Store::open(directory.path()).unwrap()),
            ai: crate::ai::AiService::new().unwrap(),
            desktop: std::sync::Arc::new(crate::desktop::DesktopRuntime::default()),
        };
        let initial = snapshot(&state).unwrap();
        assert_eq!(initial.ai_config, AiConfig::default());
        assert_eq!(initial.ai_channels, AiChannels::default());
        let serialized = serde_json::to_value(initial).unwrap();
        assert!(serialized.get("engines").is_none());
        assert!(serialized.get("configs").is_none());
        let channels = crate::test_support::channel_collection(&AiConfig {
            model: "active-fixture".into(),
            ..AiConfig::default()
        });
        state
            .store
            .save_configuration(&Settings::default(), &channels)
            .unwrap();
        let projected = snapshot(&state).unwrap();
        assert_eq!(projected.ai_channels, channels);
        assert_eq!(
            projected.ai_config,
            projected.ai_channels.active_config().unwrap().clone()
        );
        state
            .desktop
            .warnings
            .lock()
            .unwrap()
            .push("测试托盘警告".into());
        assert_eq!(snapshot(&state).unwrap().warnings, ["测试托盘警告"]);
        assert!(record_translation(&state.store, result()).warning.is_none());
        let page = state.store.list_history("hello", 10, 0).unwrap();
        assert_eq!(page.total, 1);
        assert_eq!(page.items[0].translated_text, "你好");
    }

    #[test]
    fn settings_reject_bad_lengths_locales_and_target_auto() {
        assert!(validate_settings(&Settings::default()).is_ok());
        for settings in [
            Settings {
                clipboard_min_length: 0,
                ..Settings::default()
            },
            Settings {
                locale: "unknown".into(),
                ..Settings::default()
            },
            Settings {
                default_target_lang: "auto".into(),
                ..Settings::default()
            },
        ] {
            assert!(validate_settings(&settings).is_err());
        }
    }
}
