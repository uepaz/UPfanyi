use super::schema::{APPLICATION_ID, SCHEMA_VERSION};
use super::*;
use crate::model::AiProtocol;
use crate::test_support::channel_collection;
use serde_json::{json, Value};
use tempfile::{tempdir, TempDir};

fn test_store() -> (TempDir, Store) {
    let directory = tempdir().unwrap();
    let store = Store::open(directory.path()).unwrap();
    (directory, store)
}

#[test]
fn shared_target_survives_stale_settings_saves_and_restart() {
    let (directory, store) = test_store();
    let (mut draft, channels) = store.configuration_snapshot().unwrap();
    let initial = store.target_language().unwrap();
    let selected = store.set_target_language("ja").unwrap();
    assert!(selected.revision > initial.revision);
    assert_eq!(store.set_target_language("ja").unwrap(), selected);
    assert_eq!(store.ai_channels().unwrap(), channels);
    assert_eq!(
        store.settings().unwrap(),
        Settings {
            default_target_lang: "ja".into(),
            ..draft.clone()
        }
    );
    draft.locale = "en_US".into();
    store.save_configuration(&draft, &channels).unwrap();
    assert_eq!(store.target_language().unwrap(), selected);
    assert!(store.set_target_language("auto").is_err());
    store
        .lock()
        .unwrap()
        .execute_batch("PRAGMA query_only = ON")
        .unwrap();
    assert!(store.set_target_language("de").is_err());
    assert_eq!(store.target_language().unwrap(), selected);
    drop(store);
    let reopened = Store::open(directory.path()).unwrap();
    assert_eq!(reopened.target_language().unwrap().language, "ja");
    assert_eq!(reopened.settings().unwrap().locale, "en_US");
    assert_eq!(reopened.list_history("", 10, 0).unwrap().total, 0);
}

fn translation(text: &str, time: &str) -> Translation {
    Translation {
        source_text: text.into(),
        translated_text: format!("译文 {text}"),
        source_lang: "en".into(),
        target_lang: "zh".into(),
        model: "fixture-model".into(),
        create_time: time.into(),
        warning: None,
    }
}

fn row_json(store: &Store) -> String {
    store
        .lock()
        .unwrap()
        .query_row(
            "SELECT config_json FROM ai_configuration WHERE id = 1",
            [],
            |row| row.get(0),
        )
        .unwrap()
}

fn assert_rejected_without_changes(directory: &Path) {
    let path = directory.join(DATABASE_FILE);
    let before = fs::read(&path).unwrap();
    assert!(Store::open(directory).is_err());
    assert_eq!(fs::read(&path).unwrap(), before);
    assert_eq!(fs::read_dir(directory).unwrap().count(), 1);
}

#[test]
fn creates_native_tables_format_markers_and_explicit_defaults() {
    let (_directory, store) = test_store();
    assert_eq!(store.settings().unwrap(), Settings::default());
    let config = store.ai_config().unwrap();
    assert_eq!(config, AiConfig::default());
    assert_eq!(store.ai_channels().unwrap(), AiChannels::default());
    assert_eq!(config.protocol, AiProtocol::Openai);
    assert_eq!(config.endpoint, "https://uptoken.top");
    assert!(config.api_key.is_empty());
    assert!(config.model.is_empty());
    assert!(!config.system_prompt.is_empty());
    assert!(store.list_history("", 20, 0).unwrap().items.is_empty());
    assert!(store.list_favorites("", 20, 0).unwrap().items.is_empty());
    let connection = store.lock().unwrap();
    assert_eq!(
        connection
            .pragma_query_value(None, "application_id", |row| row.get::<_, i64>(0))
            .unwrap(),
        APPLICATION_ID
    );
    assert_eq!(
        connection
            .pragma_query_value(None, "user_version", |row| row.get::<_, i64>(0))
            .unwrap(),
        SCHEMA_VERSION
    );
    let mut tables = connection.prepare("SELECT name FROM sqlite_schema WHERE type = 'table' AND name NOT LIKE 'sqlite_%' ORDER BY name").unwrap();
    let names = tables
        .query_map([], |row| row.get::<_, String>(0))
        .unwrap()
        .collect::<rusqlite::Result<Vec<_>>>()
        .unwrap();
    assert_eq!(
        names,
        [
            "ai_configuration",
            "app_settings",
            "favorite_entries",
            "translation_history"
        ]
    );
    let mut columns = connection
        .prepare("PRAGMA table_info(ai_configuration)")
        .unwrap();
    let names = columns
        .query_map([], |row| row.get::<_, String>(1))
        .unwrap()
        .collect::<rusqlite::Result<Vec<_>>>()
        .unwrap();
    assert_eq!(names, ["id", "config_json"]);
    let mut columns = connection
        .prepare("PRAGMA table_info(translation_history)")
        .unwrap();
    let names = columns
        .query_map([], |row| row.get::<_, String>(1))
        .unwrap()
        .collect::<rusqlite::Result<Vec<_>>>()
        .unwrap();
    assert!(names.contains(&"model".into()));
    assert!(!names.contains(&"engine_id".into()));
}

