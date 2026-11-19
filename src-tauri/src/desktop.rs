use crate::{model::Settings, AppState};
use std::{
    collections::HashSet,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};
#[cfg(windows)]
use std::{
    sync::atomic::AtomicU64,
    time::{Duration, Instant},
};
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager,
};
#[cfg(windows)]
use tauri_plugin_global_shortcut::{Code, Modifiers};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

#[cfg(any(windows, test))]
mod selection;
#[cfg(windows)]
use selection::windows::{Foreground, WindowsCopy};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ShortcutAction {
    Show,
    TranslateSelection,
}

#[cfg(windows)]
struct PendingSelection {
    shortcut: u32,
    generation: u64,
    target: Foreground,
}

pub struct DesktopRuntime {
    stop: AtomicBool,
    tray_available: AtomicBool,
    shortcuts: Mutex<Vec<(Shortcut, ShortcutAction)>>,
    shortcut_update: Mutex<()>,
    pressed: Mutex<HashSet<u32>>,
    #[cfg(windows)]
    pending_selection: Mutex<Option<PendingSelection>>,
    #[cfg(windows)]
    selection_running: AtomicBool,
    #[cfg(windows)]
    selection_generation: AtomicU64,
    pub warnings: Mutex<Vec<String>>,
}

impl Default for DesktopRuntime {
    fn default() -> Self {
        Self {
            stop: AtomicBool::new(false),
            tray_available: AtomicBool::new(false),
            shortcuts: Mutex::new(Vec::new()),
            shortcut_update: Mutex::new(()),
            pressed: Mutex::new(HashSet::new()),
            #[cfg(windows)]
            pending_selection: Mutex::new(None),
            #[cfg(windows)]
            selection_running: AtomicBool::new(false),
            #[cfg(windows)]
            selection_generation: AtomicU64::new(0),
            warnings: Mutex::new(Vec::new()),
        }
    }
}

fn shortcut_pair(settings: &Settings) -> Result<Vec<(Shortcut, ShortcutAction)>, String> {
    let [show, selection] = crate::validation::shortcuts(settings)?;
    #[cfg(windows)]
    {
        // 注册 Ctrl+C 会截走发送给前台窗口的复制按键。仅拒绝注册，不使既有库无法打开。
        let copy = Shortcut::new(Some(Modifiers::CONTROL), Code::KeyC);
        if [show.id(), selection.id()].contains(&copy.id()) {
            return Err("Ctrl+C 用于普通复制，请为应用选择其他快捷键".into());
        }
    }
    Ok(vec![
        (show, ShortcutAction::Show),
        (selection, ShortcutAction::TranslateSelection),
    ])
}

trait ShortcutRegistry {
    fn register(&mut self, shortcut: Shortcut) -> Result<(), String>;
    fn unregister(&mut self, shortcut: Shortcut) -> Result<(), String>;
}

struct SystemShortcutRegistry<'a>(&'a AppHandle);

impl ShortcutRegistry for SystemShortcutRegistry<'_> {
    fn register(&mut self, shortcut: Shortcut) -> Result<(), String> {
        self.0
            .global_shortcut()
            .register(shortcut)
            .map_err(|error| error.to_string())
    }
    fn unregister(&mut self, shortcut: Shortcut) -> Result<(), String> {
        self.0
            .global_shortcut()
            .unregister(shortcut)
            .map_err(|error| error.to_string())
    }
}

fn rollback_shortcuts(
    registry: &mut impl ShortcutRegistry,
    added: &[Shortcut],
    removed: &[Shortcut],
    error: String,
) -> String {
    let mut failed = false;
    for shortcut in added.iter().rev() {
        if registry.unregister(*shortcut).is_err() {
            failed = true;
        }
    }
    // 只恢复确实已卸载的组合，避免对仍注册的旧组合重复注册。
    for shortcut in removed {
        if registry.register(*shortcut).is_err() {
            failed = true;
        }
    }
    if failed {
        format!("{error}；快捷键恢复失败，请重新启动应用")
    } else {
        error
    }
}

