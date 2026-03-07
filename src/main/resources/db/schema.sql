-- UP翻译数据库初始化脚本

-- 历史记录表
CREATE TABLE IF NOT EXISTS history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    source_text TEXT NOT NULL,
    translated_text TEXT NOT NULL,
    source_lang VARCHAR(10),
    target_lang VARCHAR(10),
    api_used VARCHAR(50),
    create_time DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- 创建历史记录表的索引
CREATE INDEX IF NOT EXISTS idx_history_create_time ON history(create_time);
CREATE INDEX IF NOT EXISTS idx_history_source_text ON history(source_text);

-- 收藏/生词本表
CREATE TABLE IF NOT EXISTS favorites (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    word TEXT NOT NULL UNIQUE,
    translation TEXT NOT NULL,
    note TEXT,
    create_time DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- 创建收藏表的索引
CREATE INDEX IF NOT EXISTS idx_favorites_word ON favorites(word);
CREATE INDEX IF NOT EXISTS idx_favorites_create_time ON favorites(create_time);

-- API配置表
CREATE TABLE IF NOT EXISTS api_config (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    api_name VARCHAR(50) NOT NULL UNIQUE,
    api_key VARCHAR(255),
    api_secret VARCHAR(255),
    endpoint VARCHAR(500),
    model_name VARCHAR(100),
    extra_config TEXT,
    is_enabled BOOLEAN DEFAULT TRUE,
    priority INTEGER DEFAULT 0
);

-- 插入默认API配置
INSERT OR IGNORE INTO api_config (api_name, is_enabled, priority) VALUES ('baidu', TRUE, 1);
INSERT OR IGNORE INTO api_config (api_name, is_enabled, priority) VALUES ('youdao', TRUE, 2);
INSERT OR IGNORE INTO api_config (api_name, is_enabled, priority) VALUES ('llm', FALSE, 3);

-- 应用设置表
CREATE TABLE IF NOT EXISTS settings (
    key VARCHAR(100) PRIMARY KEY,
    value TEXT,
    description TEXT
);

-- 插入默认设置
INSERT OR IGNORE INTO settings (key, value, description) VALUES ('app.locale', 'zh_CN', 'Interface language');
INSERT OR IGNORE INTO settings (key, value, description) VALUES ('app.theme', 'default', 'UI theme');
INSERT OR IGNORE INTO settings (key, value, description) VALUES ('app.minimize_to_tray', 'true', 'Minimize to system tray');
INSERT OR IGNORE INTO settings (key, value, description) VALUES ('app.always_on_top', 'false', 'Always on top');
INSERT OR IGNORE INTO settings (key, value, description) VALUES ('clipboard.monitor', 'false', 'Monitor clipboard');
INSERT OR IGNORE INTO settings (key, value, description) VALUES ('clipboard.min_length', '1', 'Minimum text length for clipboard translation');
INSERT OR IGNORE INTO settings (key, value, description) VALUES ('hotkey.show_window', 'ctrl shift T', 'Hotkey to show window');
INSERT OR IGNORE INTO settings (key, value, description) VALUES ('hotkey.clipboard_translate', 'ctrl shift C', 'Hotkey to translate clipboard');
INSERT OR IGNORE INTO settings (key, value, description) VALUES ('api.default', 'baidu', 'Default translation API');
INSERT OR IGNORE INTO settings (key, value, description) VALUES ('translate.source_lang', 'auto', 'Default source language');
INSERT OR IGNORE INTO settings (key, value, description) VALUES ('translate.target_lang', 'en', 'Default target language');
INSERT OR IGNORE INTO settings (key, value, description) VALUES ('translate.auto_detect', 'true', 'Auto detect source language');
