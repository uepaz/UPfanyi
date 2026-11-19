use super::*;
use crate::test_support::{channel_collection, MockServer};
use serde_json::json;
use std::cell::{Cell, RefCell};

#[derive(Default)]
struct DesktopFixture {
    shortcut_settings: RefCell<Settings>,
    on_top: Cell<bool>,
    shortcut_calls: Cell<usize>,
    top_calls: Cell<usize>,
    fail_first_top: bool,
    fail_shortcut_rollback: bool,
}

impl SettingsEffects for DesktopFixture {
    fn replace_shortcuts(&self, settings: &Settings) -> Result<(), String> {
        let call = self.shortcut_calls.get() + 1;
        self.shortcut_calls.set(call);
        if self.fail_shortcut_rollback && call == 2 {
            return Err("模拟恢复快捷键失败".into());
        }
        self.shortcut_settings.replace(settings.clone());
        Ok(())
    }

    fn set_always_on_top(&self, enabled: bool) -> Result<(), String> {
        let call = self.top_calls.get() + 1;
        self.top_calls.set(call);
        if self.fail_first_top && call == 1 {
            return Err("模拟窗口操作失败".into());
        }
        self.on_top.set(enabled);
        Ok(())
    }
}

fn changed_settings() -> Settings {
    Settings {
        always_on_top: true,
        locale: "en_US".into(),
        hotkey_show_window: "Ctrl+Alt+T".into(),
        ..Settings::default()
    }
}

#[test]
fn successful_save_applies_desktop_and_persists_configuration() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    let effects = DesktopFixture::default();
    let settings = changed_settings();
    let config = store.ai_channels().unwrap();
    save_settings(&store, &effects, &settings, &config).unwrap();
    assert_eq!(
        store.configuration_snapshot().unwrap(),
        (settings.clone(), config)
    );
    assert_eq!(*effects.shortcut_settings.borrow(), settings);
    assert!(effects.on_top.get());
}

#[test]
fn sql_failure_restores_both_desktop_effects_and_database_transaction() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    let original = store.configuration_snapshot().unwrap();
    rusqlite::Connection::open(dir.path().join("upfanyi.sqlite3"))
        .unwrap()
        .execute_batch("CREATE TRIGGER fail_config BEFORE INSERT ON ai_configuration BEGIN SELECT RAISE(ABORT, 'fixture SQL failure'); END;")
        .unwrap();
    let effects = DesktopFixture::default();
    let pending = channel_collection(&AiConfig {
        model: "uncommitted-model".into(),
        ..AiConfig::default()
    });
    let result = save_settings(&store, &effects, &changed_settings(), &pending);
    assert!(result.unwrap_err().contains("fixture SQL failure"));
    assert_eq!(store.configuration_snapshot().unwrap(), original);
    assert_eq!(*effects.shortcut_settings.borrow(), original.0);
    assert!(!effects.on_top.get());
    assert_eq!(effects.shortcut_calls.get(), 2);
    assert_eq!(effects.top_calls.get(), 2);
}

#[test]
fn native_failure_keeps_saved_settings_and_restores_shortcuts() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    let original = store.configuration_snapshot().unwrap();
    let effects = DesktopFixture {
        fail_first_top: true,
        ..Default::default()
    };
    let pending = channel_collection(&AiConfig {
        api_key: "uncommitted-fictional-key".into(),
        ..AiConfig::default()
    });
    assert!(save_settings(&store, &effects, &changed_settings(), &pending).is_err());
    assert_eq!(store.configuration_snapshot().unwrap(), original);
    assert_eq!(*effects.shortcut_settings.borrow(), original.0);
    assert!(!effects.on_top.get());
}

#[test]
fn rollback_failure_is_visible_and_does_not_skip_other_compensation() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    let effects = DesktopFixture {
        fail_first_top: true,
        fail_shortcut_rollback: true,
        ..Default::default()
    };
    let error = save_settings(
        &store,
        &effects,
        &changed_settings(),
        &store.ai_channels().unwrap(),
    )
    .unwrap_err();
    assert!(error.contains("桌面状态恢复失败"));
    assert_eq!(effects.top_calls.get(), 2);
    assert_eq!(store.settings().unwrap(), Settings::default());
}