fn replace_registered_shortcuts(
    registry: &mut impl ShortcutRegistry,
    current: &mut Vec<(Shortcut, ShortcutAction)>,
    next: Vec<(Shortcut, ShortcutAction)>,
) -> Result<(), String> {
    let mut added = Vec::new();
    let mut removed = Vec::new();
    // 先注册新组合；仅交换两个动作时不会重新向操作系统抢占同一组合。
    for (shortcut, _) in &next {
        if current.iter().any(|(old, _)| old.id() == shortcut.id()) {
            continue;
        }
        if let Err(error) = registry.register(*shortcut) {
            return Err(rollback_shortcuts(
                registry,
                &added,
                &removed,
                format!("快捷键注册失败（可能已被占用）：{error}"),
            ));
        }
        added.push(*shortcut);
    }
    for (shortcut, _) in current.iter() {
        if next.iter().any(|(new, _)| new.id() == shortcut.id()) {
            continue;
        }
        if let Err(error) = registry.unregister(*shortcut) {
            return Err(rollback_shortcuts(
                registry,
                &added,
                &removed,
                format!("快捷键替换失败：{error}"),
            ));
        }
        removed.push(*shortcut);
    }
    *current = next;
    Ok(())
}

impl DesktopRuntime {
    pub fn replace_shortcuts(&self, app: &AppHandle, settings: &Settings) -> Result<(), String> {
        let next = shortcut_pair(settings)?;
        let _update = self
            .shortcut_update
            .lock()
            .map_err(|_| "快捷键更新状态不可用".to_string())?;
        if self.stop.load(Ordering::Acquire) {
            return Err("应用正在退出".into());
        }
        // 插件注册会同步等待主线程；不能持有按键回调也要读取的配置锁。
        let mut current = self
            .shortcuts
            .lock()
            .map_err(|_| "快捷键状态不可用".to_string())?
            .clone();
        replace_registered_shortcuts(&mut SystemShortcutRegistry(app), &mut current, next)?;
        #[cfg(windows)]
        {
            self.selection_generation.fetch_add(1, Ordering::AcqRel);
            if let Ok(mut pending) = self.pending_selection.lock() {
                *pending = None;
            }
        }
        if let Ok(mut pressed) = self.pressed.lock() {
            // 同一组合仍被按住时保持去重状态，不因保存设置而再触发一次。
            pressed.retain(|id| current.iter().any(|(shortcut, _)| shortcut.id() == *id));
        }
        *self
            .shortcuts
            .lock()
            .map_err(|_| "快捷键状态不可用".to_string())? = current;
        Ok(())
    }

    pub fn handle_shortcut(
        self: &Arc<Self>,
        app: &AppHandle,
        shortcut: &Shortcut,
        state: ShortcutState,
    ) {
        if self.stop.load(Ordering::Acquire) {
            return;
        }
        let Ok(mut pressed) = self.pressed.lock() else {
            return;
        };
        if state == ShortcutState::Released {
            if !pressed.remove(&shortcut.id()) {
                return;
            }
            drop(pressed);
            #[cfg(windows)]
            self.finish_selection(app, shortcut.id());
            return;
        }
        if !pressed.insert(shortcut.id()) {
            return;
        }
        drop(pressed);
        let action = self.shortcuts.lock().ok().and_then(|items| {
            items
                .iter()
                .find(|(key, _)| key.id() == shortcut.id())
                .map(|(_, action)| *action)
        });
        match action {
            Some(ShortcutAction::Show) => show_window(app),
            Some(ShortcutAction::TranslateSelection) => {
                #[cfg(windows)]
                if !self.selection_running.load(Ordering::Acquire) {
                    if let Ok(mut pending) = self.pending_selection.lock() {
                        let generation = self.selection_generation.load(Ordering::Acquire);
                        *pending = Foreground::current().map(|target| PendingSelection {
                            shortcut: shortcut.id(),
                            generation,
                            target,
                        });
                    }
                }
            }
            None => {}
        }
    }

