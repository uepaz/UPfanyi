mod native;
mod placement;
mod session;
#[cfg(test)]
mod tests;

use crate::{
    model::{AiConfig, Settings, TranslationRequest},
    AppState,
};
pub(crate) use session::Snapshot;
use session::{Attempt, Session};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex, MutexGuard,
};
use tauri::{
    AppHandle, Emitter, EventTarget, Manager, PhysicalPosition, WebviewWindowBuilder, Window,
};
use tauri_plugin_clipboard_manager::ClipboardExt;

pub(crate) const LABEL: &str = "selection-popup";
const UPDATED: &str = "selection-popup-updated";
const WINDOW_ERROR: &str = "POPUP_WINDOW_FAILED";

#[derive(Clone)]
pub(super) struct Origin {
    pub cursor: Option<PhysicalPosition<i32>>,
    pub source_monitor: Arc<dyn Fn() -> Option<PhysicalPosition<i32>> + Send + Sync>,
    pub valid: Arc<dyn Fn() -> bool + Send + Sync>,
}

#[derive(Default)]
pub(crate) struct Popup {
    session: Mutex<Session>,
    creating: AtomicBool,
    stream_notice_pending: AtomicBool,
}

fn configuration(state: &AppState) -> Result<(Settings, AiConfig), String> {
    let (settings, channels) = state.store.configuration_snapshot()?;
    Ok((settings, channels.active_config()?.clone()))
}

async fn saved_configuration(app: &AppHandle) -> Result<(Settings, AiConfig), String> {
    let handle = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let state = handle.try_state::<AppState>().ok_or(WINDOW_ERROR)?;
        configuration(&state)
    })
    .await
    .map_err(|_| WINDOW_ERROR.to_owned())?
}

pub(crate) async fn saved_channel(app: &AppHandle) -> Result<AiConfig, String> {
    saved_configuration(app).await.map(|(_, config)| config)
}

