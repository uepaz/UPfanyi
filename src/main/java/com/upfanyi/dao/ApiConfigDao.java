package com.upfanyi.dao;

import com.upfanyi.config.DatabaseConfig;
import com.upfanyi.model.ApiConfig;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

import java.sql.*;
import java.util.ArrayList;
import java.util.List;

/**
 * API配置数据访问对象
 *
 * @author UP翻译团队
 * @version 1.0.0
 */
public class ApiConfigDao {

    private static final Logger logger = LoggerFactory.getLogger(ApiConfigDao.class);

    /**
     * 保存或更新API配置
     */
    public void save(ApiConfig config) {
        String sql = "INSERT OR REPLACE INTO api_config (api_name, api_key, api_secret, endpoint, model_name, extra_config, is_enabled, priority) VALUES (?, ?, ?, ?, ?, ?, ?, ?)";

        try (Connection conn = DatabaseConfig.getInstance().getConnection();
             PreparedStatement pstmt = conn.prepareStatement(sql, Statement.RETURN_GENERATED_KEYS)) {

            pstmt.setString(1, config.getApiName());
            pstmt.setString(2, config.getApiKey());
            pstmt.setString(3, config.getApiSecret());
            pstmt.setString(4, config.getEndpoint());
            pstmt.setString(5, config.getModelName());
            pstmt.setString(6, config.getExtraConfig());
            pstmt.setBoolean(7, config.isEnabled());
            pstmt.setInt(8, config.getPriority());

            pstmt.executeUpdate();

            try (ResultSet rs = pstmt.getGeneratedKeys()) {
                if (rs.next()) {
                    config.setId(rs.getLong(1));
                }
            }

            logger.debug("保存API配置成功: {}", config.getApiName());
        } catch (SQLException e) {
            logger.error("保存API配置失败", e);
        }
    }

    /**
     * 查询所有API配置
     */
    public List<ApiConfig> findAll() {
        List<ApiConfig> configs = new ArrayList<>();
        String sql = "SELECT * FROM api_config ORDER BY priority ASC";

        try (Connection conn = DatabaseConfig.getInstance().getConnection();
             Statement stmt = conn.createStatement();
             ResultSet rs = stmt.executeQuery(sql)) {

            while (rs.next()) {
                configs.add(mapResultSetToConfig(rs));
            }
        } catch (SQLException e) {
            logger.error("查询API配置失败", e);
        }

        return configs;
    }

    /**
     * 根据名称查询API配置
     */
    public ApiConfig findByName(String apiName) {
        String sql = "SELECT * FROM api_config WHERE api_name = ?";

        try (Connection conn = DatabaseConfig.getInstance().getConnection();
             PreparedStatement pstmt = conn.prepareStatement(sql)) {

            pstmt.setString(1, apiName);

            try (ResultSet rs = pstmt.executeQuery()) {
                if (rs.next()) {
                    return mapResultSetToConfig(rs);
                }
            }
        } catch (SQLException e) {
            logger.error("查询API配置失败", e);
        }

        return null;
    }

    /**
     * 查询已启用的API配置
     */
    public List<ApiConfig> findEnabled() {
        List<ApiConfig> configs = new ArrayList<>();
        String sql = "SELECT * FROM api_config WHERE is_enabled = TRUE ORDER BY priority ASC";

        try (Connection conn = DatabaseConfig.getInstance().getConnection();
             Statement stmt = conn.createStatement();
             ResultSet rs = stmt.executeQuery(sql)) {

            while (rs.next()) {
                configs.add(mapResultSetToConfig(rs));
            }
        } catch (SQLException e) {
            logger.error("查询已启用API配置失败", e);
        }

        return configs;
    }

    /**
     * 删除API配置
     */
    public void deleteByName(String apiName) {
        String sql = "DELETE FROM api_config WHERE api_name = ?";

        try (Connection conn = DatabaseConfig.getInstance().getConnection();
             PreparedStatement pstmt = conn.prepareStatement(sql)) {

            pstmt.setString(1, apiName);
            pstmt.executeUpdate();
            logger.debug("删除API配置成功: {}", apiName);
        } catch (SQLException e) {
            logger.error("删除API配置失败", e);
        }
    }

    /**
     * 将ResultSet映射为ApiConfig对象
     */
    private ApiConfig mapResultSetToConfig(ResultSet rs) throws SQLException {
        ApiConfig config = new ApiConfig();
        config.setId(rs.getLong("id"));
        config.setApiName(rs.getString("api_name"));
        config.setApiKey(rs.getString("api_key"));
        config.setApiSecret(rs.getString("api_secret"));
        config.setEndpoint(rs.getString("endpoint"));
        config.setModelName(rs.getString("model_name"));
        config.setExtraConfig(rs.getString("extra_config"));
        config.setEnabled(rs.getBoolean("is_enabled"));
        config.setPriority(rs.getInt("priority"));
        return config;
    }
}
