package com.upfanyi.dao;

import com.upfanyi.config.DatabaseConfig;
import com.upfanyi.model.HistoryRecord;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

import java.sql.*;
import java.time.LocalDateTime;
import java.util.ArrayList;
import java.util.List;

/**
 * 历史记录数据访问对象
 *
 * @author UP翻译团队
 * @version 1.0.0
 */
public class HistoryDao {

    private static final Logger logger = LoggerFactory.getLogger(HistoryDao.class);

    /**
     * 保存历史记录
     */
    public void save(HistoryRecord record) {
        String sql = "INSERT INTO history (source_text, translated_text, source_lang, target_lang, api_used, create_time) VALUES (?, ?, ?, ?, ?, ?)";

        try (Connection conn = DatabaseConfig.getInstance().getConnection();
             PreparedStatement pstmt = conn.prepareStatement(sql)) {

            pstmt.setString(1, record.getSourceText());
            pstmt.setString(2, record.getTranslatedText());
            pstmt.setString(3, record.getSourceLang());
            pstmt.setString(4, record.getTargetLang());
            pstmt.setString(5, record.getApiUsed());
            pstmt.setString(6, record.getCreateTime().toString());

            pstmt.executeUpdate();
            logger.debug("保存历史记录成功: {}", record.getSourceText());
        } catch (SQLException e) {
            logger.error("保存历史记录失败", e);
        }
    }

    /**
     * 查询所有历史记录
     */
    public List<HistoryRecord> findAll() {
        List<HistoryRecord> records = new ArrayList<>();
        String sql = "SELECT * FROM history ORDER BY create_time DESC";

        try (Connection conn = DatabaseConfig.getInstance().getConnection();
             Statement stmt = conn.createStatement();
             ResultSet rs = stmt.executeQuery(sql)) {

            while (rs.next()) {
                records.add(mapResultSetToRecord(rs));
            }
        } catch (SQLException e) {
            logger.error("查询历史记录失败", e);
        }

        return records;
    }

    /**
     * 根据关键词搜索历史记录
     */
    public List<HistoryRecord> search(String keyword) {
        List<HistoryRecord> records = new ArrayList<>();
        String sql = "SELECT * FROM history WHERE source_text LIKE ? OR translated_text LIKE ? ORDER BY create_time DESC";

        try (Connection conn = DatabaseConfig.getInstance().getConnection();
             PreparedStatement pstmt = conn.prepareStatement(sql)) {

            String pattern = "%" + keyword + "%";
            pstmt.setString(1, pattern);
            pstmt.setString(2, pattern);

            try (ResultSet rs = pstmt.executeQuery()) {
                while (rs.next()) {
                    records.add(mapResultSetToRecord(rs));
                }
            }
        } catch (SQLException e) {
            logger.error("搜索历史记录失败", e);
        }

        return records;
    }

    /**
     * 根据ID删除历史记录
     */
    public void deleteById(Long id) {
        String sql = "DELETE FROM history WHERE id = ?";

        try (Connection conn = DatabaseConfig.getInstance().getConnection();
             PreparedStatement pstmt = conn.prepareStatement(sql)) {

            pstmt.setLong(1, id);
            pstmt.executeUpdate();
            logger.debug("删除历史记录成功: {}", id);
        } catch (SQLException e) {
            logger.error("删除历史记录失败", e);
        }
    }

    /**
     * 清空所有历史记录
     */
    public void deleteAll() {
        String sql = "DELETE FROM history";

        try (Connection conn = DatabaseConfig.getInstance().getConnection();
             Statement stmt = conn.createStatement()) {

            stmt.executeUpdate(sql);
            logger.info("清空历史记录成功");
        } catch (SQLException e) {
            logger.error("清空历史记录失败", e);
        }
    }

    /**
     * 获取历史记录总数
     */
    public int count() {
        String sql = "SELECT COUNT(*) FROM history";

        try (Connection conn = DatabaseConfig.getInstance().getConnection();
             Statement stmt = conn.createStatement();
             ResultSet rs = stmt.executeQuery(sql)) {

            if (rs.next()) {
                return rs.getInt(1);
            }
        } catch (SQLException e) {
            logger.error("获取历史记录数量失败", e);
        }

        return 0;
    }

    /**
     * 将ResultSet映射为HistoryRecord对象
     */
    private HistoryRecord mapResultSetToRecord(ResultSet rs) throws SQLException {
        HistoryRecord record = new HistoryRecord();
        record.setId(rs.getLong("id"));
        record.setSourceText(rs.getString("source_text"));
        record.setTranslatedText(rs.getString("translated_text"));
        record.setSourceLang(rs.getString("source_lang"));
        record.setTargetLang(rs.getString("target_lang"));
        record.setApiUsed(rs.getString("api_used"));

        String createTimeStr = rs.getString("create_time");
        if (createTimeStr != null) {
            record.setCreateTime(LocalDateTime.parse(createTimeStr));
        }

        return record;
    }
}