#[test]
fn invalid_settings_are_rejected_before_desktop_changes() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    let effects = DesktopFixture::default();
    let original = store.configuration_snapshot().unwrap();
    for settings in [
        Settings {
            locale: "invalid".into(),
            ..Settings::default()
        },
        Settings {
            hotkey_show_window: "ctrl shift T".into(),
            ..Settings::default()
        },
        Settings {
            hotkey_show_window: "Ctrl+Shift+C".into(),
            ..Settings::default()
        },
    ] {
        assert!(save_settings(&store, &effects, &settings, &original.1).is_err());
        assert_eq!(store.configuration_snapshot().unwrap(), original);
    }
    assert_eq!(effects.shortcut_calls.get(), 0);
    assert_eq!(effects.top_calls.get(), 0);
}

#[test]
fn invalid_channel_collections_are_rejected_before_any_desktop_effect() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    let original = store.configuration_snapshot().unwrap();
    let effects = DesktopFixture::default();
    let valid = channel_collection(&AiConfig::default());
    let mut cases = Vec::new();
    let mut value = valid.clone();
    value.channels[0].config.endpoint = "https://forged.fixture.invalid".into();
    cases.push(value);
    let mut value = valid.clone();
    value.channels[0].name = "forged".into();
    cases.push(value);
    let mut value = valid.clone();
    value.channels.remove(0);
    cases.push(value);
    let mut value = valid.clone();
    value.active_id = "add-channel".into();
    cases.push(value);
    let mut value = valid.clone();
    value.channels[1].id = "custom".into();
    cases.push(value);
    let mut value = valid.clone();
    value.channels.push(value.channels[1].clone());
    cases.push(value);
    let mut value = valid.clone();
    value.channels[1].name = "UPfanyi official AI".into();
    cases.push(value);
    let mut value = valid;
    value.channels[1].config.endpoint = "https://user:confidential-fixture@fixture.invalid".into();
    cases.push(value);
    for value in cases {
        let error = save_settings(&store, &effects, &changed_settings(), &value).unwrap_err();
        assert!(!error.contains("confidential-fixture"));
        assert_eq!(store.configuration_snapshot().unwrap(), original);
    }
    assert_eq!(effects.shortcut_calls.get(), 0);
    assert_eq!(effects.top_calls.get(), 0);
}

#[tokio::test]
async fn formal_translation_changes_channel_only_after_the_collection_is_saved() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    let first = MockServer::new(
        200,
        json!({"choices":[{"message":{"content":"first result"}}]}),
    );
    let second = MockServer::new(
        200,
        json!({"choices":[{"message":{"content":"second result"}}]}),
    );
    let mut saved = channel_collection(&AiConfig {
        protocol: AiProtocol::Openai,
        endpoint: first.endpoint.clone(),
        model: "model-first".into(),
        ..AiConfig::default()
    });
    saved.channels.push(AiChannel {
        id: "22222222-2222-4222-8222-222222222222".into(),
        name: "Second service".into(),
        config: AiConfig {
            protocol: AiProtocol::Openai,
            endpoint: second.endpoint.clone(),
            model: "model-second".into(),
            ..AiConfig::default()
        },
    });
    store
        .save_configuration(&Settings::default(), &saved)
        .unwrap();
    let mut draft = saved.clone();
    draft.active_id = draft.channels[2].id.clone();
    let ai = ai::AiService::new().unwrap();
    let first_result = translate(&store, &ai, &request()).await.unwrap();
    assert_eq!(first_result.model, "model-first");
    assert_eq!(first_result.translated_text, "first result");
    assert!(first.captured().contains("model-first"));
    assert_eq!(store.ai_channels().unwrap(), saved);
    save_settings(
        &store,
        &DesktopFixture::default(),
        &Settings::default(),
        &draft,
    )
    .unwrap();
    let second_result = translate(&store, &ai, &request()).await.unwrap();
    assert_eq!(second_result.model, "model-second");
    assert_eq!(second_result.translated_text, "second result");
    assert!(second.captured().contains("model-second"));
    assert_eq!(store.ai_channels().unwrap(), draft);
    assert_eq!(store.list_history("", 20, 0).unwrap().total, 2);
}

