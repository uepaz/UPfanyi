pub mod ai;
mod application;
mod channels;
mod commands;
mod desktop;
pub mod model;
pub mod storage;
mod validation;

#[cfg(test)]
mod test_support;

use std::{path::PathBuf, sync::Arc};
use tauri::{Manager, RunEvent, WindowEvent};

pub struct AppState {
    pub store: Arc<storage::Store>,
    pub ai: ai::AiService,
    pub desktop: Arc<desktop::DesktopRuntime>,
}

pub struct StartupFailure(pub String);

fn initialize(app: &mut tauri::App) -> Result<(), String> {
    let data_dir = data_directory(app.handle())?;
    let store = Arc::new(storage::Store::open(&data_dir)?);
    let settings = store.settings()?;
    let desktop = Arc::new(desktop::DesktopRuntime::default());
    let ai = ai::AiService::new()?;
    app.manage(AppState {
        store,
        ai,
        desktop: Arc::clone(&desktop),
    });
    let top_result = app
        .get_webview_window("main")
        .ok_or("主窗口不可用".to_owned())
        .and_then(|window| {
            window
                .set_always_on_top(settings.always_on_top)
                .map_err(|error| error.to_string())
        });
    for result in [
        top_result,
        desktop.init_tray(app.handle(), &settings),
        desktop.replace_shortcuts(app.handle(), &settings),
    ] {
        if let Err(error) = result {
            if let Ok(mut warnings) = desktop.warnings.lock() {
                warnings.push(error);
            }
        }
    }
    Ok(())
}

fn data_directory(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let data_root = match std::env::var_os("UPFANYI_DATA_DIR") {
        Some(path) => PathBuf::from(path),
        None => app.path().app_data_dir().map_err(|e| e.to_string())?,
    };
    // 独立格式目录从空白数据开始，启动过程不探测应用根目录中的其它数据文件。
    Ok(data_root.join("data-v3"))
}

pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_dialog::init())
        // 官网跳转仅由固定 IPC 发起，不启用任意 HTML 链接的自动打开。
        .plugin(
            tauri_plugin_opener::Builder::new()
                .open_js_links_on_click(false)
                .build(),
        )
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, shortcut, event| {
                    if let Some(state) = app.try_state::<AppState>() {
                        state.desktop.handle_shortcut(app, shortcut, event.state());
                    }
                })
                .build(),
        )
        .setup(|app| {
            // 保留 WebView 显示启动失败原因；损坏数据不能让发行版无声退出。
            if let Err(error) = initialize(app) {
                app.manage(StartupFailure(error));
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() == desktop::popup::LABEL {
                if let Some(state) = window.try_state::<AppState>() {
                    match event {
                        WindowEvent::CloseRequested { api, .. } => {
                            api.prevent_close();
                            let _ = state.desktop.popup.dismiss(window.app_handle(), None);
                        }
                        WindowEvent::Focused(focused) => {
                            state.desktop.popup.focus_changed(window, *focused)
                        }
                        _ => {}
                    }
                }
                return;
            }
            if let WindowEvent::CloseRequested { api, .. } = event {
                if let Some(state) = window.app_handle().try_state::<AppState>() {
                    if state
                        .store
                        .settings()
                        .is_ok_and(|settings| state.desktop.should_hide(&settings))
                    {
                        api.prevent_close();
                        let _ = window.hide();
                    } else {
                        state.desktop.shutdown(window.app_handle());
                        window.app_handle().exit(0);
                    }
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::bootstrap,
            commands::get_target_language,
            commands::set_target_language,
            commands::translate,
            commands::test_ai_connection,
            commands::list_ai_models,
            commands::open_official_website,
            commands::save_settings,
            commands::list_history,
            commands::delete_history,
            commands::clear_history,
            commands::list_favorites,
            commands::save_favorite,
            commands::delete_favorite,
            commands::export_favorites,
            commands::copy_text,
            commands::popup_snapshot,
            commands::popup_ready,
            commands::popup_retranslate,
            commands::popup_copy,
            commands::popup_start_drag,
            commands::popup_pin,
            commands::popup_dismiss
        ])
        .build(tauri::generate_context!())
        .expect("UP翻译初始化失败，请检查数据目录和运行环境");
    app.run(|handle, event| {
        if matches!(event, RunEvent::Exit | RunEvent::ExitRequested { .. }) {
            if let Some(state) = handle.try_state::<AppState>() {
                state.desktop.shutdown(handle);
            }
        }
    });
}