#[test]
fn first_start_official_configuration_survives_restart_without_rewriting_files() {
    let (directory, store) = test_store();
    let initial = store.ai_config().unwrap();
    assert_eq!(initial.protocol, AiProtocol::Openai);
    assert_eq!(initial.endpoint, "https://uptoken.top");
    assert!(initial.api_key.is_empty());
    assert!(initial.model.is_empty());
    assert_eq!(
        serde_json::to_value(&initial)
            .unwrap()
            .as_object()
            .unwrap()
            .len(),
        5
    );
    drop(store);
    let path = directory.path().join(DATABASE_FILE);
    let before = fs::read(&path).unwrap();
    let reopened = Store::open(directory.path()).unwrap();
    assert_eq!(reopened.ai_config().unwrap(), initial);
    drop(reopened);
    assert_eq!(fs::read(&path).unwrap(), before);
}

#[test]
fn saved_user_channel_is_not_replaced_by_the_official_initial_default() {
    let (directory, store) = test_store();
    let settings = store.settings().unwrap();
    let saved = AiConfig {
        endpoint: "http://localhost:11434".into(),
        model: "existing-fixture-model".into(),
        system_prompt: "existing fixture prompt".into(),
        ..AiConfig::default()
    };
    let mut channels = channel_collection(&saved);
    channels.channels[1].name = "ollama".into();
    store.save_configuration(&settings, &channels).unwrap();
    store
        .save_history(&translation("saved", "2026-10-04T00:00:00Z"))
        .unwrap();
    store
        .save_favorite("saved", "已保存", Some("note"))
        .unwrap();
    drop(store);
    let path = directory.path().join(DATABASE_FILE);
    let before = fs::read(&path).unwrap();
    let reopened = Store::open(directory.path()).unwrap();
    assert_eq!(reopened.ai_config().unwrap(), saved);
    assert_eq!(reopened.ai_channels().unwrap(), channels);
    assert_eq!(reopened.settings().unwrap(), settings);
    assert_eq!(reopened.list_history("", 20, 0).unwrap().total, 1);
    assert_eq!(reopened.list_favorites("", 20, 0).unwrap().total, 1);
    drop(reopened);
    assert_eq!(fs::read(&path).unwrap(), before);
}

#[test]
fn service_domain_configuration_survives_restart_without_rewriting_files() {
    let (directory, store) = test_store();
    let settings = store.settings().unwrap();
    let config = AiConfig {
        endpoint: "relay.example.test".into(),
        model: "domain-fixture".into(),
        ..AiConfig::default()
    };
    store
        .save_configuration(&settings, &channel_collection(&config))
        .unwrap();
    drop(store);
    let path = directory.path().join(DATABASE_FILE);
    let before = fs::read(&path).unwrap();
    let reopened = Store::open(directory.path()).unwrap();
    assert_eq!(reopened.ai_config().unwrap(), config);
    assert_eq!(reopened.settings().unwrap(), settings);
    drop(reopened);
    assert_eq!(fs::read(&path).unwrap(), before);
}

#[test]
fn restart_preserves_native_records_settings_and_file_bytes() {
    let (directory, store) = test_store();
    let settings = Settings {
        locale: "en_US".into(),
        clipboard_monitor: true,
        clipboard_min_length: 7,
        ..Settings::default()
    };
    let config = AiConfig {
        protocol: AiProtocol::Openai,
        endpoint: "http://localhost:11434".into(),
        api_key: "fictional-restart-key".into(),
        model: "fixture-model".into(),
        system_prompt: "翻译为{target_lang}".into(),
    };
    store
        .save_configuration(&settings, &channel_collection(&config))
        .unwrap();
    store
        .save_history(&translation("saved", "2026-10-03T14:00:00+08:00"))
        .unwrap();
    store
        .save_favorite("saved", "已保存", Some("note"))
        .unwrap();
    drop(store);
    let before = fs::read(directory.path().join(DATABASE_FILE)).unwrap();
    let reopened = Store::open(directory.path()).unwrap();
    assert_eq!(reopened.settings().unwrap(), settings);
    assert_eq!(reopened.ai_config().unwrap(), config);
    let history = reopened.list_history("", 20, 0).unwrap();
    assert_eq!(history.total, 1);
    assert_eq!(history.items[0].model, "fixture-model");
    assert_eq!(reopened.list_favorites("", 20, 0).unwrap().total, 1);
    drop(reopened);
    assert_eq!(
        fs::read(directory.path().join(DATABASE_FILE)).unwrap(),
        before
    );
}

