package com.upfanyi.dao;

import com.upfanyi.config.DatabaseConfig;
import com.upfanyi.model.FavoriteWord;
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

class FavoriteDaoTest {

    @TempDir
    Path tempDir;

    private final FavoriteDao favoriteDao = new FavoriteDao();

    @BeforeEach
    void setUp() {
        DatabaseConfig.getInstance().init(tempDir.resolve("favorite-test.db").toString());
    }

    @AfterEach
    void tearDown() {
        DatabaseConfig.getInstance().close();
    }

    @Test
    void readsSqliteCurrentTimestampFormat() throws SQLException {
        insertFavorite("hello", "你好", "2026-06-07 01:02:03");

        List<FavoriteWord> words = favoriteDao.findAll();

        assertEquals(LocalDateTime.of(2026, 6, 7, 1, 2, 3), words.get(0).getCreateTime());
    }

    @Test
    void findPageLimitsAndOffsetsNewestRecords() throws SQLException {
        insertFavorite("first", "一", "2026-06-07T01:00:00");
        insertFavorite("second", "二", "2026-06-07T02:00:00");
        insertFavorite("third", "三", "2026-06-07T03:00:00");

        List<FavoriteWord> words = favoriteDao.findPage(2, 1);

        assertEquals(List.of("second", "first"), words.stream().map(FavoriteWord::getWord).toList());
    }

    @Test
    void searchCanLimitResults() throws SQLException {
        insertFavorite("alpha one", "一", "2026-06-07T01:00:00");
        insertFavorite("alpha two", "二", "2026-06-07T02:00:00");
        insertFavorite("alpha three", "三", "2026-06-07T03:00:00");

        List<FavoriteWord> words = favoriteDao.search("alpha", 2, 0);

        assertEquals(List.of("alpha three", "alpha two"), words.stream().map(FavoriteWord::getWord).toList());
    }

    private void insertFavorite(String word, String translation, String createTime) throws SQLException {
        String sql = """
                INSERT INTO favorites (word, translation, note, create_time)
                VALUES (?, ?, NULL, ?)
                """;

        try (Connection conn = DatabaseConfig.getInstance().getConnection();
             PreparedStatement pstmt = conn.prepareStatement(sql)) {
            pstmt.setString(1, word);
            pstmt.setString(2, translation);
            pstmt.setString(3, createTime);
            pstmt.executeUpdate();
        }
    }
}
