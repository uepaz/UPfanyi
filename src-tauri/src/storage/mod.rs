mod config;
mod schema;

#[cfg(test)]
mod tests;

use self::config::{read_ai_channels, read_settings, write_ai_channels, write_settings};
use self::schema::{initialize_database, prepare_connection, validate_database, CandidateDatabase};
use crate::model::{
    AiChannels, AiConfig, FavoriteWord, HistoryRecord, Page, Settings, TargetLanguage, Translation,
};
use chrono::{DateTime, SecondsFormat, Utc};
use rusqlite::{params, Connection, OpenFlags, Row};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard};

const DATABASE_FILE: &str = "upfanyi.sqlite3";

pub struct Store {
    connection: Mutex<Connection>,
    // 在连接锁内读取或递增。版本只对本次运行的窗口有效，语言本身保存在 SQLite。
    target_revision: AtomicU64,
    pub data_dir: PathBuf,
}

impl Store {
    pub fn open(data_dir: &Path) -> Result<Self, String> {
        fs::create_dir_all(data_dir).map_err(|error| format!("无法创建数据目录: {error}"))?;
        let database_path = data_dir.join(DATABASE_FILE);
        if database_path
            .try_exists()
            .map_err(|error| format!("无法检查数据库文件: {error}"))?
        {
            // 先以只读连接验证；不对未知文件执行修补、journal_mode 或默认值写入。
            let mut existing = Connection::open_with_flags(
                &database_path,
                OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
            )
            .map_err(database_error)?;
            prepare_connection(&existing)?;
            validate_database(&mut existing)?;
            existing
                .close()
                .map_err(|(_, error)| database_error(error))?;
        } else {
            // 仅本次候选库通过完整初始化、验证和落盘后才发布；已有目标不会被覆盖。
            let candidate = CandidateDatabase::create(data_dir)?;
            let mut connection = Connection::open(&candidate.path).map_err(database_error)?;
            prepare_connection(&connection)?;
            initialize_database(&mut connection)?;
            validate_database(&mut connection)?;
            connection
                .close()
                .map_err(|(_, error)| database_error(error))?;
            candidate.publish(&database_path)?;
        }
        let mut connection = Connection::open_with_flags(
            &database_path,
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .map_err(database_error)?;
        prepare_connection(&connection)?;
        // 写连接再次验证，连接建立后仍不修改数据库格式或补齐缺失配置。
        validate_database(&mut connection)?;
        Ok(Self {
            connection: Mutex::new(connection),
            target_revision: AtomicU64::new(0),
            data_dir: data_dir.to_path_buf(),
        })
    }

    pub fn settings(&self) -> Result<Settings, String> {
        let connection = self.lock()?;
        read_settings(&connection)
    }

    pub fn target_language(&self) -> Result<TargetLanguage, String> {
        let connection = self.lock()?;
        Ok(TargetLanguage {
            language: read_settings(&connection)?.default_target_lang,
            revision: self.target_revision.load(Ordering::Relaxed),
        })
    }

    pub fn set_target_language(&self, language: &str) -> Result<TargetLanguage, String> {
        crate::validation::validate_languages("auto", language)?;
        let mut connection = self.lock()?;
        let transaction = connection.transaction().map_err(database_error)?;
        let mut settings = read_settings(&transaction)?;
        let changed = settings.default_target_lang != language;
        if changed {
            settings.default_target_lang = language.to_owned();
            write_settings(&transaction, &settings)?;
        }
        transaction.commit().map_err(database_error)?;
        if changed {
            self.target_revision.fetch_add(1, Ordering::Relaxed);
        }
        Ok(TargetLanguage {
            language: settings.default_target_lang,
            revision: self.target_revision.load(Ordering::Relaxed),
        })
    }

    pub fn ai_config(&self) -> Result<AiConfig, String> {
        Ok(self.ai_channels()?.active_config()?.clone())
    }

    pub fn ai_channels(&self) -> Result<AiChannels, String> {
        let connection = self.lock()?;
        read_ai_channels(&connection)
    }

    pub fn configuration_snapshot(&self) -> Result<(Settings, AiChannels), String> {
        let mut connection = self.lock()?;
        // 一次互斥区间阻止本应用写入；读事务也让其他 SQLite 连接无法混入半组配置。
        let transaction = connection.transaction().map_err(database_error)?;
        let settings = read_settings(&transaction)?;
        let config = read_ai_channels(&transaction)?;
        transaction.commit().map_err(database_error)?;
        Ok((settings, config))
    }

    pub fn save_configuration(
        &self,
        settings: &Settings,
        config: &AiChannels,
    ) -> Result<(), String> {
        crate::validation::validate_settings(settings)?;
        let config = crate::channels::normalize(config)?;
        let mut connection = self.lock()?;
        let transaction = connection.transaction().map_err(database_error)?;
        // 设置页的草稿可能早于另一个窗口的语言选择。仅专用命令可更新目标语言。
        let settings = Settings {
            default_target_lang: read_settings(&transaction)?.default_target_lang,
            ..settings.clone()
        };
        read_ai_channels(&transaction)?;
        write_settings(&transaction, &settings)?;
        write_ai_channels(&transaction, &config)?;
        transaction.commit().map_err(database_error)
    }

    pub fn save_history(&self, translation: &Translation) -> Result<i64, String> {
        let created_at_ms = parse_timestamp(&translation.create_time)?;
        let connection = self.lock()?;
        connection
            .execute(
                "INSERT INTO translation_history \
                 (source_text, translated_text, source_lang, target_lang, model, created_at_ms) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    translation.source_text,
                    translation.translated_text,
                    translation.source_lang,
                    translation.target_lang,
                    translation.model,
                    created_at_ms,
                ],
            )
            .map_err(database_error)?;
        Ok(connection.last_insert_rowid())
    }