#[test]
fn previous_data_v2_v1_and_other_old_locations_are_not_read_or_changed() {
    let directory = tempdir().unwrap();
    let old_java = directory.path().join(".upfanyi");
    fs::create_dir(&old_java).unwrap();
    let old_database = old_java.join("upfanyi.db");
    let old_properties = old_java.join("config.properties");
    let old_tauri = directory.path().join("upfanyi.db");
    let old_v1 = directory.path().join("data-v1");
    fs::create_dir(&old_v1).unwrap();
    let old_v1_database = old_v1.join(DATABASE_FILE);
    fs::write(&old_database, b"invalid old Java database").unwrap();
    fs::write(&old_properties, b"bad=\\uXXXX").unwrap();
    fs::write(&old_tauri, b"invalid prior Tauri database").unwrap();
    fs::write(&old_v1_database, b"invalid data-v1 database").unwrap();
    let old_v2 = directory.path().join("data-v2");
    fs::create_dir(&old_v2).unwrap();
    let old_v2_database = old_v2.join(DATABASE_FILE);
    fs::write(&old_v2_database, b"not even a valid v2 SQLite file").unwrap();
    let store = Store::open(&directory.path().join("data-v3")).unwrap();
    assert_eq!(store.settings().unwrap(), Settings::default());
    assert_eq!(store.list_history("", 20, 0).unwrap().total, 0);
    assert_eq!(
        fs::read(old_database).unwrap(),
        b"invalid old Java database"
    );
    assert_eq!(fs::read(old_properties).unwrap(), b"bad=\\uXXXX");
    assert_eq!(
        fs::read(old_tauri).unwrap(),
        b"invalid prior Tauri database"
    );
    assert_eq!(
        fs::read(old_v1_database).unwrap(),
        b"invalid data-v1 database"
    );
    assert_eq!(
        fs::read(old_v2_database).unwrap(),
        b"not even a valid v2 SQLite file"
    );
    assert_eq!(store.ai_channels().unwrap(), AiChannels::default());
}

#[test]
fn native_channel_records_survive_responses_selection() {
    for protocol in ["openai", "openai-responses"] {
        let (directory, store) = test_store();
        let settings = Settings {
            locale: "en_US".into(),
            ..Settings::default()
        };
        // 每渠道仍使用完整五字段配置，但保存必须通过新集合契约。
        let original: AiConfig = serde_json::from_value(json!({
            "protocol": protocol,
            "endpoint": "https://fixture.example/chat/completions",
            "apiKey": "fictional-existing-key",
            "model": "existing-model",
            "systemPrompt": "翻译为{target_lang}"
        }))
        .unwrap();
        store
            .save_configuration(&settings, &channel_collection(&original))
            .unwrap();
        store
            .save_history(&translation("existing", "2026-10-04T01:00:00Z"))
            .unwrap();
        let favorite = store
            .save_favorite("existing", "已有", Some("保留备注"))
            .unwrap();
        drop(store);
        let path = directory.path().join(DATABASE_FILE);
        let bytes = fs::read(&path).unwrap();
        let store = Store::open(directory.path()).unwrap();
        assert_eq!(
            store.configuration_snapshot().unwrap(),
            (settings.clone(), channel_collection(&original))
        );
        assert_eq!(fs::read(&path).unwrap(), bytes);

        let responses = AiConfig {
            protocol: AiProtocol::OpenaiResponses,
            endpoint: "https://fixture.example/responses".into(),
            ..original
        };
        store
            .save_configuration(&settings, &channel_collection(&responses))
            .unwrap();
        assert_eq!(
            serde_json::from_str::<Value>(&row_json(&store)).unwrap()["channels"][1]["config"]
                .as_object()
                .unwrap()
                .len(),
            5
        );
        drop(store);
        let bytes = fs::read(&path).unwrap();
        let reopened = Store::open(directory.path()).unwrap();
        assert_eq!(
            reopened.configuration_snapshot().unwrap(),
            (settings, channel_collection(&responses))
        );
        assert_eq!(reopened.list_history("", 20, 0).unwrap().total, 1);
        let favorites = reopened.list_favorites("", 20, 0).unwrap();
        assert_eq!(favorites.total, 1);
        assert_eq!(favorites.items[0].id, favorite.id);
        assert_eq!(favorites.items[0].note.as_deref(), Some("保留备注"));
        assert_eq!(fs::read(&path).unwrap(), bytes);
    }
}

#[test]
fn unsupported_format_versions_and_corrupt_files_are_rejected_without_writes() {
    for (application_id, version) in [
        (0, 0),
        (42, SCHEMA_VERSION),
        (APPLICATION_ID, 1),
        (APPLICATION_ID, 2),
        (APPLICATION_ID, SCHEMA_VERSION + 1),
    ] {
        let (directory, store) = test_store();
        drop(store);
        let connection = Connection::open(directory.path().join(DATABASE_FILE)).unwrap();
        connection
            .pragma_update(None, "application_id", application_id)
            .unwrap();
        connection
            .pragma_update(None, "user_version", version)
            .unwrap();
        connection.close().unwrap();
        assert_rejected_without_changes(directory.path());
    }
    let directory = tempdir().unwrap();
    fs::write(
        directory.path().join(DATABASE_FILE),
        b"not a SQLite database",
    )
    .unwrap();
    assert_rejected_without_changes(directory.path());
}

