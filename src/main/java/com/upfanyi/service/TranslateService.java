package com.upfanyi.service;

import com.upfanyi.api.ApiFactory;
import com.upfanyi.api.TranslateApi;
import com.upfanyi.config.AppConfig;
import com.upfanyi.dao.HistoryDao;
import com.upfanyi.model.HistoryRecord;
import com.upfanyi.model.Translation;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

import java.util.List;

/**
 * 翻译服务
 *
 * @author UP翻译团队
 * @version 1.0.0
 */
public class TranslateService {

    private static final Logger logger = LoggerFactory.getLogger(TranslateService.class);

    private static final TranslateService instance = new TranslateService();

    private final ApiFactory apiFactory;
    private final HistoryDao historyDao;
    private final AppConfig appConfig;

    private TranslateService() {
        this.apiFactory = ApiFactory.getInstance();
        this.historyDao = new HistoryDao();
        this.appConfig = AppConfig.getInstance();
    }

    public static TranslateService getInstance() {
        return instance;
    }

    /**
     * 翻译文本
     *
     * @param text     要翻译的文本
     * @param fromLang 源语言
     * @param toLang   目标语言
     * @return 翻译结果
     * @throws Exception 翻译异常
     */
    public Translation translate(String text, String fromLang, String toLang) throws Exception {
        String preferredApiName = appConfig.getDefaultApi();
        TranslateApi preferredApi = apiFactory.getEnabledApi(preferredApiName);
        if (preferredApi != null && preferredApi.isConfigured()) {
            return translate(text, fromLang, toLang, preferredApiName);
        }

        TranslateApi fallbackApi = apiFactory.getDefaultApi();
        if (fallbackApi == null) {
            throw new IllegalStateException("没有启用的翻译API");
        }

        return translate(text, fromLang, toLang, fallbackApi.getApiName());
    }

    /**
     * 翻译文本（指定API）
     *
     * @param text     要翻译的文本
     * @param fromLang 源语言
     * @param toLang   目标语言
     * @param apiName  API名称
     * @return 翻译结果
     * @throws Exception 翻译异常
     */
    public Translation translate(String text, String fromLang, String toLang, String apiName) throws Exception {
        logger.info("翻译请求: {} -> {}, API: {}, 文本长度: {}", fromLang, toLang, apiName, text.length());

        TranslateApi api = apiFactory.getApi(apiName);
        if (api == null) {
            throw new IllegalArgumentException("未知的翻译API: " + apiName);
        }

        if (!apiFactory.isApiEnabled(apiName)) {
            throw new IllegalStateException("翻译API已禁用: " + api.getDisplayName());
        }

        if (!api.isConfigured()) {
            throw new IllegalStateException("翻译API未配置: " + api.getDisplayName());
        }

        // 执行翻译
        Translation translation = api.translate(text, fromLang, toLang);

        // 保存历史记录
        saveHistory(translation);

        logger.info("翻译成功: {} -> {}", text.substring(0, Math.min(50, text.length())),
                translation.getTranslatedText().substring(0, Math.min(50, translation.getTranslatedText().length())));

        return translation;
    }

    /**
     * 自动检测语言并翻译
     *
     * @param text   要翻译的文本
     * @param toLang 目标语言
     * @return 翻译结果
     * @throws Exception 翻译异常
     */
    public Translation translateAuto(String text, String toLang) throws Exception {
        return translate(text, "auto", toLang);
    }

    /**
     * 获取所有支持的翻译API
     */
    public List<String> getAvailableApis() {
        return apiFactory.getAvailableApis();
    }

    /**
     * 获取API显示名称
     */
    public String getApiDisplayName(String apiName) {
        TranslateApi api = apiFactory.getApi(apiName);
        return api != null ? api.getDisplayName() : apiName;
    }

    /**
     * 检查API是否已配置
     */
    public boolean isApiConfigured(String apiName) {
        return apiFactory.isApiConfigured(apiName);
    }

    /**
     * 获取API支持的语言列表
     */
    public List<String> getSupportedLanguages(String apiName) {
        TranslateApi api = apiFactory.getApi(apiName);
        return api != null ? api.getSupportedLanguages() : List.of();
    }

    /**
     * 保存翻译历史
     */
    private void saveHistory(Translation translation) {
        try {
            HistoryRecord record = new HistoryRecord(
                    translation.getSourceText(),
                    translation.getTranslatedText(),
                    translation.getSourceLang(),
                    translation.getTargetLang(),
                    translation.getApiUsed()
            );
            historyDao.save(record);
        } catch (Exception e) {
            logger.error("保存翻译历史失败", e);
        }
    }
}
