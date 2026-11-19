package com.upfanyi.service;

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
import java.util.List;

import static org.junit.jupiter.api.Assertions.assertEquals;

class HistoryServiceTest {

    @TempDir
    Path tempDir;

    private final HistoryService historyService = HistoryService.getInstance();

    @BeforeEach
    void setUp() {
        DatabaseConfig.getInstance().init(tempDir.resolve("history-service-test.db").toString());
    }

    @AfterEach
    void tearDown() {
        DatabaseConfig.getInstance().close();
    }

    @Test
    void getHistoryPageUsesLimitAndOffset() throws SQLException {
        insertHistory("first", "一", "2026-06-07T01:00:00");
        insertHistory("second", "二", "2026-06-07T02:00:00");
        insertHistory("third", "三", "2026-06-07T03:00:00");

        List<HistoryRecord> records = historyService.getHistoryPage(2, 1);

        assertEquals(List.of("second", "first"), records.stream().map(HistoryRecord::getSourceText).toList());
    }

    @Test
    void emptySearchUsesPagedHistory() throws SQLException {
        insertHistory("first", "一", "2026-06-07T01:00:00");
        insertHistory("second", "二", "2026-06-07T02:00:00");
        insertHistory("third", "三", "2026-06-07T03:00:00");

        List<HistoryRecord> records = historyService.searchHistory(" ", 2, 0);

        assertEquals(List.of("third", "second"), records.stream().map(HistoryRecord::getSourceText).toList());
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