#[test]
fn old_tables_cannot_be_accepted_by_spoofing_native_markers() {
    let directory = tempdir().unwrap();
    let connection = Connection::open(directory.path().join(DATABASE_FILE)).unwrap();
    connection.execute_batch("CREATE TABLE history (id INTEGER PRIMARY KEY, create_time TEXT); CREATE TABLE api_config (api_key TEXT, extra_config TEXT);").unwrap();
    connection
        .pragma_update(None, "application_id", APPLICATION_ID)
        .unwrap();
    connection
        .pragma_update(None, "user_version", SCHEMA_VERSION)
        .unwrap();
    connection.close().unwrap();
    assert_rejected_without_changes(directory.path());
}

#[test]
fn unexpected_columns_missing_indexes_and_extra_objects_are_rejected() {
    for change in [
        "ALTER TABLE ai_configuration ADD COLUMN api_key TEXT",
        "DROP INDEX idx_translation_history_time",
        "CREATE TABLE unexpected (id INTEGER)",
        "CREATE TABLE sqliteXunexpected (id INTEGER)",
        "CREATE TRIGGER unexpected BEFORE UPDATE ON app_settings BEGIN SELECT RAISE(FAIL, 'fixture'); END",
    ] {
        let (directory, store) = test_store();
        drop(store);
        let connection = Connection::open(directory.path().join(DATABASE_FILE)).unwrap();
        connection.execute_batch(change).unwrap();
        connection.close().unwrap();
        assert_rejected_without_changes(directory.path());
    }
}

