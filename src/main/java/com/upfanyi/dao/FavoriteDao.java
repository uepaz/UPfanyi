package com.upfanyi.dao;

import com.upfanyi.config.DatabaseConfig;
import com.upfanyi.model.FavoriteWord;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

import java.sql.*;
import java.time.LocalDateTime;
import java.util.ArrayList;
import java.util.List;

/**
 * 收藏/生词本数据访问对象
 *
 * @author UP翻译团队
 * @version 1.0.0
 */
public class FavoriteDao {

    private static final Logger logger = LoggerFactory.getLogger(FavoriteDao.class);

    /**
     * 保存收藏
     */
    public void save(FavoriteWord word) {
        String sql = "INSERT OR REPLACE INTO favorites (word, translation, note, create_time) VALUES (?, ?, ?, ?)";

        try (Connection conn = DatabaseConfig.getInstance().getConnection();
             PreparedStatement pstmt = conn.prepareStatement(sql)) {

            pstmt.setString(1, word.getWord());
            pstmt.setString(2, word.getTranslation());
            pstmt.setString(3, word.getNote());
            pstmt.setString(4, word.getCreateTime().toString());

            pstmt.executeUpdate();
            logger.debug("保存收藏成功: {}", word.getWord());
        } catch (SQLException e) {
            logger.error("保存收藏失败", e);
        }
    }

    /**
     * 查询所有收藏
     */
    public List<FavoriteWord> findAll() {
        List<FavoriteWord> words = new ArrayList<>();
        String sql = "SELECT * FROM favorites ORDER BY create_time DESC";

        try (Connection conn = DatabaseConfig.getInstance().getConnection();
             Statement stmt = conn.createStatement();
             ResultSet rs = stmt.executeQuery(sql)) {

            while (rs.next()) {
                words.add(mapResultSetToWord(rs));
            }
        } catch (SQLException e) {
            logger.error("查询收藏失败", e);
        }

        return words;
    }

    /**
     * 根据单词查找
     */
    public FavoriteWord findByWord(String word) {
        String sql = "SELECT * FROM favorites WHERE word = ?";

        try (Connection conn = DatabaseConfig.getInstance().getConnection();
             PreparedStatement pstmt = conn.prepareStatement(sql)) {

            pstmt.setString(1, word);

            try (ResultSet rs = pstmt.executeQuery()) {
                if (rs.next()) {
                    return mapResultSetToWord(rs);
                }
            }
        } catch (SQLException e) {
            logger.error("查找单词失败", e);
        }

        return null;
    }

    /**
     * 根据关键词搜索
     */
    public List<FavoriteWord> search(String keyword) {
        List<FavoriteWord> words = new ArrayList<>();
        String sql = "SELECT * FROM favorites WHERE word LIKE ? OR translation LIKE ? OR note LIKE ? ORDER BY create_time DESC";

        try (Connection conn = DatabaseConfig.getInstance().getConnection();
             PreparedStatement pstmt = conn.prepareStatement(sql)) {

            String pattern = "%" + keyword + "%";
            pstmt.setString(1, pattern);
            pstmt.setString(2, pattern);
            pstmt.setString(3, pattern);

            try (ResultSet rs = pstmt.executeQuery()) {
                while (rs.next()) {
                    words.add(mapResultSetToWord(rs));
                }
            }
        } catch (SQLException e) {
            logger.error("搜索收藏失败", e);
        }

        return words;
    }

    /**
     * 根据ID删除
     */
    public void deleteById(Long id) {
        String sql = "DELETE FROM favorites WHERE id = ?";

        try (Connection conn = DatabaseConfig.getInstance().getConnection();
             PreparedStatement pstmt = conn.prepareStatement(sql)) {

            pstmt.setLong(1, id);
            pstmt.executeUpdate();
            logger.debug("删除收藏成功: {}", id);
        } catch (SQLException e) {
            logger.error("删除收藏失败", e);
        }
    }

    /**
     * 根据单词删除
     */
    public void deleteByWord(String word) {
        String sql = "DELETE FROM favorites WHERE word = ?";

        try (Connection conn = DatabaseConfig.getInstance().getConnection();
             PreparedStatement pstmt = conn.prepareStatement(sql)) {

            pstmt.setString(1, word);
            pstmt.executeUpdate();
            logger.debug("删除收藏成功: {}", word);
        } catch (SQLException e) {
            logger.error("删除收藏失败", e);
        }
    }

    /**
     * 检查单词是否已收藏
     */
    public boolean exists(String word) {
        String sql = "SELECT COUNT(*) FROM favorites WHERE word = ?";

        try (Connection conn = DatabaseConfig.getInstance().getConnection();
             PreparedStatement pstmt = conn.prepareStatement(sql)) {

            pstmt.setString(1, word);

            try (ResultSet rs = pstmt.executeQuery()) {
                if (rs.next()) {
                    return rs.getInt(1) > 0;
                }
            }
        } catch (SQLException e) {
            logger.error("检查收藏存在失败", e);
        }

        return false;
    }

    /**
     * 获取收藏总数
     */
    public int count() {
        String sql = "SELECT COUNT(*) FROM favorites";

        try (Connection conn = DatabaseConfig.getInstance().getConnection();
             Statement stmt = conn.createStatement();
             ResultSet rs = stmt.executeQuery(sql)) {

            if (rs.next()) {
                return rs.getInt(1);
            }
        } catch (SQLException e) {
            logger.error("获取收藏数量失败", e);
        }

        return 0;
    }

    /**
     * 将ResultSet映射为FavoriteWord对象
     */
    private FavoriteWord mapResultSetToWord(ResultSet rs) throws SQLException {
        FavoriteWord word = new FavoriteWord();
        word.setId(rs.getLong("id"));
        word.setWord(rs.getString("word"));
        word.setTranslation(rs.getString("translation"));
        word.setNote(rs.getString("note"));

        String createTimeStr = rs.getString("create_time");
        if (createTimeStr != null) {
            word.setCreateTime(LocalDateTime.parse(createTimeStr));
        }

        return word;
    }
}
