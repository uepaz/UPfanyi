use super::*;
use crate::desktop::selection::{capture_text, ClipboardStamp, CopySource};
use crate::{
    ai::AiService,
    storage::Store,
    test_support::{channel_collection, chat_event, MockServer},
};
use serde_json::{json, Value};
use session::{Phase, PopupError};
use std::cell::{Cell, RefCell};
use tauri::PhysicalSize;

// 模拟编辑器执行普通复制，记录剪贴板的实际读写及复制阶段移动后的鼠标位置。
struct Editor {
    selected: &'static str,
    clipboard: RefCell<String>,
    sequence: Cell<u32>,
    reads: Cell<u32>,
    cursor: Cell<PhysicalPosition<i32>>,
}

impl Editor {
    fn ordinary_copy(&self, text: &str) {
        *self.clipboard.borrow_mut() = text.into();
        self.sequence.update(|n| n + 1);
    }
}

impl CopySource for Editor {
    fn target_is_current(&self) -> bool {
        true
    }
    fn copy_keys_released(&self) -> bool {
        true
    }
    fn clipboard_sequence(&self) -> u32 {
        self.sequence.get()
    }
    fn target_clipboard_stamp(&self) -> Option<ClipboardStamp> {
        Some(ClipboardStamp {
            sequence: self.sequence.get(),
            owner: 8,
        })
    }
    fn send_copy(&self) -> bool {
        self.ordinary_copy(self.selected);
        self.cursor.set(PhysicalPosition::new(700, 40));
        true
    }
    fn read_text(&self) -> Option<String> {
        self.reads.update(|n| n + 1);
        Some(self.clipboard.borrow().clone())
    }
}

impl Session {
    fn prepare(
        &mut self,
        text: String,
        origin: Origin,
        config: Result<(Settings, AiConfig), String>,
    ) -> u64 {
        let id = self.reserve();
        assert!(self.prepare_reserved(id, text, origin, config));
        id
    }
}

fn origin(valid: bool) -> Origin {
    Origin {
        cursor: Some(PhysicalPosition::new(-500, 100)),
        source_monitor: Arc::new(|| None),
        valid: Arc::new(move || valid),
    }
}

fn defaults() -> Settings {
    Settings {
        default_source_lang: "en".into(),
        default_target_lang: "zh".into(),
        ..Settings::default()
    }
}

fn prepare(popup: &Popup, text: &str, valid: bool) -> u64 {
    popup.lock().prepare(
        text.into(),
        origin(valid),
        Ok((defaults(), AiConfig::default())),
    )
}

fn config(server: &MockServer, model: &str) -> AiConfig {
    AiConfig {
        endpoint: server.endpoint.clone(),
        model: model.into(),
        api_key: "fictional-token".into(),
        ..AiConfig::default()
    }
}

fn request(text: &str, source: &str, target: &str) -> TranslationRequest {
    TranslationRequest {
        text: text.into(),
        source_lang: source.into(),
        target_lang: target.into(),
    }
}

fn body(server: &MockServer) -> Value {
    let request = server.captured();
    serde_json::from_str(request.split_once("\r\n\r\n").unwrap().1).unwrap()
}

#[test]
fn only_current_painted_selection_can_present_and_start_once() {
    let popup = Popup::default();
    let first = prepare(&popup, "first", true);
    assert!(popup.begin_creation());
    let second = prepare(&popup, "second", true);
    assert!(!popup.begin_creation());
    assert_eq!(popup.finish_creation(first, false), Some(second));
    assert!(popup.begin_creation());
    assert_eq!(popup.finish_creation(second, true), None);
    assert!(popup.present(first, |_| panic!("stale show")).is_none());
    let snapshot = popup.snapshot();
    assert_eq!(snapshot.preparing.unwrap().phase, Phase::Preparing);
    let attempt = popup
        .present(second, |anchor| {
            assert_eq!(anchor.cursor, Some(PhysicalPosition::new(-500, 100)));
            assert!(!popup.lock().visible());
            Ok(())
        })
        .unwrap();
    assert_eq!(attempt.request.text, "second");
    assert!(popup
        .present(second, |_| panic!("duplicate show"))
        .is_none());
    assert_eq!(popup.snapshot().selection.unwrap().phase, Phase::Loading);
    let invalid = prepare(&popup, "changed focus", false);
    assert!(!popup.lock().matches_request(second));
    assert!(popup.lock().matches_request(invalid));
    assert!(popup
        .present(invalid, |_| panic!("invalid origin"))
        .is_none());
    assert_eq!(popup.snapshot().selection.unwrap().source_text, "second");
    let failed = prepare(&popup, "cannot show", true);
    assert!(popup.present(failed, |_| Err("fixture".into())).is_none());
    let expired = prepare(&popup, "never ready", true);
    assert!(popup.lock().abandon(expired));
    assert!(popup.present(expired, |_| panic!("expired")).is_none());
    popup.clear();
    assert!(popup.snapshot().selection.is_none());
    assert!(!popup.lock().finish(second, Err(PopupError::Network)));
}