#[test]
fn saved_language_validation_matches_translation_requests() {
    for language in [
        "AUTO",
        "aUto",
        "zh!",
        "zh CN",
        "",
        "language-code-that-is-over-thirty-two-characters",
    ] {
        let settings = Settings {
            default_target_lang: language.into(),
            ..Settings::default()
        };
        assert!(
            validate_settings(&settings).is_err(),
            "invalid target accepted: {language}"
        );
    }
    let settings = Settings {
        default_source_lang: "zh!".into(),
        ..Settings::default()
    };
    assert!(validate_settings(&settings).is_err());
    for language in ["zh-CN", "pt_BR", "en"] {
        let settings = Settings {
            default_target_lang: language.into(),
            ..Settings::default()
        };
        assert!(validate_settings(&settings).is_ok());
    }
}

#[test]
fn export_writes_utf8_and_reports_real_filesystem_failure() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    store
        .save_favorite("hello", "你好", Some("常用词😀"))
        .unwrap();
    let content = store.export_favorites().unwrap();
    let output = dir.path().join("favorites.txt");
    assert_eq!(
        write_export(&output, &content).unwrap(),
        output.to_string_lossy()
    );
    assert_eq!(std::fs::read_to_string(&output).unwrap(), content);
    write_export(&output, "更新😀").unwrap();
    assert_eq!(std::fs::read_to_string(&output).unwrap(), "更新😀");
    assert!(content.contains("常用词😀"));
    assert!(write_export(dir.path(), &content)
        .unwrap_err()
        .starts_with("导出失败："));
    assert!(std::fs::read_dir(dir.path()).unwrap().all(|entry| !entry
        .unwrap()
        .file_name()
        .to_string_lossy()
        .starts_with(".upfanyi-export-")));
}

#[test]
#[cfg(windows)]
fn export_publish_failure_preserves_existing_read_only_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("protected.txt");
    std::fs::write(&path, "已有导出").unwrap();
    let original_permissions = std::fs::metadata(&path).unwrap().permissions();
    let mut read_only = original_permissions.clone();
    read_only.set_readonly(true);
    std::fs::set_permissions(&path, read_only).unwrap();
    let result = write_export(&path, "新的内容");
    std::fs::set_permissions(&path, original_permissions).unwrap();
    assert!(result.is_err());
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "已有导出");
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
}

fn configure_local_ai(store: &Store, server: &MockServer) -> AiConfig {
    let config = AiConfig {
        protocol: AiProtocol::Openai,
        endpoint: server.endpoint.clone(),
        model: "fixture-model".into(),
        ..AiConfig::default()
    };
    store
        .save_configuration(&Settings::default(), &channel_collection(&config))
        .unwrap();
    config
}

fn request() -> TranslationRequest {
    TranslationRequest {
        text: "Hello".into(),
        source_lang: "en".into(),
        target_lang: "zh".into(),
    }
}

#[tokio::test]
async fn translation_uses_saved_configuration_http_response_and_real_history() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    let server = MockServer::new(200, json!({"choices":[{"message":{"content":" 你好 "}}]}));
    configure_local_ai(&store, &server);
    let ai = ai::AiService::new().unwrap();
    let result = translate(&store, &ai, &request()).await.unwrap();
    assert_eq!(result.translated_text, "你好");
    assert_eq!(result.model, "fixture-model");
    assert!(result.warning.is_none());
    let history = store.list_history("Hello", 20, 0).unwrap();
    assert_eq!(history.total, 1);
    assert_eq!(history.items[0].translated_text, result.translated_text);
    assert_eq!(history.items[0].model, result.model);
    let captured = server.captured();
    assert!(captured.contains("fixture-model"));
    assert!(captured.contains("Hello"));
}

#[tokio::test]
async fn network_failure_creates_no_history() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    let server = MockServer::new(500, json!({"error":"fixture error"}));
    configure_local_ai(&store, &server);
    let ai = ai::AiService::new().unwrap();
    assert!(translate(&store, &ai, &request()).await.is_err());
    assert_eq!(store.list_history("", 20, 0).unwrap().total, 0);
    assert!(server.captured().contains("Hello"));
}

#[tokio::test]
async fn connection_test_keeps_saved_configuration_and_history_unchanged() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    let server = MockServer::new(200, json!({"choices":[{"message":{"content":"fixture"}}]}));
    let mut draft = configure_local_ai(&store, &server);
    let original = store.configuration_snapshot().unwrap();
    draft.model = "unsaved-model".into();
    let ai = ai::AiService::new().unwrap();
    assert_eq!(ai.test(&draft).await.unwrap().translated_text, "fixture");
    assert_eq!(store.configuration_snapshot().unwrap(), original);
    assert_eq!(store.list_history("", 20, 0).unwrap().total, 0);
    assert!(server.captured().contains("unsaved-model"));
}

