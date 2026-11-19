package com.upfanyi.dao;

import com.upfanyi.config.DatabaseConfig;
import com.upfanyi.model.HistoryRecord;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;

import java.nio.file.Path;
import java.sql.Connection;
import java.sql.PreparedStatement;
import java.sql.SQLException;
import java.time.LocalDateTime;
import java.util.List;

import static org.junit.jupiter.api.Assertions.assertEquals;

class HistoryDaoTest {

    @TempDir
    Path tempDir;

    private final HistoryDao historyDao = new HistoryDao();

    @BeforeEach
    void setUp() {
        DatabaseConfig.getInstance().init(tempDir.resolve("history-test.db").toString());
    }

    @AfterEach
    void tearDown() {
        DatabaseConfig.getInstance().close();
    }

    @Test
    void readsSqliteCurrentTimestampFormat() throws SQLException {
        insertHistory("hello", "你好", "2026-06-07 01:02:03");

        List<HistoryRecord> records = historyDao.findAll();

        assertEquals(LocalDateTime.of(2026, 6, 7, 1, 2, 3), records.get(0).getCreateTime());
    }

    @Test
    void findPageLimitsAndOffsetsNewestRecords() throws SQLException {
        insertHistory("first", "一", "2026-06-07T01:00:00");
        insertHistory("second", "二", "2026-06-07T02:00:00");
        insertHistory("third", "三", "2026-06-07T03:00:00");

        List<HistoryRecord> records = historyDao.findPage(2, 1);

        assertEquals(List.of("second", "first"), records.stream().map(HistoryRecord::getSourceText).toList());
    }

    @Test
    void searchCanLimitResults() throws SQLException {
        insertHistory("alpha one", "一", "2026-06-07T01:00:00");
        insertHistory("alpha two", "二", "2026-06-07T02:00:00");
        insertHistory("alpha three", "三", "2026-06-07T03:00:00");

        List<HistoryRecord> records = historyDao.search("alpha", 2, 0);

        assertEquals(List.of("alpha three", "alpha two"), records.stream().map(HistoryRecord::getSourceText).toList());
    }

    private void insertHistory(String sourceText, String translatedText, String createTime) throws SQLException {
        String sql = """
                INSERT INTO history (source_text, translated_text, source_lang, target_lang, api_used, create_time)
                VALUES (?, ?, 'auto', 'zh', 'test', ?)
                """;

        try (Connection conn = DatabaseConfig.getInstance().getConnection();
             PreparedStatement pstmt = conn.prepareStatement(sql)) {
            pstmt.setString(1, sourceText);
            pstmt.setString(2, translatedText);
            pstmt.setString(3, createTime);
            pstmt.executeUpdate();
        }
    }
}