#[test]
fn delayed_configuration_cannot_block_the_next_copy_or_revive_a_dismissed_selection() {
    let popup = Popup::default();
    let first = prepare(&popup, "shown", true);
    popup.present(first, |_| Ok(())).unwrap();
    let slow = popup.lock().reserve();
    let next = popup.lock().reserve();
    let stale_config = || Ok((defaults(), AiConfig::default()));
    assert!(!popup
        .lock()
        .prepare_reserved(slow, "old".into(), origin(true), stale_config()));
    assert_eq!(popup.snapshot().dismiss_request_id, Some(next));
    assert!(!popup.lock().matches_request(first));
    assert!(popup
        .lock()
        .prepare_reserved(next, "new".into(), origin(true), stale_config()));
    assert!(!popup
        .lock()
        .prepare_reserved(next, "duplicate".into(), origin(true), stale_config()));
    let attempt = popup.present(next, |_| Ok(())).unwrap();
    assert_eq!(attempt.request.text, "new");
    let cancelled = popup.lock().reserve();
    assert!(popup.lock().abandon(cancelled));
    assert_eq!(popup.snapshot().dismiss_request_id, Some(next));
    let closing = popup.lock().reserve();
    assert!(popup.lock().matches_request(closing));
    popup.clear();
    assert!(!popup
        .lock()
        .prepare_reserved(closing, "closed".into(), origin(true), stale_config()));
    assert!(popup.snapshot().selection.is_none());
}

#[test]
fn streaming_snapshots_are_readable_but_only_the_current_complete_result_is_copyable() {
    let popup = Popup::default();
    let id = prepare(&popup, "original", true);
    popup.present(id, |_| Ok(())).unwrap();
    assert!(popup.lock().append(id, "首"));
    assert!(popup.lock().append(id, "段"));
    assert_eq!(
        popup.snapshot().selection.unwrap().phase,
        Phase::Streaming {
            translation: "首段".into()
        }
    );
    assert!(popup.lock().copied_text(id).is_none());
    let next = popup
        .lock()
        .retranslate(
            id,
            request("revised original", "en", "zh"),
            Ok(AiConfig::default()),
        )
        .unwrap()
        .unwrap();
    assert_eq!(next.request.text, "revised original");
    assert_eq!(popup.snapshot().selection.unwrap().phase, Phase::Loading);
    assert!(!popup.lock().append(id, "过时"));
    assert!(popup.lock().append(next.id, "part"));
    assert!(popup.lock().finish(next.id, Err(PopupError::Network)));
    assert!(!popup.lock().append(next.id, "late"));
    assert_eq!(
        popup.snapshot().selection.unwrap().phase,
        Phase::Error {
            error: PopupError::Network
        }
    );
    assert!(popup.lock().copied_text(next.id).is_none());
    popup.clear();
    assert!(!popup.lock().append(next.id, "hidden"));
}

