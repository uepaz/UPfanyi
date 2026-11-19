use super::database_error;
use crate::{
    channels,
    model::{AiChannels, Settings},
    validation,
};
use rusqlite::{Connection, Transaction};

pub(super) fn read_settings(connection: &Connection) -> Result<Settings, String> {
    let value: String = connection
        .query_row(
            "SELECT settings_json FROM app_settings WHERE id = 1",
            [],
            |row| row.get(0),
        )
        .map_err(|_| "原生应用设置缺失或无法读取".to_owned())?;
    let settings: Settings =
        serde_json::from_str(&value).map_err(|_| "本版设置 JSON 不完整或格式错误".to_owned())?;
    validation::validate_settings(&settings)?;
    Ok(settings)
}

pub(super) fn read_ai_channels(connection: &Connection) -> Result<AiChannels, String> {
    let value: String = connection
        .query_row(
            "SELECT config_json FROM ai_configuration WHERE id = 1",
            [],
            |row| row.get(0),
        )
        .map_err(|_| "本版 AI 配置缺失或无法读取".to_owned())?;
    // 集合及每渠道配置必须完整；未知字段、无效身份与引用均拒绝，错误不回显原 JSON。
    let config: AiChannels = serde_json::from_str(&value)
        .map_err(|_| "本版 AI 配置 JSON 不完整或格式错误".to_owned())?;
    channels::validate(&config)?;
    Ok(config)
}

pub(super) fn write_settings(
    transaction: &Transaction<'_>,
    settings: &Settings,
) -> Result<(), String> {
    let value = serde_json::to_string(settings).map_err(|_| "设置序列化失败".to_owned())?;
    transaction.execute(
        "INSERT INTO app_settings (id, settings_json) VALUES (1, ?1) ON CONFLICT(id) DO UPDATE SET settings_json = excluded.settings_json",
        [value],
    ).map_err(database_error)?;
    Ok(())
}

pub(super) fn write_ai_channels(
    transaction: &Transaction<'_>,
    config: &AiChannels,
) -> Result<(), String> {
    let value = serde_json::to_string(config).map_err(|_| "AI 配置序列化失败".to_owned())?;
    // 保存整个配置，不与原 JSON 合并；显式清空密钥不会从其他来源复活。
    transaction.execute(
        "INSERT INTO ai_configuration (id, config_json) VALUES (1, ?1) ON CONFLICT(id) DO UPDATE SET config_json = excluded.config_json",
        [value],
    ).map_err(database_error)?;
    Ok(())
}