    pub fn list_history(
        &self,
        keyword: &str,
        limit: usize,
        offset: usize,
    ) -> Result<Page<HistoryRecord>, String> {
        let connection = self.lock()?;
        let pattern = search_pattern(keyword);
        let total: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM translation_history \
                 WHERE source_text LIKE ?1 ESCAPE '\\' OR translated_text LIKE ?1 ESCAPE '\\'",
                [&pattern],
                |row| row.get(0),
            )
            .map_err(database_error)?;
        let mut statement = connection
            .prepare(
                "SELECT id, source_text, translated_text, source_lang, target_lang, model, created_at_ms \
                 FROM translation_history \
                 WHERE source_text LIKE ?1 ESCAPE '\\' OR translated_text LIKE ?1 ESCAPE '\\' \
                 ORDER BY created_at_ms DESC, id DESC LIMIT ?2 OFFSET ?3"
            )
            .map_err(database_error)?;
        let rows = statement
            .query_map(
                params![pattern, page_number(limit)?, page_number(offset)?],
                history_from_row,
            )
            .map_err(database_error)?;
        Ok(Page {
            items: rows
                .collect::<rusqlite::Result<Vec<_>>>()
                .map_err(database_error)?,
            total: total as usize,
        })
    }

    pub fn delete_history(&self, id: i64) -> Result<(), String> {
        self.lock()?
            .execute("DELETE FROM translation_history WHERE id = ?1", [id])
            .map_err(database_error)?;
        Ok(())
    }

    pub fn clear_history(&self) -> Result<(), String> {
        self.lock()?
            .execute("DELETE FROM translation_history", [])
            .map_err(database_error)?;
        Ok(())
    }

    pub fn list_favorites(
        &self,
        keyword: &str,
        limit: usize,
        offset: usize,
    ) -> Result<Page<FavoriteWord>, String> {
        let connection = self.lock()?;
        let pattern = search_pattern(keyword);
        let total: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM favorite_entries \
                 WHERE word LIKE ?1 ESCAPE '\\' OR translation LIKE ?1 ESCAPE '\\' \
                 OR note LIKE ?1 ESCAPE '\\'",
                [&pattern],
                |row| row.get(0),
            )
            .map_err(database_error)?;
        let mut statement = connection
            .prepare(
                "SELECT id, word, translation, note, created_at_ms FROM favorite_entries \
                 WHERE word LIKE ?1 ESCAPE '\\' OR translation LIKE ?1 ESCAPE '\\' \
                 OR note LIKE ?1 ESCAPE '\\' \
                 ORDER BY created_at_ms DESC, id DESC LIMIT ?2 OFFSET ?3",
            )
            .map_err(database_error)?;
        let rows = statement
            .query_map(
                params![pattern, page_number(limit)?, page_number(offset)?],
                favorite_from_row,
            )
            .map_err(database_error)?;
        Ok(Page {
            items: rows
                .collect::<rusqlite::Result<Vec<_>>>()
                .map_err(database_error)?,
            total: total as usize,
        })
    }

    pub fn save_favorite(
        &self,
        word: &str,
        translation: &str,
        note: Option<&str>,
    ) -> Result<FavoriteWord, String> {
        if word.trim().is_empty() {
            return Err("收藏原文不能为空".into());
        }
        let connection = self.lock()?;
        let created_at_ms = Utc::now().timestamp_millis();
        // 使用 UPDATE 保留已有收藏的 id 与创建时间，避免 REPLACE 的删除/重建副作用。
        connection
            .execute(
                "INSERT INTO favorite_entries (word, translation, note, created_at_ms) \
                 VALUES (?1, ?2, ?3, ?4) \
                 ON CONFLICT(word) DO UPDATE SET translation = excluded.translation, \
                 note = CASE WHEN excluded.note IS NULL THEN favorite_entries.note ELSE excluded.note END",
                params![word, translation, note, created_at_ms],
            )
            .map_err(database_error)?;
        connection
            .query_row(
                "SELECT id, word, translation, note, created_at_ms FROM favorite_entries WHERE word = ?1",
                [word],
                favorite_from_row,
            )
            .map_err(database_error)
    }

    pub fn delete_favorite(&self, id: i64) -> Result<(), String> {
        self.lock()?
            .execute("DELETE FROM favorite_entries WHERE id = ?1", [id])
            .map_err(database_error)?;
        Ok(())
    }

    pub fn export_favorites(&self) -> Result<String, String> {
        let connection = self.lock()?;
        let mut statement = connection
            .prepare(
                "SELECT id, word, translation, note, created_at_ms FROM favorite_entries \
                 ORDER BY created_at_ms DESC, id DESC",
            )
            .map_err(database_error)?;
        let rows = statement
            .query_map([], favorite_from_row)
            .map_err(database_error)?;
        let mut output = String::from("单词\t译文\t备注\t创建时间\n");
        for row in rows {
            let row = row.map_err(database_error)?;
            let fields = [
                row.word,
                row.translation,
                row.note.unwrap_or_default(),
                row.create_time,
            ];
            output.push_str(
                &fields
                    .iter()
                    .map(|value| tsv_field(value))
                    .collect::<Vec<_>>()
                    .join("\t"),
            );
            output.push('\n');
        }
        Ok(output)
    }

    fn lock(&self) -> Result<MutexGuard<'_, Connection>, String> {
        self.connection
            .lock()
            .map_err(|_| "本地数据库锁不可用，请重启应用".into())
    }
}