#[test]
fn history_uses_utc_milliseconds_with_stable_ids_across_offsets() {
    let (_directory, store) = test_store();
    store
        .save_history(&translation("earlier", "2026-10-03T13:00:00+08:00"))
        .unwrap();
    store
        .save_history(&translation("later", "2026-10-03T06:00:00Z"))
        .unwrap();
    let fraction_id = store
        .save_history(&translation("fraction", "2026-10-03T01:00:00.123456-05:00"))
        .unwrap();
    let tie_id = store
        .save_history(&translation("tie", "2026-10-03T14:00:00.123999+08:00"))
        .unwrap();
    let page = store.list_history("", 2, 1).unwrap();
    assert_eq!(page.total, 4);
    assert_eq!(page.items[0].id, fraction_id);
    assert_eq!(page.items[0].create_time, "2026-10-03T06:00:00.123Z");
    assert_eq!(page.items[1].source_text, "later");
    assert_eq!(store.list_history("", 1, 0).unwrap().items[0].id, tie_id);
    let kind: String = store
        .lock()
        .unwrap()
        .query_row(
            "SELECT typeof(created_at_ms) FROM translation_history WHERE id = ?1",
            [tie_id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(kind, "integer");
}

#[test]
fn daylight_saving_offsets_are_actual_instants_without_local_time_guessing() {
    let (_directory, store) = test_store();
    store
        .save_history(&translation("older", "2026-11-01T01:30:00-04:00"))
        .unwrap();
    store
        .save_history(&translation("newer", "2026-11-01T01:15:00-05:00"))
        .unwrap();
    let page = store.list_history("", 20, 0).unwrap();
    assert_eq!(page.items[0].source_text, "newer");
    assert_eq!(page.items[0].create_time, "2026-11-01T06:15:00.000Z");
}

#[test]
fn timezone_free_or_invalid_times_are_rejected_before_writing() {
    let (_directory, store) = test_store();
    for time in ["2026-10-03T14:00:00", "2026-10-03 14:00:00", "bad time", ""] {
        assert!(store
            .save_history(&translation("invalid", time))
            .unwrap_err()
            .contains("RFC3339"));
    }
    assert_eq!(store.list_history("", 20, 0).unwrap().total, 0);
}

#[test]
fn history_search_paging_delete_and_clear_have_real_results() {
    let (_directory, store) = test_store();
    let first = store
        .save_history(&translation("alpha first", "2026-06-07T01:00:00Z"))
        .unwrap();
    store
        .save_history(&translation("alpha second", "2026-06-07T02:00:00Z"))
        .unwrap();
    store
        .save_history(&translation("100%_literal", "2026-06-07T03:00:00Z"))
        .unwrap();
    let page = store.list_history("alpha", 1, 1).unwrap();
    assert_eq!(page.total, 2);
    assert_eq!(page.items[0].id, first);
    assert_eq!(store.list_history("%_", 20, 0).unwrap().total, 1);
    assert_eq!(store.list_history("译文", 0, 0).unwrap().total, 3);
    assert!(store.list_history("译文", 0, 0).unwrap().items.is_empty());
    store.delete_history(first).unwrap();
    assert_eq!(store.list_history("alpha", 20, 0).unwrap().total, 1);
    store.clear_history().unwrap();
    assert_eq!(store.list_history("", 20, 0).unwrap().total, 0);
}

#[test]
fn duplicate_favorites_preserve_identity_creation_time_and_unedited_note() {
    let (_directory, store) = test_store();
    let first = store
        .save_favorite("hello", "你好", Some("原备注"))
        .unwrap();
    let updated = store
        .save_favorite("hello", "您好", Some("新备注"))
        .unwrap();
    assert_eq!(updated.id, first.id);
    assert_eq!(updated.create_time, first.create_time);
    assert_eq!(updated.translation, "您好");
    assert_eq!(updated.note.as_deref(), Some("新备注"));
    assert_eq!(store.list_favorites("新备注", 20, 0).unwrap().total, 1);
    let recollected = store.save_favorite("hello", "再译", None).unwrap();
    assert_eq!(recollected.note.as_deref(), Some("新备注"));
    assert_eq!(recollected.id, first.id);
    assert_eq!(recollected.create_time, first.create_time);
    assert_eq!(
        store
            .save_favorite("hello", "您好", Some(""))
            .unwrap()
            .note
            .as_deref(),
        Some("")
    );
    assert!(first.create_time.ends_with('Z'));
    assert!(store.save_favorite("  ", "译文", None).is_err());
    store.delete_favorite(first.id).unwrap();
    assert_eq!(store.list_favorites("", 20, 0).unwrap().total, 0);
}

#[test]
fn favorites_paging_search_and_complete_tsv_export_use_native_times() {
    let (_directory, store) = test_store();
    let connection = store.lock().unwrap();
    for (word, text, note, time) in [
        ("one", "一", "early", "2026-06-07T01:00:00Z"),
        ("two", "二", "middle", "2026-06-07T02:00:00Z"),
        ("three", "三\t译文", "备注\n第二行", "2026-06-07T03:00:00Z"),
    ] {
        connection.execute("INSERT INTO favorite_entries (word, translation, note, created_at_ms) VALUES (?1, ?2, ?3, ?4)", params![word, text, note, parse_timestamp(time).unwrap()]).unwrap();
    }
    drop(connection);
    assert_eq!(store.list_favorites("", 1, 1).unwrap().items[0].word, "two");
    assert_eq!(
        store.list_favorites("early", 20, 0).unwrap().items[0].word,
        "one"
    );
    let exported = store.export_favorites().unwrap();
    assert!(exported.starts_with("单词\t译文\t备注\t创建时间\n"));
    assert!(exported.contains("\"三\t译文\"\t\"备注\n第二行\""));
    assert!(exported.contains("2026-06-07T02:00:00.000Z"));
    assert!(exported.contains("one\t一"));
    assert!(exported.find("three\t").unwrap() < exported.find("one\t").unwrap());
}

#[test]
fn configuration_replaces_complete_ai_config_and_clears_credentials() {
    let (_directory, store) = test_store();
    let first = AiConfig {
        api_key: "fictional-key".into(),
        model: "fixture-model".into(),
        system_prompt: "custom prompt".into(),
        ..AiConfig::default()
    };
    store
        .save_configuration(&Settings::default(), &channel_collection(&first))
        .unwrap();
    let cleared = AiConfig {
        protocol: AiProtocol::Openai,
        endpoint: "".into(),
        api_key: "".into(),
        model: "".into(),
        system_prompt: "".into(),
    };
    let settings = Settings {
        locale: "en_US".into(),
        ..Settings::default()
    };
    store
        .save_configuration(&settings, &channel_collection(&cleared))
        .unwrap();
    assert_eq!(store.settings().unwrap(), settings);
    assert_eq!(store.ai_config().unwrap(), cleared);
    assert_eq!(
        serde_json::from_str::<Value>(&row_json(&store)).unwrap(),
        serde_json::to_value(channel_collection(&cleared)).unwrap()
    );
}

#[test]
fn complete_named_channels_and_active_identity_survive_restart_and_clear_only_one_key() {
    let (directory, store) = test_store();
    let mut value = channel_collection(&AiConfig {
        endpoint: "relay.fixture.invalid".into(),
        api_key: "user-fictional-key".into(),
        model: "user-model".into(),
        system_prompt: "user prompt".into(),
        ..AiConfig::default()
    });
    value.channels[0].config.api_key = "official-fictional-key".into();
    value.channels[0].config.model = "official-model".into();
    value.channels[0].config.protocol = AiProtocol::OpenaiResponses;
    value.channels[1].name = "  我的服务 😀  ".into();
    let saved_settings = Settings {
        locale: "en_US".into(),
        ..Settings::default()
    };
    store.save_configuration(&saved_settings, &value).unwrap();
    let mut saved = value.clone();
    saved.channels[1].name = "我的服务 😀".into();
    assert_eq!(
        store.configuration_snapshot().unwrap(),
        (saved_settings.clone(), saved.clone())
    );
    saved.channels[1].config.api_key.clear();
    store.save_configuration(&saved_settings, &saved).unwrap();
    drop(store);
    let path = directory.path().join(DATABASE_FILE);
    let before = fs::read(&path).unwrap();
    let reopened = Store::open(directory.path()).unwrap();
    assert_eq!(
        reopened.configuration_snapshot().unwrap(),
        (saved_settings, saved.clone())
    );
    assert_eq!(reopened.ai_config().unwrap(), saved.channels[1].config);
    assert_eq!(
        reopened.ai_channels().unwrap().channels[0].config.api_key,
        "official-fictional-key"
    );
    drop(reopened);
    assert_eq!(fs::read(path).unwrap(), before);
}

#[test]
fn invalid_or_missing_settings_are_not_defaulted_or_overwritten() {
    let unknown_settings = {
        let mut value = serde_json::to_value(Settings::default()).unwrap();
        value["defaultApi"] = json!("confidential-fixture");
        value.to_string()
    };
    for value in ["not JSON".to_owned(), "{}".to_owned(), unknown_settings] {
        let (_directory, store) = test_store();
        let previous = store.ai_channels().unwrap();
        let connection = store.lock().unwrap();
        connection
            .execute_batch("PRAGMA ignore_check_constraints = ON")
            .unwrap();
        connection
            .execute(
                "UPDATE app_settings SET settings_json = ?1 WHERE id = 1",
                [&value],
            )
            .unwrap();
        connection
            .execute_batch("PRAGMA ignore_check_constraints = OFF")
            .unwrap();
        drop(connection);
        let error = store.settings().unwrap_err();
        assert!(!error.contains("confidential-fixture"));
        assert!(store
            .save_configuration(&Settings::default(), &previous)
            .is_err());
        let raw: String = store
            .lock()
            .unwrap()
            .query_row(
                "SELECT settings_json FROM app_settings WHERE id = 1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(raw, value);
    }
    let (_directory, store) = test_store();
    store
        .lock()
        .unwrap()
        .execute("DELETE FROM app_settings", [])
        .unwrap();
    assert!(store.settings().is_err());
    assert!(store
        .save_configuration(&Settings::default(), &store.ai_channels().unwrap())
        .is_err());
}

fn malformed_ai_channels() -> Vec<String> {
    let config = AiConfig {
        api_key: "confidential-fixture".into(),
        ..AiConfig::default()
    };
    let complete = serde_json::to_value(channel_collection(&config)).unwrap();
    let mut cases = vec![
        "bad JSON".into(),
        "[]".into(),
        "{}".into(),
        serde_json::to_string(&config).unwrap(),
    ];
    for field in ["activeId", "channels"] {
        let mut value = complete.clone();
        value.as_object_mut().unwrap().remove(field);
        cases.push(value.to_string());
    }
    for field in ["id", "name", "config"] {
        let mut value = complete.clone();
        value["channels"][1].as_object_mut().unwrap().remove(field);
        cases.push(value.to_string());
    }
    for field in ["protocol", "endpoint", "apiKey", "model", "systemPrompt"] {
        let mut value = complete.clone();
        value["channels"][1]["config"]
            .as_object_mut()
            .unwrap()
            .remove(field);
        cases.push(value.to_string());
    }
    for (field, invalid) in [
        ("protocol", json!("unknown")),
        ("protocol", json!("ollama")),
        ("endpoint", json!(7)),
        ("apiKey", Value::Null),
        ("model", json!({"secret":"confidential-fixture"})),
        ("systemPrompt", json!(false)),
        ("modelName", json!("confidential-fixture")),
        (
            "endpoint",
            json!("https://user:confidential-fixture@fixture.invalid"),
        ),
    ] {
        let mut value = complete.clone();
        value["channels"][1]["config"][field] = invalid;
        cases.push(value.to_string());
    }
    for (field, invalid) in [
        ("extra", json!(true)),
        ("activeId", json!(7)),
        ("activeId", json!("missing")),
        ("activeId", json!("add-channel")),
        ("channels", json!([])),
        ("channels", json!({})),
    ] {
        let mut value = complete.clone();
        value[field] = invalid;
        cases.push(value.to_string());
    }
    for (field, invalid) in [
        ("extra", json!(true)),
        ("id", json!("custom")),
        ("id", json!("upfanyi")),
        ("name", json!("")),
        ("name", json!(" padded ")),
        ("name", json!("a\nb")),
        ("name", json!("😀".repeat(61))),
        ("name", json!("UPfanyi official AI")),
    ] {
        let mut value = complete.clone();
        value["channels"][1][field] = invalid;
        cases.push(value.to_string());
    }
    for (field, invalid) in [
        ("name", json!("renamed")),
        ("id", json!("11111111-1111-4111-8111-111111111112")),
        (
            "config",
            json!({"protocol":"ollama","endpoint":"https://uptoken.top","apiKey":"","model":"","systemPrompt":""}),
        ),
    ] {
        let mut value = complete.clone();
        value["channels"][0][field] = invalid;
        cases.push(value.to_string());
    }
    let mut value = complete.clone();
    value["channels"][0]["config"]["endpoint"] = json!("https://relay.fixture.invalid");
    cases.push(value.to_string());
    let mut value = complete.clone();
    value["channels"].as_array_mut().unwrap().remove(0);
    cases.push(value.to_string());
    let mut value = complete.clone();
    let duplicate = value["channels"][1].clone();
    value["channels"].as_array_mut().unwrap().push(duplicate);
    cases.push(value.to_string());
    let mut value = complete.clone();
    let mut duplicate = value["channels"][1].clone();
    duplicate["id"] = json!("11111111-1111-4111-8111-111111111112");
    duplicate["name"] = json!("fIXTURE SERVICE");
    value["channels"].as_array_mut().unwrap().push(duplicate);
    cases.push(value.to_string());
    cases
}

#[test]
fn typed_ai_config_corruption_is_visible_and_cannot_be_overwritten() {
    for raw in malformed_ai_channels() {
        let (_directory, store) = test_store();
        let before_settings = store.settings().unwrap();
        let valid_config = store.ai_channels().unwrap();
        let connection = store.lock().unwrap();
        connection
            .execute_batch("PRAGMA ignore_check_constraints = ON")
            .unwrap();
        connection
            .execute(
                "UPDATE ai_configuration SET config_json = ?1 WHERE id = 1",
                [&raw],
            )
            .unwrap();
        connection
            .execute_batch("PRAGMA ignore_check_constraints = OFF")
            .unwrap();
        drop(connection);
        let error = store.ai_config().unwrap_err();
        assert!(!error.contains("confidential-fixture"));
        let changed = Settings {
            locale: "en_US".into(),
            ..before_settings.clone()
        };
        assert!(store.save_configuration(&changed, &valid_config).is_err());
        assert_eq!(store.settings().unwrap(), before_settings);
        assert_eq!(row_json(&store), raw);
        drop(store);
        assert_rejected_without_changes(_directory.path());
    }
}

#[test]
fn corrupt_configuration_is_rejected_on_restart_without_file_changes() {
    for sql in [
        "UPDATE app_settings SET settings_json = '{}' WHERE id = 1",
        "UPDATE ai_configuration SET config_json = '{}' WHERE id = 1",
    ] {
        let (directory, store) = test_store();
        store.lock().unwrap().execute(sql, []).unwrap();
        drop(store);
        assert_rejected_without_changes(directory.path());
    }
}

#[test]
fn missing_ai_configuration_is_an_error_not_repaired() {
    let (directory, store) = test_store();
    let original = store.configuration_snapshot().unwrap();
    store
        .lock()
        .unwrap()
        .execute("DELETE FROM ai_configuration", [])
        .unwrap();
    assert!(store.ai_config().is_err());
    assert!(store.configuration_snapshot().is_err());
    assert!(store.save_configuration(&original.0, &original.1).is_err());
    drop(store);
    assert_rejected_without_changes(directory.path());
}

#[test]
fn strict_schema_rejects_null_config_extra_rows_noninteger_times_and_invalid_json() {
    let (_directory, store) = test_store();
    let connection = store.lock().unwrap();
    for sql in [
        "UPDATE ai_configuration SET config_json = NULL WHERE id = 1",
        "UPDATE ai_configuration SET config_json = 'bad JSON' WHERE id = 1",
        "UPDATE ai_configuration SET config_json = '[]' WHERE id = 1",
        "INSERT INTO ai_configuration (id, config_json) VALUES (2, '{}')",
        "INSERT INTO favorite_entries (word, translation, created_at_ms) VALUES ('bad', 'bad', '2026-10-03 14:00:00')",
        "INSERT INTO app_settings (id, settings_json) VALUES (2, '{}')",
    ] {
        assert!(connection.execute_batch(sql).is_err());
    }
}

#[test]
fn transaction_failure_rolls_back_settings_and_ai_config() {
    let (_directory, store) = test_store();
    let original = store.configuration_snapshot().unwrap();
    store.lock().unwrap().execute_batch(
        "CREATE TRIGGER reject_configuration BEFORE UPDATE ON ai_configuration BEGIN SELECT RAISE(FAIL, 'simulated persistence failure'); END;"
    ).unwrap();
    let changed = Settings {
        locale: "en_US".into(),
        ..Settings::default()
    };
    let changed_config = AiConfig {
        api_key: "fictional-uncommitted-key".into(),
        model: "uncommitted-model".into(),
        ..original.1.active_config().unwrap().clone()
    };
    assert!(store
        .save_configuration(&changed, &channel_collection(&changed_config))
        .unwrap_err()
        .contains("simulated persistence failure"));
    assert_eq!(store.configuration_snapshot().unwrap(), original);
}

#[test]
fn invalid_configuration_is_rejected_before_any_write() {
    let (_directory, store) = test_store();
    let original = store.configuration_snapshot().unwrap();
    let changed = Settings {
        locale: "en_US".into(),
        ..Settings::default()
    };
    let invalid_config = AiConfig {
        endpoint: "https://user:confidential-fixture@example.invalid/chat".into(),
        ..original.1.active_config().unwrap().clone()
    };
    let error = store
        .save_configuration(&changed, &channel_collection(&invalid_config))
        .unwrap_err();
    assert!(!error.contains("confidential-fixture"));
    let invalid_settings = Settings {
        default_target_lang: "AUTO".into(),
        ..changed
    };
    assert!(store
        .save_configuration(&invalid_settings, &original.1)
        .is_err());
    assert_eq!(store.configuration_snapshot().unwrap(), original);
}

#[test]
fn concurrent_configuration_snapshots_never_mix_saved_groups() {
    use std::sync::{Arc, Barrier};
    let (_directory, store) = test_store();
    let mut groups = [
        (
            Settings::default(),
            AiConfig {
                api_key: "snapshot-key-A".into(),
                model: "snapshot-model-A".into(),
                system_prompt: "snapshot-prompt-A".into(),
                ..AiConfig::default()
            },
        ),
        (
            Settings {
                locale: "en_US".into(),
                default_source_lang: "en".into(),
                ..Settings::default()
            },
            AiConfig {
                protocol: AiProtocol::Openai,
                endpoint: "http://localhost:11434".into(),
                api_key: "snapshot-key-B".into(),
                model: "snapshot-model-B".into(),
                system_prompt: "snapshot-prompt-B".into(),
            },
        ),
    ]
    .map(|(settings, config)| (settings, channel_collection(&config)));
    groups[0].1.channels[0].config = groups[0].1.channels[1].config.clone();
    groups[0].1.active_id = crate::model::OFFICIAL_AI_CHANNEL_ID.into();
    store
        .save_configuration(&groups[0].0, &groups[0].1)
        .unwrap();
    let expected = groups.clone();
    let store = Arc::new(store);
    let start = Arc::new(Barrier::new(2));
    let writer_store = Arc::clone(&store);
    let writer_start = Arc::clone(&start);
    let writer = std::thread::spawn(move || {
        writer_start.wait();
        for iteration in 0..200 {
            let (settings, config) = &groups[iteration % 2];
            writer_store.save_configuration(settings, config).unwrap();
            std::thread::yield_now();
        }
    });
    start.wait();
    for _ in 0..400 {
        let snapshot = store.configuration_snapshot().unwrap();
        assert!(
            snapshot == expected[0] || snapshot == expected[1],
            "读取到了不完整配置组"
        );
        std::thread::yield_now();
    }
    writer.join().unwrap();
}

#[test]
fn actual_database_write_failures_are_visible() {
    let (_directory, store) = test_store();
    store
        .lock()
        .unwrap()
        .execute_batch("PRAGMA query_only = ON")
        .unwrap();
    assert!(store
        .save_history(&translation("hello", "2026-06-07T01:00:00Z"))
        .is_err());
    assert!(store.save_favorite("hello", "你好", None).is_err());
    assert!(store.clear_history().is_err());
    assert!(store
        .save_configuration(&Settings::default(), &store.ai_channels().unwrap())
        .is_err());
}

#[test]
fn candidate_publish_never_overwrites_an_existing_destination() {
    let directory = tempdir().unwrap();
    let destination = directory.path().join(DATABASE_FILE);
    fs::write(&destination, b"original-data").unwrap();
    let candidate = CandidateDatabase::create(directory.path()).unwrap();
    fs::write(&candidate.path, b"candidate-data").unwrap();
    assert!(candidate.publish(&destination).is_err());
    assert_eq!(fs::read(&destination).unwrap(), b"original-data");
    drop(candidate);
    assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 1);
}

#[test]
fn concurrent_candidate_publish_has_exactly_one_winner() {
    let directory = tempdir().unwrap();
    let destination = directory.path().join(DATABASE_FILE);
    let first = CandidateDatabase::create(directory.path()).unwrap();
    let second = CandidateDatabase::create(directory.path()).unwrap();
    fs::write(&first.path, b"first").unwrap();
    fs::write(&second.path, b"second").unwrap();
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
    let handles: Vec<_> = [first, second]
        .into_iter()
        .map(|candidate| {
            let destination = destination.clone();
            let barrier = std::sync::Arc::clone(&barrier);
            std::thread::spawn(move || {
                barrier.wait();
                candidate.publish(&destination)
            })
        })
        .collect();
    let successes = handles
        .into_iter()
        .map(|handle| handle.join().unwrap())
        .filter(Result::is_ok)
        .count();
    assert_eq!(successes, 1);
    let result = fs::read(destination).unwrap();
    assert!(result == b"first" || result == b"second");
    assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 1);
}