#[test]
fn pin_focus_close_and_retry_are_bound_to_the_visible_attempt() {
    let popup = Popup::default();
    let id = prepare(&popup, "original", true);
    popup.present(id, |_| Ok(())).unwrap();
    assert!(popup.apply_pin(id, true, || Err("fixture".into())).is_err());
    assert!(!popup.snapshot().pinned);
    popup.apply_pin(id, true, || Ok(())).unwrap();
    assert!(!popup.lock().focus_changed(false));
    assert!(popup.lock().finish(id, Err(PopupError::Network)));
    let retry = popup
        .lock()
        .retranslate(id, request("original", "en", "zh"), Ok(AiConfig::default()))
        .unwrap()
        .unwrap();
    assert!(popup
        .lock()
        .retranslate(id, request("original", "en", "zh"), Ok(AiConfig::default()))
        .unwrap()
        .is_none());
    assert_eq!(retry.request.text, "original");
    assert!(!popup.lock().finish(id, Err(PopupError::Timeout)));
    let next = prepare(&popup, "new selection", true);
    popup.present(next, |_| Ok(())).unwrap();
    popup
        .apply_pin(retry.id, false, || panic!("stale pin"))
        .unwrap();
    assert!(popup.snapshot().pinned);
    popup.apply_pin(next, false, || Ok(())).unwrap();
    assert!(popup.lock().focus_changed(false));
    popup.clear();
    assert!(!popup.lock().focus_changed(false));
    assert!(!popup.snapshot().pinned);
    assert!(popup.lock().copied_text(next).is_none());
    assert!(popup
        .lock()
        .retranslate(
            next,
            request("new selection", "ja", "en"),
            Ok(AiConfig::default())
        )
        .unwrap()
        .is_none());
}

