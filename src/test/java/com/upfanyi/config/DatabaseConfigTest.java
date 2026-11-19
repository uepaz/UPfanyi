package com.upfanyi.config;

import com.upfanyi.api.ApiFactory;
import com.upfanyi.dao.ApiConfigDao;
import com.upfanyi.model.ApiConfig;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;

import java.nio.file.Path;
import java.sql.Connection;
import java.sql.DriverManager;
import java.sql.PreparedStatement;
import java.sql.SQLException;
import java.util.List;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;

class DatabaseConfigTest {

    @TempDir
    Path tempDir;

    @BeforeEach
    void setUp() {
        DatabaseConfig.getInstance().init(tempDir.resolve("connection-test.db").toString());
    }

    @AfterEach
    void tearDown() {
        DatabaseConfig.getInstance().close();
    }

    @Test
    void connectionsAreIndependentForTryWithResourcesCallers() throws SQLException {
        Connection first = DatabaseConfig.getInstance().getConnection();
        Connection second = DatabaseConfig.getInstance().getConnection();

        first.close();

        assertFalse(second.isClosed());
        second.close();
    }

    @Test
    void initCreatesDefaultConfigRowsForAllRegisteredApis() {
        List<String> apiNames = new ApiConfigDao().findAll().stream()
                .map(ApiConfig::getApiName)
                .toList();

        assertEquals(ApiFactory.getInstance().getRegisteredApis(), apiNames);
    }

    @Test
    void initMigratesLegacyApiPrioritiesToTheCurrentDefaultOrder() throws Exception {
        Path dbPath = tempDir.resolve("legacy-priority.db");
        createLegacyApiConfigDb(dbPath);

        DatabaseConfig.getInstance().init(dbPath.toString());

        List<String> apiNames = new ApiConfigDao().findAll().stream()
                .map(ApiConfig::getApiName)
                .toList();

        assertEquals(ApiFactory.getInstance().getRegisteredApis(), apiNames);
        assertEquals(8, new ApiConfigDao().findByName("llm").getPriority());
    }

    private void createLegacyApiConfigDb(Path dbPath) throws Exception {
        Class.forName("org.sqlite.JDBC");
        try (Connection conn = DriverManager.getConnection("jdbc:sqlite:" + dbPath)) {
            conn.createStatement().execute("""
                    CREATE TABLE api_config (
                        id INTEGER PRIMARY KEY AUTOINCREMENT,
                        api_name VARCHAR(50) NOT NULL UNIQUE,
                        api_key VARCHAR(255),
                        api_secret VARCHAR(255),
                        is_enabled BOOLEAN DEFAULT TRUE,
                        priority INTEGER DEFAULT 0
                    )
                    """);

            try (PreparedStatement pstmt = conn.prepareStatement(
                    "INSERT INTO api_config (api_name, is_enabled, priority) VALUES (?, ?, ?)")) {
                insertLegacyRow(pstmt, "baidu", true, 1);
                insertLegacyRow(pstmt, "youdao", true, 2);
                insertLegacyRow(pstmt, "llm", false, 3);
            }
        }
    }

    private void insertLegacyRow(PreparedStatement pstmt, String apiName, boolean enabled, int priority) throws SQLException {
        pstmt.setString(1, apiName);
        pstmt.setBoolean(2, enabled);
        pstmt.setInt(3, priority);
        pstmt.executeUpdate();
    }
}