#[tokio::test]
async fn responses_translation_persists_actual_result_and_model() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    let server = MockServer::new(
        200,
        json!({
            "status":"completed", "output":[
                {"type":"reasoning", "summary":[]},
                {"type":"message", "role":"assistant", "status":"completed", "content":[
                    {"type":"output_text", "text":"你好"},
                    {"type":"output_text", "text":"世界"}
                ]}
            ]
        }),
    );
    let mut service_address = url::Url::parse(&server.endpoint).unwrap();
    service_address.set_path("/");
    let config = AiConfig {
        protocol: AiProtocol::OpenaiResponses,
        endpoint: service_address.into(),
        model: "responses-fixture".into(),
        ..AiConfig::default()
    };
    save_settings(
        &store,
        &DesktopFixture::default(),
        &Settings::default(),
        &channel_collection(&config),
    )
    .unwrap();
    let result = translate(&store, &ai::AiService::new().unwrap(), &request())
        .await
        .unwrap();
    assert_eq!(result.translated_text, "你好世界");
    assert_eq!(result.model, "responses-fixture");
    let history = store.list_history("Hello", 20, 0).unwrap();
    assert_eq!(history.total, 1);
    assert_eq!(history.items[0].model, result.model);
    assert_eq!(history.items[0].translated_text, result.translated_text);
    let http = server.captured();
    assert!(http.starts_with("POST /v1/responses?fixture=yes HTTP/1.1\r\n"));
    let body: serde_json::Value =
        serde_json::from_str(http.split_once("\r\n\r\n").unwrap().1).unwrap();
    assert_eq!(body["store"], false);
    assert_eq!(body["input"], "Hello");
}

#[tokio::test]
async fn incomplete_or_refused_responses_never_create_history() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    let ai = ai::AiService::new().unwrap();
    for response in [
        json!({"status":"incomplete", "output":[{"type":"message","role":"assistant","content":[{"type":"output_text","text":"partial-private"}]}]}),
        json!({"status":"completed", "output":[{"type":"message","role":"assistant","status":"completed","content":[{"type":"refusal","refusal":"refusal-private"}]}]}),
    ] {
        let server = MockServer::new(200, response);
        let config = AiConfig {
            protocol: AiProtocol::OpenaiResponses,
            endpoint: server.endpoint.clone(),
            model: "responses-fixture".into(),
            ..AiConfig::default()
        };
        store
            .save_configuration(&Settings::default(), &channel_collection(&config))
            .unwrap();
        let error = translate(&store, &ai, &request()).await.unwrap_err();
        assert!(!error.contains("private"));
        assert_eq!(store.list_history("", 20, 0).unwrap().total, 0);
        server.captured();
    }
}

#[tokio::test]
async fn model_discovery_uses_unsaved_draft_without_mutating_store() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    let original = store.configuration_snapshot().unwrap();
    let before = std::fs::read(dir.path().join("upfanyi.sqlite3")).unwrap();
    let server = MockServer::new(200, json!({"data":[{"id":"available-fixture"}]}));
    let mut endpoint = url::Url::parse(&server.endpoint).unwrap();
    endpoint.set_path("/gateway/v1");
    let draft = AiConfig {
        protocol: AiProtocol::OpenaiResponses,
        endpoint: endpoint.into(),
        api_key: "draft-fictional-key".into(),
        model: String::new(),
        system_prompt: String::new(),
    };
    let result = ai::AiService::new()
        .unwrap()
        .list_models(&draft)
        .await
        .unwrap();
    assert_eq!(result, ["available-fixture"]);
    assert_eq!(store.configuration_snapshot().unwrap(), original);
    assert_eq!(store.list_history("", 20, 0).unwrap().total, 0);
    assert_eq!(
        std::fs::read(dir.path().join("upfanyi.sqlite3")).unwrap(),
        before
    );
    let http = server.captured();
    assert!(http.starts_with("GET /gateway/v1/models?fixture=yes HTTP/1.1\r\n"));
    assert!(http.contains("Bearer draft-fictional-key"));
}