#[test]
fn pointer_placement_respects_work_areas_edges_scaling_and_fallback() {
    let start = PhysicalPosition::new(-1920, 40);
    let area = PhysicalSize::new(1920, 1040);
    for (x, y, expected) in [
        (-1920, 40, (-1902, 58)),
        (-1, 40, (-589, 58)),
        (-1920, 1079, (-1902, 761)),
        (-1, 1079, (-589, 761)),
    ] {
        let (point, size) =
            placement::placement(Some(PhysicalPosition::new(x, y)), start, area, 1.5);
        assert_eq!((point.x, point.y), expected);
        assert_eq!(size, PhysicalSize::new(570, 300));
    }
    assert_eq!(
        placement::placement(None, start, area, 1.0).0,
        PhysicalPosition::new(-392, 868)
    );
    let small = PhysicalSize::new(270, 150);
    assert_eq!(
        placement::placement(None, start, small, 2.0),
        (start, small)
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn changed_language_uses_original_and_current_channel_and_records_each_success_once() {
    let dir = tempfile::tempdir().unwrap();
    let store = Arc::new(Store::open(dir.path()).unwrap());
    let ai = Arc::new(AiService::new().unwrap());
    let (slow, release) =
        MockServer::split_stream(chat_event("old answer").as_bytes(), b"data: [DONE]\n\n");
    let fast = MockServer::chat("latest answer");
    let state = AppState {
        store: store.clone(),
        ai: AiService::new().unwrap(),
        desktop: Arc::default(),
    };
    let popup = &state.desktop.popup;
    store.set_target_language("zh").unwrap();
    store
        .save_configuration(&defaults(), &channel_collection(&config(&slow, "first")))
        .unwrap();
    let editor = Editor {
        selected: "selection A",
        clipboard: RefCell::new("old".into()),
        sequence: Cell::new(10),
        reads: Cell::new(0),
        cursor: Cell::new(PhysicalPosition::new(-500, 100)),
    };
    let anchor = Origin {
        cursor: Some(editor.cursor.get()),
        ..origin(true)
    };
    let (text, _) = capture_text(&editor, || true, || {}, 1).unwrap();
    assert_eq!(editor.cursor.get().x, 700);
    let id = popup.lock().prepare(text, anchor, configuration(&state));
    let old = popup
        .present(id, |origin| {
            let at = origin.cursor.unwrap();
            let monitor_left = if at.x < 0 { -1920 } else { 0 };
            let (location, _) = placement::placement(
                origin.cursor,
                PhysicalPosition::new(monitor_left, 0),
                PhysicalSize::new(1920, 1080),
                1.0,
            );
            assert_eq!(location, PhysicalPosition::new(-488, 112));
            Ok(())
        })
        .unwrap();
    popup.apply_pin(id, true, || Ok(())).unwrap();
    let old_store = store.clone();
    let old_ai = ai.clone();
    let in_flight = tokio::spawn(async move { old.run(&old_store, &old_ai, |_| {}).await });
    let sent = body(&slow);
    assert_eq!(sent["messages"][1]["content"], "selection A");
    store
        .save_configuration(&defaults(), &channel_collection(&config(&fast, "second")))
        .unwrap();
    let saved = store.configuration_snapshot().unwrap();
    assert!(!popup.lock().focus_changed(false));
    editor.ordinary_copy("unrelated B");
    let current_config = || configuration(&state).map(|(_, c)| c);
    assert!(popup
        .lock()
        .retranslate(id, request("selection A", "en", "auto"), current_config())
        .is_err());
    store.set_target_language("en").unwrap();
    let new = popup
        .lock()
        .retranslate(id, request("selection A", "zh", "en"), current_config())
        .unwrap()
        .unwrap();
    let request_id = new.id;
    assert!(popup
        .lock()
        .retranslate(id, request("selection A", "zh", "en"), current_config())
        .unwrap()
        .is_none());
    let result = new.run(&store, &ai, |_| {}).await;
    assert!(popup.lock().finish(request_id, result));
    let sent = body(&fast);
    assert_eq!(sent["model"], "second");
    assert_eq!(sent["messages"][1]["content"], "selection A");
    assert!(sent["messages"][0]["content"]
        .as_str()
        .unwrap()
        .contains("from 中文 to 英语"));
    release.send(()).unwrap();
    let previous = in_flight.await.unwrap();
    assert!(!popup.lock().finish(id, previous));
    assert_eq!(popup.lock().copied_text(request_id), Some("latest answer"));
    assert!(popup.snapshot().pinned);
    assert_eq!(*editor.clipboard.borrow(), "unrelated B");
    assert_eq!(editor.reads.get(), 1);
    assert_eq!(editor.sequence.get(), 12);
    assert_eq!(store.list_history("selection A", 10, 0).unwrap().total, 2);
    assert_eq!(
        store.configuration_snapshot().unwrap(),
        (
            Settings {
                default_target_lang: "en".into(),
                ..saved.0
            },
            saved.1
        )
    );
    let next = popup
        .lock()
        .prepare("new selection".into(), origin(true), configuration(&state));
    assert!(popup.lock().copied_text(request_id).is_none());
    assert!(!popup.lock().matches_request(request_id));
    let next_attempt = popup.present(next, |_| Ok(())).unwrap();
    assert_eq!(next_attempt.request.source_lang, "en");
    assert_eq!(next_attempt.request.target_lang, "en");
}

#[tokio::test]
async fn language_edits_and_swaps_record_once_with_either_main_auto_translate_setting() {
    for auto_translate in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let state = AppState {
            store: Arc::new(Store::open(dir.path()).unwrap()),
            ai: AiService::new().unwrap(),
            desktop: Arc::default(),
        };
        let settings = Settings {
            auto_translate,
            ..defaults()
        };
        let popup = &state.desktop.popup;
        let mut request_id = 0;
        for (index, pair) in [("en", "zh"), ("ja", "zh"), ("zh", "ja")]
            .into_iter()
            .enumerate()
        {
            let server = MockServer::chat("fixture translation");
            let channels = channel_collection(&config(&server, "fixture-model"));
            state.store.set_target_language(pair.1).unwrap();
            state
                .store
                .save_configuration(&settings, &channels)
                .unwrap();
            let snapshot = configuration(&state);
            let attempt = if index == 0 {
                let id = popup
                    .lock()
                    .prepare("original".into(), origin(true), snapshot);
                popup.present(id, |_| Ok(())).unwrap()
            } else {
                popup
                    .lock()
                    .retranslate(
                        request_id,
                        request("original", pair.0, pair.1),
                        snapshot.map(|(_, cfg)| cfg),
                    )
                    .unwrap()
                    .unwrap()
            };
            request_id = attempt.id;
            assert!(popup
                .lock()
                .retranslate(
                    request_id,
                    request("original", pair.0, pair.1),
                    Ok(AiConfig::default())
                )
                .unwrap()
                .is_none());
            let result = attempt.run(&state.store, &state.ai, |_| {}).await;
            assert!(popup.lock().finish(request_id, result));
            assert_eq!(body(&server)["messages"][1]["content"], "original");
            let history = state.store.list_history("", 10, 0).unwrap();
            assert_eq!(history.total, index + 1);
            assert!(history
                .items
                .iter()
                .any(|row| row.source_lang == pair.0 && row.target_lang == pair.1));
            assert_eq!(
                state.store.configuration_snapshot().unwrap(),
                (
                    Settings {
                        default_target_lang: pair.1.into(),
                        ..settings.clone()
                    },
                    channels
                )
            );
        }
    }
}

#[tokio::test]
async fn explicit_retranslation_sends_and_records_the_submitted_source_only_once_per_click() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    let ai = AiService::new().unwrap();
    let popup = Popup::default();
    let initial = MockServer::chat("first result");
    let mut id = popup.lock().prepare(
        "captured".into(),
        origin(true),
        Ok((defaults(), config(&initial, "first"))),
    );
    let first = popup.present(id, |_| Ok(())).unwrap();
    let first_result = first.run(&store, &ai, |_| {}).await;
    assert!(popup.lock().finish(id, first_result));
    assert_eq!(body(&initial)["messages"][1]["content"], "captured");
    let edited = "edited first line\n第二行";
    for result_text in ["edited result", "repeated result"] {
        let server = MockServer::chat(result_text);
        let cfg = config(&server, "current-model");
        let unchanged_version = popup.snapshot().version;
        assert!(popup
            .lock()
            .retranslate(id, request(" \n", "en", "zh"), Ok(cfg.clone()))
            .unwrap()
            .is_none());
        assert_eq!(popup.snapshot().version, unchanged_version);
        let attempt = popup
            .lock()
            .retranslate(id, request(edited, "en", "zh"), Ok(cfg.clone()))
            .unwrap()
            .unwrap();
        assert!(popup
            .lock()
            .retranslate(id, request(edited, "en", "zh"), Ok(cfg.clone()))
            .unwrap()
            .is_none());
        id = attempt.id;
        assert!(popup
            .lock()
            .retranslate(id, request(edited, "en", "zh"), Ok(cfg))
            .unwrap()
            .is_none());
        assert!(popup.lock().copied_text(id).is_none());
        let result = attempt.run(&store, &ai, |_| {}).await;
        assert!(popup.lock().finish(id, result));
        assert_eq!(body(&server)["messages"][1]["content"], edited);
        assert_eq!(popup.lock().copied_text(id), Some(result_text));
        assert_eq!(popup.snapshot().selection.unwrap().source_text, edited);
    }
    let records = store.list_history("", 10, 0).unwrap();
    assert_eq!(records.total, 3);
    assert_eq!(
        records
            .items
            .iter()
            .filter(|row| row.source_text == edited)
            .count(),
        2
    );
}