fn history_from_row(row: &Row<'_>) -> rusqlite::Result<HistoryRecord> {
    Ok(HistoryRecord {
        id: row.get(0)?,
        source_text: row.get(1)?,
        translated_text: row.get(2)?,
        source_lang: row.get(3)?,
        target_lang: row.get(4)?,
        model: row.get(5)?,
        create_time: timestamp_from_row(row, 6)?,
    })
}

fn favorite_from_row(row: &Row<'_>) -> rusqlite::Result<FavoriteWord> {
    Ok(FavoriteWord {
        id: row.get(0)?,
        word: row.get(1)?,
        translation: row.get(2)?,
        note: row.get(3)?,
        create_time: timestamp_from_row(row, 4)?,
    })
}

fn search_pattern(keyword: &str) -> String {
    format!(
        "%{}%",
        keyword
            .replace('\\', "\\\\")
            .replace('%', "\\%")
            .replace('_', "\\_")
    )
}

fn page_number(value: usize) -> Result<i64, String> {
    value.try_into().map_err(|_| "分页参数超出支持范围".into())
}

fn tsv_field(value: &str) -> String {
    if value.contains(['\t', '\n', '\r', '"']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_owned()
    }
}

fn database_error(error: rusqlite::Error) -> String {
    format!("本地数据库操作失败: {error}")
}

// 只接受带时区的标准输入；毫秒精度是本版存储契约，不猜测本地时区。
fn parse_timestamp(value: &str) -> Result<i64, String> {
    DateTime::parse_from_rfc3339(value)
        .map(|time| time.timestamp_millis())
        .map_err(|_| "记录时间必须是带时区的 RFC3339 格式".into())
}

fn timestamp_from_row(row: &Row<'_>, index: usize) -> rusqlite::Result<String> {
    let millis: i64 = row.get(index)?;
    DateTime::<Utc>::from_timestamp_millis(millis)
        .map(|time| time.to_rfc3339_opts(SecondsFormat::Millis, true))
        .ok_or_else(|| {
            rusqlite::Error::FromSqlConversionFailure(
                index,
                rusqlite::types::Type::Integer,
                std::io::Error::new(std::io::ErrorKind::InvalidData, "记录时间超出有效范围").into(),
            )
        })
}
