use super::config::{read_ai_channels, read_settings, write_ai_channels, write_settings};
use super::database_error;
use crate::model::{AiChannels, Settings};
use rusqlite::Connection;
use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

// SQLite 文件属于 UPFY 原生 AI 格式 v3，不能靠文件名或相似表名推断兼容。
pub(super) const APPLICATION_ID: i64 = 0x5550_4659;
pub(super) const SCHEMA_VERSION: i64 = 3;

const OBJECTS: [(&str, &str, &str); 6] = [
    ("table", "translation_history", "CREATE TABLE translation_history (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        source_text TEXT NOT NULL, translated_text TEXT NOT NULL,
        source_lang TEXT NOT NULL, target_lang TEXT NOT NULL, model TEXT NOT NULL,
        created_at_ms INTEGER NOT NULL CHECK(created_at_ms BETWEEN -62167219200000 AND 253402300799999)
    ) STRICT"),
    ("table", "favorite_entries", "CREATE TABLE favorite_entries (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        word TEXT NOT NULL UNIQUE, translation TEXT NOT NULL, note TEXT,
        created_at_ms INTEGER NOT NULL CHECK(created_at_ms BETWEEN -62167219200000 AND 253402300799999)
    ) STRICT"),
    ("table", "ai_configuration", "CREATE TABLE ai_configuration (
        id INTEGER PRIMARY KEY CHECK(id = 1),
        config_json TEXT NOT NULL CHECK(json_valid(config_json) AND json_type(config_json) = 'object')
    ) STRICT"),
    ("table", "app_settings", "CREATE TABLE app_settings (
        id INTEGER PRIMARY KEY CHECK(id = 1),
        settings_json TEXT NOT NULL CHECK(json_valid(settings_json) AND json_type(settings_json) = 'object')
    ) STRICT"),
    ("index", "idx_translation_history_time", "CREATE INDEX idx_translation_history_time ON translation_history(created_at_ms DESC, id DESC)"),
    ("index", "idx_favorite_entries_time", "CREATE INDEX idx_favorite_entries_time ON favorite_entries(created_at_ms DESC, id DESC)"),
];

pub(super) fn prepare_connection(connection: &Connection) -> Result<(), String> {
    connection
        .busy_timeout(Duration::from_secs(5))
        .map_err(database_error)?;
    // 均为当前连接设置，验证未知数据库之前不修改其日志模式或内容。
    connection
        .execute_batch("PRAGMA foreign_keys = ON;")
        .map_err(database_error)
}

pub(super) fn initialize_database(connection: &mut Connection) -> Result<(), String> {
    let transaction = connection.transaction().map_err(database_error)?;
    for (_, _, sql) in OBJECTS {
        transaction.execute_batch(sql).map_err(database_error)?;
    }
    transaction
        .pragma_update(None, "application_id", APPLICATION_ID)
        .map_err(database_error)?;
    transaction
        .pragma_update(None, "user_version", SCHEMA_VERSION)
        .map_err(database_error)?;
    write_settings(&transaction, &Settings::default())?;
    write_ai_channels(&transaction, &AiChannels::default())?;
    transaction.commit().map_err(database_error)
}

pub(super) fn validate_database(connection: &mut Connection) -> Result<(), String> {
    let transaction = connection.transaction().map_err(database_error)?;
    let application_id: i64 = transaction
        .pragma_query_value(None, "application_id", |row| row.get(0))
        .map_err(database_error)?;
    let version: i64 = transaction
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .map_err(database_error)?;
    if application_id != APPLICATION_ID || version != SCHEMA_VERSION {
        return Err("数据库不是本版原生格式，或格式版本不受支持；原文件未修改".into());
    }
    let expected: BTreeMap<_, _> = OBJECTS
        .iter()
        .map(|(kind, name, sql)| ((kind.to_string(), name.to_string()), normalized_sql(sql)))
        .collect();
    let actual: BTreeMap<_, _> = {
        let mut statement = transaction
            .prepare("SELECT type, name, sql FROM sqlite_schema WHERE name NOT GLOB 'sqlite_*'")
            .map_err(database_error)?;
        let rows = statement
            .query_map([], |row| {
                Ok((
                    (row.get::<_, String>(0)?, row.get::<_, String>(1)?),
                    normalized_sql(&row.get::<_, String>(2)?),
                ))
            })
            .map_err(database_error)?;
        rows.collect::<rusqlite::Result<_>>()
            .map_err(database_error)?
    };
    // 固定 v3 的结构与约束；错误版本、影子列、缺列和意外触发器均不自动修补。
    if actual != expected {
        return Err("数据库结构与本版原生格式不一致；原文件未修改".into());
    }
    let integrity: String = transaction
        .query_row("PRAGMA quick_check", [], |row| row.get(0))
        .map_err(database_error)?;
    if integrity != "ok" {
        return Err("数据库完整性检查失败；原文件未修改".into());
    }
    read_settings(&transaction)?;
    read_ai_channels(&transaction)?;
    transaction.commit().map_err(database_error)
}

fn normalized_sql(sql: &str) -> String {
    sql.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub(super) struct CandidateDatabase {
    pub(super) path: PathBuf,
}

impl CandidateDatabase {
    pub(super) fn create(data_dir: &Path) -> Result<Self, String> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| "系统时间不可用于创建候选数据库")?
            .as_nanos();
        let path = data_dir.join(format!(
            ".upfanyi-initialize-{}-{timestamp}.sqlite3",
            std::process::id()
        ));
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|error| format!("无法创建候选数据库文件: {error}"))?;
        Ok(Self { path })
    }

    pub(super) fn publish(&self, destination: &Path) -> Result<(), String> {
        OpenOptions::new()
            .read(true)
            .write(true)
            .open(&self.path)
            .and_then(|file| file.sync_all())
            .map_err(|error| format!("候选数据库落盘失败: {error}"))?;
        // 硬链接原子创建尚不存在的路径；rename 在 Windows 也可能覆盖目标。
        fs::hard_link(&self.path, destination)
            .map_err(|error| format!("无法启用新数据库，已有数据保持不变: {error}"))?;
        fs::remove_file(&self.path).map_err(|error| format!("候选数据库清理失败: {error}"))?;
        Ok(())
    }
}

impl Drop for CandidateDatabase {
    fn drop(&mut self) {
        // 仅清理本次候选及其日志，不扫描其他目录或来源不明的数据库。
        let _ = fs::remove_file(&self.path);
        for suffix in ["-journal", "-wal", "-shm"] {
            let mut name = self.path.as_os_str().to_os_string();
            name.push(suffix);
            let _ = fs::remove_file(PathBuf::from(name));
        }
    }
}