#[tokio::test]
async fn failures_retry_saved_direction_and_history_warning_keeps_copyable_result() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    let ai = AiService::new().unwrap();
    let popup = Popup::default();
    let id = popup
        .lock()
        .prepare("A".into(), origin(true), Err("database details".into()));
    let first = popup.present(id, |_| Ok(())).unwrap();
    let result = first.run(&store, &ai, |_| {}).await;
    assert_eq!(result.unwrap_err(), PopupError::Settings);
    popup.lock().finish(id, Err(PopupError::Settings));
    let server = MockServer::new(403, json!({"error":"upstream-private"}));
    let failed = popup
        .lock()
        .retranslate(id, request("A", "ja", "en"), Ok(config(&server, "m")))
        .unwrap()
        .unwrap();
    let failed_id = failed.id;
    let error = failed.run(&store, &ai, |_| {}).await.unwrap_err();
    assert_eq!(error, PopupError::Key);
    popup.lock().finish(failed_id, Err(error));
    assert_eq!(store.list_history("", 20, 0).unwrap().total, 0);
    let server = MockServer::chat("still readable");
    let retried = popup
        .lock()
        .retranslate(
            failed_id,
            request("A", "ja", "en"),
            Ok(config(&server, "replacement")),
        )
        .unwrap()
        .unwrap();
    assert_eq!(retried.request.source_lang, "ja");
    assert_eq!(retried.request.target_lang, "en");
    let last = retried.id;
    assert!(popup
        .lock()
        .retranslate(
            failed_id,
            request("A", "ja", "en"),
            Ok(config(&server, "replacement"))
        )
        .unwrap()
        .is_none());
    rusqlite::Connection::open(dir.path().join("upfanyi.sqlite3")).unwrap().execute_batch(
        "CREATE TRIGGER fail_history BEFORE INSERT ON translation_history BEGIN SELECT RAISE(ABORT, 'fixture'); END;"
    ).unwrap();
    let result = retried.run(&store, &ai, |_| {}).await;
    assert!(popup.lock().finish(last, result));
    assert_eq!(popup.lock().copied_text(last), Some("still readable"));
    let dto = serde_json::to_value(popup.snapshot()).unwrap();
    assert_eq!(dto["selection"]["historySaved"], false);
    let public = dto.to_string();
    for private in [
        "apiKey",
        "fictional-token",
        "endpoint",
        "systemPrompt",
        "upstream-private",
        "replacement",
    ] {
        assert!(!public.contains(private));
    }
    assert_eq!(store.list_history("", 20, 0).unwrap().total, 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn replaced_and_closed_selections_finish_history_without_reopening_the_popup() {
    let dir = tempfile::tempdir().unwrap();
    let store = Arc::new(Store::open(dir.path()).unwrap());
    let ai = Arc::new(AiService::new().unwrap());
    let popup = Popup::default();
    let (old_server, release) =
        MockServer::split_stream(chat_event("translation A").as_bytes(), b"data: [DONE]\n\n");
    let first = popup.lock().prepare(
        "A".into(),
        origin(true),
        Ok((defaults(), config(&old_server, "first"))),
    );
    let old = popup.present(first, |_| Ok(())).unwrap();
    let (old_store, old_ai) = (store.clone(), ai.clone());
    let in_flight = tokio::spawn(async move { old.run(&old_store, &old_ai, |_| {}).await });
    assert_eq!(body(&old_server)["messages"][1]["content"], "A");
    let server = MockServer::chat("translation B");
    let second = popup.lock().prepare(
        "B".into(),
        origin(true),
        Ok((defaults(), config(&server, "second"))),
    );
    let current = popup.present(second, |_| Ok(())).unwrap();
    assert!(popup
        .present(second, |_| panic!("reloaded window"))
        .is_none());
    let result = current.run(&store, &ai, |_| {}).await;
    assert!(popup.lock().finish(second, result));
    assert_eq!(body(&server)["messages"][1]["content"], "B");
    assert!(!popup.lock().finish(first, Err(PopupError::Network)));
    assert_eq!(popup.lock().copied_text(second), Some("translation B"));
    popup.clear();
    release.send(()).unwrap();
    let previous = in_flight.await.unwrap();
    assert!(!popup.lock().finish(first, previous));
    let hidden = popup.snapshot();
    assert!(hidden.selection.is_none() && hidden.preparing.is_none() && !hidden.pinned);
    let records = store.list_history("", 10, 0).unwrap();
    assert_eq!(records.total, 2);
    for (source, translation) in [("A", "translation A"), ("B", "translation B")] {
        assert!(records
            .items
            .iter()
            .any(|row| row.source_text == source && row.translated_text == translation));
    }
}

#[test]
fn generated_command_permissions_are_scoped_to_each_window() {
    let mut context: tauri::Context<tauri::Wry> = tauri::generate_context!();
    let authority = context.runtime_authority_mut();
    let can = |window: &str, cmd: &str| {
        authority
            .resolve_access(cmd, window, window, &tauri::ipc::Origin::Local)
            .is_some()
    };
    for command in [
        "bootstrap",
        "translate",
        "save_settings",
        "list_history",
        "delete_history",
        "clear_history",
        "list_favorites",
        "save_favorite",
        "delete_favorite",
        "list_ai_models",
        "test_ai_connection",
        "export_favorites",
        "copy_text",
        "open_official_website",
    ] {
        assert!(can("main", command), "{command}");
        assert!(!can(LABEL, command), "{command}");
    }
    for command in [
        "popup_snapshot",
        "popup_ready",
        "popup_retranslate",
        "popup_pin",
        "popup_copy",
        "popup_start_drag",
        "popup_dismiss",
    ] {
        assert!(can(LABEL, command), "{command}");
        assert!(!can("main", command), "{command}");
    }
    for window in [LABEL, "main"] {
        assert!(can(window, "get_target_language"));
        assert!(can(window, "set_target_language"));
        assert!(can(window, "plugin:event|listen"));
        assert!(can(window, "plugin:event|unlisten"));
        assert!(!can(window, "plugin:event|emit"));
        assert!(!can(window, "plugin:window|start_dragging"));
    }
}