    #[cfg(windows)]
    fn finish_selection(self: &Arc<Self>, app: &AppHandle, shortcut: u32) {
        let Ok(mut pending) = self.pending_selection.lock() else {
            return;
        };
        if pending
            .as_ref()
            .is_none_or(|request| request.shortcut != shortcut)
        {
            return;
        }
        let Some(request) = pending.take() else {
            return;
        };
        drop(pending);
        if !self.selection_is_current(&request)
            || self.selection_running.swap(true, Ordering::AcqRel)
        {
            return;
        }
        let job = SelectionJob(Arc::clone(self));
        let handle = app.clone();
        let deadline = Instant::now() + Duration::from_secs(2);
        tauri::async_runtime::spawn_blocking(move || {
            let source = WindowsCopy {
                target: request.target,
                app: &handle,
            };
            let Some(text) = selection::capture_text(
                &source,
                || Instant::now() < deadline && job.0.selection_is_current(&request),
                || std::thread::sleep(Duration::from_millis(20)),
                50,
            ) else {
                return;
            };
            let window_handle = handle.clone();
            let _ = handle.run_on_main_thread(move || {
                if Instant::now() < deadline
                    && job.0.selection_is_current(&request)
                    && request.target.is_current()
                    && present_window(&window_handle)
                {
                    let _ = window_handle.emit("desktop-clipboard", text);
                }
            });
        });
    }

    #[cfg(windows)]
    fn selection_is_current(&self, request: &PendingSelection) -> bool {
        !self.stop.load(Ordering::Acquire)
            && self.selection_generation.load(Ordering::Acquire) == request.generation
    }

    pub fn should_hide(&self, settings: &Settings) -> bool {
        settings.minimize_to_tray && self.tray_available.load(Ordering::Acquire)
    }

    pub fn shutdown(&self, app: &AppHandle) {
        self.stop.store(true, Ordering::Release);
        let _ = app.global_shortcut().unregister_all();
    }

    pub fn init_tray(&self, app: &AppHandle, settings: &Settings) -> Result<(), String> {
        let english = settings.locale == "en_US";
        let show = MenuItem::with_id(
            app,
            "show",
            if english {
                "Show UPfanyi"
            } else {
                "显示 UP翻译"
            },
            true,
            None::<&str>,
        )
        .map_err(|e| e.to_string())?;
        let preferences = MenuItem::with_id(
            app,
            "settings",
            if english { "Settings" } else { "设置" },
            true,
            None::<&str>,
        )
        .map_err(|e| e.to_string())?;
        let quit = MenuItem::with_id(
            app,
            "quit",
            if english { "Exit" } else { "退出" },
            true,
            None::<&str>,
        )
        .map_err(|e| e.to_string())?;
        let menu =
            Menu::with_items(app, &[&show, &preferences, &quit]).map_err(|e| e.to_string())?;
        if let Some(tray) = app.tray_by_id("main-tray") {
            tray.set_menu(Some(menu)).map_err(|e| e.to_string())?;
            return Ok(());
        }
        let icon = app.default_window_icon().ok_or("缺少应用图标")?.clone();
        TrayIconBuilder::with_id("main-tray")
            .icon(icon)
            .tooltip("UP翻译 · UPfanyi")
            .menu(&menu)
            .show_menu_on_left_click(false)
            .on_menu_event(|app, event| match event.id.as_ref() {
                "show" => show_window(app),
                "settings" => {
                    show_window(app);
                    let _ = app.emit("desktop-navigate", "settings");
                }
                "quit" => {
                    if let Some(state) = app.try_state::<AppState>() {
                        state.desktop.shutdown(app);
                    }
                    app.exit(0);
                }
                _ => {}
            })
            .on_tray_icon_event(|tray, event| {
                if matches!(
                    event,
                    TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    }
                ) {
                    show_window(tray.app_handle());
                }
            })
            .build(app)
            .map_err(|e| format!("系统托盘初始化失败：{e}"))?;
        self.tray_available.store(true, Ordering::Release);
        Ok(())
    }
}

#[cfg(windows)]
struct SelectionJob(Arc<DesktopRuntime>);

#[cfg(windows)]
impl Drop for SelectionJob {
    fn drop(&mut self) {
        self.0.selection_running.store(false, Ordering::Release);
    }
}

pub fn show_window(app: &AppHandle) {
    let handle = app.clone();
    // 托盘和快捷键共用主线程窗口操作，退出后丢弃待执行的显示请求。
    let _ = app.run_on_main_thread(move || {
        present_window(&handle);
    });
}

