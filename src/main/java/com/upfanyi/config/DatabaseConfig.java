package com.upfanyi.config;

import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

import java.io.IOException;
import java.io.InputStream;
import java.nio.charset.StandardCharsets;
import java.sql.Connection;
import java.sql.DriverManager;
import java.sql.SQLException;
import java.sql.Statement;

/**
 * 数据库配置管理类
 *
 * @author UP翻译团队
 * @version 1.0.0
 */
public class DatabaseConfig {

    private static final Logger logger = LoggerFactory.getLogger(DatabaseConfig.class);

    private static DatabaseConfig instance;

    private String dbPath;
    private Connection connection;

    private DatabaseConfig() {
    }

    public static synchronized DatabaseConfig getInstance() {
        if (instance == null) {
            instance = new DatabaseConfig();
        }
        return instance;
    }

    /**
     * 初始化数据库
     */
    public void init(String dbPath) {
        this.dbPath = dbPath;
        try {
            // 加载SQLite驱动
            Class.forName("org.sqlite.JDBC");
            logger.info("SQLite驱动加载成功");

            // 创建数据库连接
            connection = DriverManager.getConnection("jdbc:sqlite:" + dbPath);
            logger.info("数据库连接成功: {}", dbPath);

            // 初始化表结构
            initTables();
        } catch (ClassNotFoundException e) {
            logger.error("SQLite驱动加载失败", e);
        } catch (SQLException e) {
            logger.error("数据库连接失败", e);
        }
    }

    /**
     * 初始化数据库表
     */
    private void initTables() {
        try {
            // 读取SQL脚本
            InputStream is = getClass().getResourceAsStream("/db/schema.sql");
            if (is == null) {
                logger.error("无法找到数据库初始化脚本");
                createTablesManually();
                return;
            }

            String sql = new String(is.readAllBytes(), StandardCharsets.UTF_8);
            is.close();

            // 执行SQL
            Statement stmt = connection.createStatement();
            String[] statements = sql.split(";");
            for (String statement : statements) {
                String trimmed = statement.trim();
                if (!trimmed.isEmpty()) {
                    stmt.execute(trimmed);
                }
            }
            stmt.close();
            logger.info("数据库表初始化成功");

            // 运行数据库迁移
            runMigrations();
        } catch (IOException e) {
            logger.error("读取SQL脚本失败", e);
            createTablesManually();
            runMigrations();
        } catch (SQLException e) {
            // 如果表已存在，忽略错误
            logger.warn("数据库表初始化警告: {}", e.getMessage());
            runMigrations();
        }
    }

    /**
     * 运行数据库迁移
     */
    private void runMigrations() {
        try {
            Statement stmt = connection.createStatement();

            // 迁移1: 为api_config表添加新列
            addColumnIfNotExists(stmt, "api_config", "endpoint", "VARCHAR(500)");
            addColumnIfNotExists(stmt, "api_config", "model_name", "VARCHAR(100)");
            addColumnIfNotExists(stmt, "api_config", "extra_config", "TEXT");

            // 添加LLM配置记录（如果不存在）
            stmt.execute("INSERT OR IGNORE INTO api_config (api_name, is_enabled, priority) VALUES ('llm', FALSE, 3)");

            stmt.close();
            logger.info("数据库迁移完成");
        } catch (SQLException e) {
            logger.warn("数据库迁移警告: {}", e.getMessage());
        }
    }

    /**
     * 如果列不存在则添加
     */
    private void addColumnIfNotExists(Statement stmt, String table, String column, String type) {
        try {
            // SQLite不支持IF NOT EXISTS for ALTER TABLE ADD COLUMN，所以直接尝试添加
            stmt.execute("ALTER TABLE " + table + " ADD COLUMN " + column + " " + type);
            logger.info("添加列 {}.{} 成功", table, column);
        } catch (SQLException e) {
            // 列已存在会报错，忽略
            if (e.getMessage().contains("duplicate column name") || e.getMessage().contains("already exists")) {
                logger.debug("列 {}.{} 已存在", table, column);
            } else {
                logger.warn("添加列 {}.{} 失败: {}", table, column, e.getMessage());
            }
        }
    }

    /**
     * 手动创建数据库表
     */
    private void createTablesManually() {
        try {
            Statement stmt = connection.createStatement();

            // 历史记录表
            stmt.execute("""
                CREATE TABLE IF NOT EXISTS history (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    source_text TEXT NOT NULL,
                    translated_text TEXT NOT NULL,
                    source_lang VARCHAR(10),
                    target_lang VARCHAR(10),
                    api_used VARCHAR(50),
                    create_time DATETIME DEFAULT CURRENT_TIMESTAMP
                )
                """);

            // 收藏/生词本表
            stmt.execute("""
                CREATE TABLE IF NOT EXISTS favorites (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    word TEXT NOT NULL UNIQUE,
                    translation TEXT NOT NULL,
                    note TEXT,
                    create_time DATETIME DEFAULT CURRENT_TIMESTAMP
                )
                """);

            // API配置表
            stmt.execute("""
                CREATE TABLE IF NOT EXISTS api_config (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    api_name VARCHAR(50) NOT NULL UNIQUE,
                    api_key VARCHAR(255),
                    api_secret VARCHAR(255),
                    is_enabled BOOLEAN DEFAULT TRUE,
                    priority INTEGER DEFAULT 0
                )
                """);

            // 应用设置表
            stmt.execute("""
                CREATE TABLE IF NOT EXISTS settings (
                    key VARCHAR(100) PRIMARY KEY,
                    value TEXT,
                    description TEXT
                )
                """);

            stmt.close();
            logger.info("数据库表创建成功");
        } catch (SQLException e) {
            logger.error("创建数据库表失败", e);
        }
    }

    /**
     * 获取数据库连接
     */
    public Connection getConnection() throws SQLException {
        if (connection == null || connection.isClosed()) {
            connection = DriverManager.getConnection("jdbc:sqlite:" + dbPath);
        }
        return connection;
    }

    /**
     * 关闭数据库连接
     */
    public void close() {
        if (connection != null) {
            try {
                connection.close();
                logger.info("数据库连接已关闭");
            } catch (SQLException e) {
                logger.error("关闭数据库连接失败", e);
            }
        }
    }
}