impl Popup {
    fn lock(&self) -> MutexGuard<'_, Session> {
        self.session
            .lock()
            .unwrap_or_else(|error| error.into_inner())
    }

    pub(crate) fn snapshot(&self) -> Snapshot {
        self.lock().snapshot()
    }

    fn notify(&self, app: &AppHandle) {
        let version = self.lock().version();
        let _ = app.emit_to(EventTarget::webview_window(LABEL), UPDATED, version);
    }

    pub(super) fn offer(&self, app: &AppHandle, text: String, origin: Origin) {
        let id = self.lock().reserve();
        self.notify(app);
        let handle = app.clone();
        tauri::async_runtime::spawn(async move {
            let config = saved_configuration(&handle).await;
            let _ = on_ui(handle, move |app, state| {
                let popup = &state.desktop.popup;
                if !(origin.valid)() {
                    if popup.lock().abandon(id) {
                        popup.notify(app);
                    }
                } else if popup.lock().prepare_reserved(id, text, origin, config) {
                    popup.ensure_window(app, id);
                    popup.expire_if_unready(app, id);
                }
                Ok(())
            })
            .await;
        });
    }

    fn expire_if_unready(&self, app: &AppHandle, id: u64) {
        let handle = app.clone();
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_secs(8)).await;
            let _ = on_ui(handle, move |app, state| {
                let popup = &state.desktop.popup;
                if popup.lock().abandon(id) {
                    popup.notify(app);
                }
                Ok(())
            })
            .await;
        });
    }

    fn ensure_window(&self, app: &AppHandle, id: u64) {
        if app.get_webview_window(LABEL).is_some() {
            self.notify(app);
            return;
        }
        if !self.begin_creation() {
            return;
        }
        // Windows 创建 WebView 时需要消息循环继续处理回调，不能在 UI 线程等待构建。
        let handle = app.clone();
        tauri::async_runtime::spawn_blocking(move || {
            let created = handle
                .config()
                .app
                .windows
                .iter()
                .find(|c| c.label == LABEL)
                .is_some_and(|config| {
                    WebviewWindowBuilder::from_config(&handle, config)
                        .and_then(|builder| {
                            builder
                                .on_document_title_changed(|window, title| {
                                    let _ = window.set_title(&title);
                                })
                                .build()
                        })
                        .is_ok()
                });
            let host = handle.clone();
            let _ = handle.run_on_main_thread(move || {
                let state = host.state::<AppState>();
                let popup = &state.desktop.popup;
                let next = popup.finish_creation(id, created);
                if created {
                    popup.notify(&host);
                }
                if let Some(next) = next {
                    popup.ensure_window(&host, next);
                }
            });
        });
    }

    fn begin_creation(&self) -> bool {
        !self.creating.swap(true, Ordering::AcqRel)
    }

    fn finish_creation(&self, id: u64, created: bool) -> Option<u64> {
        self.creating.store(false, Ordering::Release);
        if created {
            return None;
        }
        let mut session = self.lock();
        session.abandon(id);
        session.pending_id()
    }

    // 仅当前选区的首次就绪回执可显示窗口。窗口操作前释放会话锁，允许焦点事件重入。
    fn present(
        &self,
        id: u64,
        show: impl FnOnce(&Origin) -> Result<(), String>,
    ) -> Option<Attempt> {
        let origin = self.lock().origin(id)?;
        if !(origin.valid)() || show(&origin).is_err() {
            self.lock().abandon(id);
            return None;
        }
        self.lock().present(id)
    }

    pub(crate) fn ready(&self, app: &AppHandle, id: u64) {
        if self.lock().pending_id() != Some(id) {
            return;
        }
        let attempt = self.present(id, |origin| {
            let w = app.get_webview_window(LABEL).ok_or(WINDOW_ERROR)?;
            let monitor_at = |p: PhysicalPosition<i32>| {
                app.monitor_from_point(p.x.into(), p.y.into())
                    .ok()
                    .flatten()
            };
            let cursor_monitor = origin.cursor.and_then(monitor_at);
            let cursor = cursor_monitor.as_ref().and(origin.cursor);
            let monitor = cursor_monitor
                .or_else(|| (origin.source_monitor)().and_then(monitor_at))
                .or_else(|| app.primary_monitor().ok().flatten())
                .ok_or(WINDOW_ERROR)?;
            let area = monitor.work_area();
            let (position, size) =
                placement::placement(cursor, area.position, area.size, monitor.scale_factor());
            let previous = if self.lock().visible() {
                Some((
                    w.outer_position().map_err(|_| WINDOW_ERROR)?,
                    w.inner_size().map_err(|_| WINDOW_ERROR)?,
                ))
            } else {
                None
            };
            if !(origin.valid)() {
                return Err(WINDOW_ERROR.into());
            }
            let show = || -> Result<(), String> {
                w.set_position(position)
                    .and_then(|()| w.set_size(size))
                    .map_err(|_| WINDOW_ERROR)?;
                if previous.is_none() {
                    native::pin(&w, false)?;
                }
                if !(origin.valid)() {
                    return Err(WINDOW_ERROR.into());
                }
                w.show()
                    .and_then(|()| w.set_focus())
                    .map_err(|_| WINDOW_ERROR)?;
                if !native::foreground(&w.as_ref().window()) {
                    return Err(WINDOW_ERROR.into());
                }
                w.as_ref().set_focus().map_err(|_| WINDOW_ERROR.to_owned())
            };
            let result = show();
            if result.is_err() {
                if let Some((position, size)) = previous {
                    let _ = w.set_position(position);
                    let _ = w.set_size(size);
                }
            }
            result
        });
        if attempt.is_none() && !self.lock().visible() {
            if let Some(window) = app.get_webview_window(LABEL) {
                let _ = window.hide();
            }
        }
        self.dispatch(app, attempt);
    }

    pub(crate) fn retranslate(
        &self,
        app: &AppHandle,
        id: u64,
        request: TranslationRequest,
        config: Result<AiConfig, String>,
    ) -> Result<Snapshot, String> {
        let attempt = self.lock().retranslate(id, request, config)?;
        self.dispatch(app, attempt);
        Ok(self.snapshot())
    }

    fn notify_stream(&self, app: &AppHandle) {
        // 多个增量共用一次版本通知。计时到期再拉取最新快照，关闭或换向后不会重放旧文本。
        if self.stream_notice_pending.swap(true, Ordering::AcqRel) {
            return;
        }
        let handle = app.clone();
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(25)).await;
            if let Some(state) = handle.try_state::<AppState>() {
                let popup = &state.desktop.popup;
                popup.stream_notice_pending.store(false, Ordering::Release);
                popup.notify(&handle);
            }
        });
    }

    fn dispatch(&self, app: &AppHandle, attempt: Option<Attempt>) {
        self.notify(app);
        if let Some(attempt) = attempt {
            let handle = app.clone();
            tauri::async_runtime::spawn(async move {
                let state = handle.state::<AppState>();
                let id = attempt.id;
                let popup = &state.desktop.popup;
                let result = attempt
                    .run(&state.store, &state.ai, |piece| {
                        if popup.lock().append(id, piece) {
                            popup.notify_stream(&handle);
                        }
                    })
                    .await;
                if popup.lock().finish(id, result) {
                    popup.notify(&handle);
                }
            });
        }
    }

    pub(crate) fn pin(&self, app: &AppHandle, id: u64, pinned: bool) -> Result<(), String> {
        self.apply_pin(id, pinned, || {
            native::pin(&app.get_webview_window(LABEL).ok_or(WINDOW_ERROR)?, pinned)
        })?;
        self.notify(app);
        Ok(())
    }

    fn apply_pin(
        &self,
        id: u64,
        pinned: bool,
        change: impl FnOnce() -> Result<(), String>,
    ) -> Result<(), String> {
        if self.lock().active(id).is_some() {
            change()?;
            let mut session = self.lock();
            if session.active(id).is_some() {
                session.set_pinned(pinned);
            }
        }
        Ok(())
    }

    pub(crate) fn copy(&self, app: &AppHandle, id: u64) -> Result<(), String> {
        let text = self.lock().copied_text(id).ok_or("POPUP_STALE")?.to_owned();
        app.clipboard()
            .write_text(text)
            .map_err(|_| "POPUP_COPY_FAILED".into())
    }

    pub(crate) fn dismiss(&self, app: &AppHandle, id: Option<u64>) -> Result<(), String> {
        if id.is_some_and(|id| !self.lock().matches_request(id)) {
            return Ok(());
        }
        if let Some(window) = app.get_webview_window(LABEL) {
            window.hide().map_err(|_| WINDOW_ERROR)?;
            let _ = native::pin(&window, false);
        }
        self.clear();
        self.notify(app);
        Ok(())
    }

    pub(crate) fn focus_changed(&self, window: &Window, focused: bool) {
        if native::foreground(window) != focused {
            return;
        }
        let dismiss = self.lock().focus_changed(focused);
        if dismiss {
            let _ = self.dismiss(window.app_handle(), None);
        }
    }

    pub(super) fn clear(&self) {
        self.lock().clear();
    }
}

// IPC 操作与原生关闭、焦点事件在同一线程排序。异步等待不会阻塞 WebView 消息循环。
pub(crate) async fn on_ui<T: Send + 'static>(
    app: AppHandle,
    action: impl FnOnce(&AppHandle, &AppState) -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    let (send, receive) = tokio::sync::oneshot::channel();
    let host = app.clone();
    app.run_on_main_thread(move || {
        let result = host
            .try_state::<AppState>()
            .ok_or_else(|| WINDOW_ERROR.to_owned())
            .and_then(|state| action(&host, &state));
        let _ = send.send(result);
    })
    .map_err(|_| WINDOW_ERROR)?;
    receive.await.map_err(|_| WINDOW_ERROR.to_owned())?
}