fn present_window(handle: &AppHandle) -> bool {
    if handle
        .try_state::<AppState>()
        .is_some_and(|state| state.desktop.stop.load(Ordering::Acquire))
    {
        return false;
    }
    let Some(window) = handle.get_webview_window("main") else {
        return false;
    };
    if window.show().is_err() {
        return false;
    }
    let _ = window.unminimize();
    let _ = window.set_focus();
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeRegistry {
        registered: HashSet<u32>,
        calls: Vec<(&'static str, u32)>,
        fail_register: Option<u32>,
        fail_unregister: Option<u32>,
    }

    impl FakeRegistry {
        fn new(current: &[(Shortcut, ShortcutAction)]) -> Self {
            Self {
                registered: current.iter().map(|(shortcut, _)| shortcut.id()).collect(),
                calls: Vec::new(),
                fail_register: None,
                fail_unregister: None,
            }
        }
    }

    impl ShortcutRegistry for FakeRegistry {
        fn register(&mut self, shortcut: Shortcut) -> Result<(), String> {
            self.calls.push(("register", shortcut.id()));
            if self.fail_register == Some(shortcut.id()) {
                return Err("虚构注册冲突".into());
            }
            if !self.registered.insert(shortcut.id()) {
                return Err("重复注册".into());
            }
            Ok(())
        }

        fn unregister(&mut self, shortcut: Shortcut) -> Result<(), String> {
            self.calls.push(("unregister", shortcut.id()));
            if self.fail_unregister == Some(shortcut.id()) {
                return Err("虚构卸载失败".into());
            }
            if !self.registered.remove(&shortcut.id()) {
                return Err("组合未注册".into());
            }
            Ok(())
        }
    }

    fn changed_shortcuts() -> Vec<(Shortcut, ShortcutAction)> {
        shortcut_pair(&Settings {
            hotkey_show_window: "Ctrl+Shift+Y".into(),
            hotkey_clipboard_translate: "Ctrl+Shift+U".into(),
            ..Settings::default()
        })
        .unwrap()
    }

    fn identities(shortcuts: &[(Shortcut, ShortcutAction)]) -> Vec<(u32, ShortcutAction)> {
        shortcuts
            .iter()
            .map(|(shortcut, action)| (shortcut.id(), *action))
            .collect()
    }

    #[test]
    fn registration_failure_undoes_added_shortcut_and_preserves_old_pair() {
        let mut current = shortcut_pair(&Settings::default()).unwrap();
        let original = identities(&current);
        let next = changed_shortcuts();
        let mut registry = FakeRegistry::new(&current);
        let registered_before = registry.registered.clone();
        registry.fail_register = Some(next[1].0.id());
        assert!(
            replace_registered_shortcuts(&mut registry, &mut current, next.clone())
                .unwrap_err()
                .contains("注册失败")
        );
        assert_eq!(identities(&current), original);
        assert_eq!(registry.registered, registered_before);
        assert_eq!(
            registry.calls,
            [
                ("register", next[0].0.id()),
                ("register", next[1].0.id()),
                ("unregister", next[0].0.id())
            ]
        );
    }

    #[test]
    fn unregister_failure_restores_only_removed_old_shortcuts() {
        let mut current = shortcut_pair(&Settings::default()).unwrap();
        let original = identities(&current);
        let mut registry = FakeRegistry::new(&current);
        let registered_before = registry.registered.clone();
        registry.fail_unregister = Some(current[1].0.id());
        assert!(
            replace_registered_shortcuts(&mut registry, &mut current, changed_shortcuts()).is_err()
        );
        assert_eq!(identities(&current), original);
        assert_eq!(registry.registered, registered_before);
        assert_eq!(
            registry
                .calls
                .iter()
                .filter(|(operation, id)| *operation == "register" && *id == original[0].0)
                .count(),
            1
        );
        assert_eq!(
            registry
                .calls
                .iter()
                .filter(|(operation, id)| *operation == "register" && *id == original[1].0)
                .count(),
            0
        );
    }

    #[test]
    fn swapping_actions_reuses_existing_os_registrations() {
        let mut current = shortcut_pair(&Settings::default()).unwrap();
        let original = identities(&current);
        let next = vec![
            (current[1].0, ShortcutAction::Show),
            (current[0].0, ShortcutAction::TranslateSelection),
        ];
        let mut registry = FakeRegistry::new(&current);
        let registered_before = registry.registered.clone();
        replace_registered_shortcuts(&mut registry, &mut current, next).unwrap();
        assert!(registry.calls.is_empty());
        assert_eq!(registry.registered, registered_before);
        assert_eq!(
            identities(&current),
            [
                (original[1].0, ShortcutAction::Show),
                (original[0].0, ShortcutAction::TranslateSelection)
            ]
        );
    }

    #[test]
    fn successful_replacement_registers_new_pair_before_removing_old_pair() {
        let mut current = shortcut_pair(&Settings::default()).unwrap();
        let original = identities(&current);
        let next = changed_shortcuts();
        let mut registry = FakeRegistry::new(&current);
        replace_registered_shortcuts(&mut registry, &mut current, next.clone()).unwrap();
        assert_eq!(identities(&current), identities(&next));
        assert_eq!(
            registry.registered,
            next.iter().map(|(shortcut, _)| shortcut.id()).collect()
        );
        assert_eq!(
            registry.calls,
            [
                ("register", next[0].0.id()),
                ("register", next[1].0.id()),
                ("unregister", original[0].0),
                ("unregister", original[1].0)
            ]
        );
    }

    #[test]
    fn failed_rollback_is_reported_instead_of_claiming_old_pair_is_restored() {
        let mut current = shortcut_pair(&Settings::default()).unwrap();
        let original = identities(&current);
        let next = changed_shortcuts();
        let mut registry = FakeRegistry::new(&current);
        registry.fail_register = Some(next[1].0.id());
        registry.fail_unregister = Some(next[0].0.id());
        let error = replace_registered_shortcuts(&mut registry, &mut current, next).unwrap_err();
        assert!(error.contains("恢复失败"));
        assert_eq!(identities(&current), original);
    }

    #[test]
    fn shortcuts_accept_current_format_and_reject_space_separated_input() {
        let mut settings = Settings::default();
        let defaults = shortcut_pair(&settings).unwrap();
        assert_ne!(defaults[0].0.id(), defaults[1].0.id());
        for invalid in [
            "ctrl shift T",
            "Ctrl +Shift+T",
            "Ctrl++T",
            "Ctrl+Shift+T ",
            "",
        ] {
            settings.hotkey_show_window = invalid.into();
            assert!(
                shortcut_pair(&settings).is_err(),
                "invalid shortcut accepted: {invalid}"
            );
        }
        settings = Settings::default();
        settings.hotkey_clipboard_translate = "ctrl shift C".into();
        assert!(shortcut_pair(&settings).is_err());
    }

    #[test]
    fn duplicate_and_invalid_shortcuts_fail_before_registration() {
        let mut settings = Settings::default();
        settings
            .hotkey_clipboard_translate
            .clone_from(&settings.hotkey_show_window);
        assert!(shortcut_pair(&settings).is_err());
        settings.hotkey_show_window = "invalid hotkey".into();
        assert!(shortcut_pair(&settings).is_err());
    }

    #[cfg(windows)]
    #[test]
    fn native_shortcuts_leave_normal_copy_available_to_the_foreground_window() {
        for settings in [
            Settings {
                hotkey_show_window: "Ctrl+C".into(),
                ..Settings::default()
            },
            Settings {
                hotkey_clipboard_translate: "Control+KeyC".into(),
                ..Settings::default()
            },
        ] {
            assert!(crate::validation::validate_settings(&settings).is_ok());
            assert!(shortcut_pair(&settings).unwrap_err().contains("Ctrl+C"));
        }
        assert!(shortcut_pair(&Settings::default()).is_ok());
    }

    #[test]
    fn tray_unavailable_never_hides_last_window() {
        let runtime = DesktopRuntime::default();
        assert!(!runtime.should_hide(&Settings::default()));
        runtime.tray_available.store(true, Ordering::Release);
        assert!(runtime.should_hide(&Settings::default()));
        let settings = Settings {
            minimize_to_tray: false,
            ..Settings::default()
        };
        assert!(!runtime.should_hide(&settings));
    }
}
