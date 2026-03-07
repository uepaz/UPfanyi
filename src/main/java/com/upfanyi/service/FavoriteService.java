package com.upfanyi.service;

import com.upfanyi.dao.FavoriteDao;
import com.upfanyi.model.FavoriteWord;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

import java.util.List;

/**
 * 收藏/生词本服务
 *
 * @author UP翻译团队
 * @version 1.0.0
 */
public class FavoriteService {

    private static final Logger logger = LoggerFactory.getLogger(FavoriteService.class);

    private static final FavoriteService instance = new FavoriteService();

    private final FavoriteDao favoriteDao;

    private FavoriteService() {
        this.favoriteDao = new FavoriteDao();
    }

    public static FavoriteService getInstance() {
        return instance;
    }

    /**
     * 添加收藏
     */
    public void addFavorite(String word, String translation, String note) {
        FavoriteWord favoriteWord = new FavoriteWord(word, translation, note);
        favoriteDao.save(favoriteWord);
        logger.info("添加收藏: {}", word);
    }

    /**
     * 添加收藏（无备注）
     */
    public void addFavorite(String word, String translation) {
        addFavorite(word, translation, null);
    }

    /**
     * 删除收藏
     */
    public void removeFavorite(Long id) {
        favoriteDao.deleteById(id);
        logger.info("删除收藏: {}", id);
    }

    /**
     * 根据单词删除收藏
     */
    public void removeFavoriteByWord(String word) {
        favoriteDao.deleteByWord(word);
        logger.info("删除收藏: {}", word);
    }

    /**
     * 检查是否已收藏
     */
    public boolean isFavorited(String word) {
        return favoriteDao.exists(word);
    }

    /**
     * 获取所有收藏
     */
    public List<FavoriteWord> getAllFavorites() {
        return favoriteDao.findAll();
    }

    /**
     * 搜索收藏
     */
    public List<FavoriteWord> searchFavorites(String keyword) {
        if (keyword == null || keyword.trim().isEmpty()) {
            return getAllFavorites();
        }
        return favoriteDao.search(keyword.trim());
    }

    /**
     * 根据单词查找收藏
     */
    public FavoriteWord findByWord(String word) {
        return favoriteDao.findByWord(word);
    }

    /**
     * 获取收藏总数
     */
    public int getFavoriteCount() {
        return favoriteDao.count();
    }

    /**
     * 导出收藏为文本
     */
    public String exportFavorites() {
        List<FavoriteWord> favorites = getAllFavorites();
        StringBuilder sb = new StringBuilder();
        sb.append("=== UP翻译收藏导出 ===\n\n");

        for (FavoriteWord word : favorites) {
            sb.append(word.getWord()).append("\n");
            sb.append("翻译: ").append(word.getTranslation()).append("\n");
            if (word.getNote() != null && !word.getNote().isEmpty()) {
                sb.append("备注: ").append(word.getNote()).append("\n");
            }
            sb.append("添加时间: ").append(word.getCreateTime()).append("\n");
            sb.append("---\n");
        }

        sb.append("\n共 ").append(favorites.size()).append(" 条记录");
        return sb.toString();
    }
}
