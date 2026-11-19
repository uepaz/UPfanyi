package com.upfanyi.service;

import com.upfanyi.dao.HistoryDao;
import com.upfanyi.model.HistoryRecord;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

import java.util.List;

/**
 * 历史记录服务
 *
 * @author UP翻译团队
 * @version 1.0.0
 */
public class HistoryService {

    private static final Logger logger = LoggerFactory.getLogger(HistoryService.class);

    private static final HistoryService instance = new HistoryService();

    private final HistoryDao historyDao;

    private HistoryService() {
        this.historyDao = new HistoryDao();
    }

    public static HistoryService getInstance() {
        return instance;
    }

    /**
     * 获取所有历史记录
     */
    public List<HistoryRecord> getAllHistory() {
        return historyDao.findAll();
    }

    /**
     * 分页获取历史记录
     */
    public List<HistoryRecord> getHistoryPage(int limit, int offset) {
        return historyDao.findPage(limit, offset);
    }

    /**
     * 搜索历史记录
     */
    public List<HistoryRecord> searchHistory(String keyword) {
        if (keyword == null || keyword.trim().isEmpty()) {
            return getAllHistory();
        }
        return historyDao.search(keyword.trim());
    }

    /**
     * 分页搜索历史记录
     */
    public List<HistoryRecord> searchHistory(String keyword, int limit, int offset) {
        if (keyword == null || keyword.trim().isEmpty()) {
            return getHistoryPage(limit, offset);
        }
        return historyDao.search(keyword.trim(), limit, offset);
    }

    /**
     * 删除历史记录
     */
    public void deleteHistory(Long id) {
        historyDao.deleteById(id);
    }

    /**
     * 清空所有历史记录
     */
    public void clearAllHistory() {
        historyDao.deleteAll();
        logger.info("已清空所有历史记录");
    }

    /**
     * 获取历史记录总数
     */
    public int getHistoryCount() {
        return historyDao.count();
    }
}
