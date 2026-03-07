package com.upfanyi.api;

import com.upfanyi.dao.ApiConfigDao;
import com.upfanyi.model.ApiConfig;
import com.upfanyi.model.ApiConfigField;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

import java.util.HashMap;
import java.util.List;
import java.util.Map;

/**
 * 翻译API工厂类
 * 用于创建和管理不同的翻译API实例
 *
 * @author UP翻译团队
 * @version 1.0.0
 */
public class ApiFactory {

    private static final Logger logger = LoggerFactory.getLogger(ApiFactory.class);

    private static final ApiFactory instance = new ApiFactory();

    private final Map<String, TranslateApi> apis;
    private ApiConfigDao apiConfigDao;

    private ApiFactory() {
        apis = new HashMap<>();
        // 注册默认的API实现
        registerDefaultApis();
    }

    public static ApiFactory getInstance() {
        return instance;
    }

    /**
     * 注册默认的API实现
     */
    private void registerDefaultApis() {
        apis.put("baidu", new BaiduTranslateApi());
        apis.put("youdao", new YoudaoTranslateApi());
        apis.put("tencent", new TencentTranslateApi());
        apis.put("aliyun", new AliyunTranslateApi());
        apis.put("deepl", new DeepLTranslateApi());
        apis.put("google", new GoogleTranslateApi());
        apis.put("microsoft", new MicrosoftTranslateApi());
        apis.put("llm", new LLMTranslateApi());
        logger.info("已注册默认翻译API: {}", apis.keySet());
    }

    /**
     * 设置API配置DAO
     */
    public void setApiConfigDao(ApiConfigDao apiConfigDao) {
        this.apiConfigDao = apiConfigDao;
        loadApiConfigurations();
    }

    /**
     * 从数据库加载API配置
     */
    private void loadApiConfigurations() {
        if (apiConfigDao == null) {
            logger.warn("API配置DAO未设置，跳过加载配置");
            return;
        }

        try {
            List<ApiConfig> configs = apiConfigDao.findAll();
            for (ApiConfig config : configs) {
                configureApi(config);
            }
            logger.info("已加载 {} 个API配置", configs.size());
        } catch (Exception e) {
            logger.error("加载API配置失败", e);
        }
    }

    /**
     * 配置API
     */
    private void configureApi(ApiConfig config) {
        TranslateApi api = apis.get(config.getApiName());
        if (api == null) {
            logger.warn("未知的API: {}", config.getApiName());
            return;
        }

        // 使用新的接口方法配置API
        for (ApiConfigField field : api.getConfigFields()) {
            String value = null;
            String fieldName = field.getFieldName();

            // 映射key字段
            if ("apiKey".equals(fieldName) || "appKey".equals(fieldName) || "appId".equals(fieldName)
                    || "secretId".equals(fieldName) || "accessKeyId".equals(fieldName)
                    || "authKey".equals(fieldName) || "subscriptionKey".equals(fieldName)) {
                value = config.getApiKey();
            }
            // 映射secret字段
            else if ("apiSecret".equals(fieldName) || "secretKey".equals(fieldName) || "appSecret".equals(fieldName)
                    || "accessKeySecret".equals(fieldName)) {
                value = config.getApiSecret();
            }
            // 其他字段从extraConfig获取
            else if (config.getExtraConfig() != null && !config.getExtraConfig().isEmpty()) {
                value = extractFromExtraConfig(config.getExtraConfig(), fieldName);
            }

            if (value != null) {
                api.setConfigValue(fieldName, value);
            }
        }

        logger.info("已配置API: {}", config.getApiName());
    }

    /**
     * 从extraConfig JSON中提取字段值
     */
    private String extractFromExtraConfig(String extraConfig, String fieldName) {
        String key = "\"" + fieldName + "\":";
        int idx = extraConfig.indexOf(key);
        if (idx >= 0) {
            int start = idx + key.length();
            while (start < extraConfig.length() && (extraConfig.charAt(start) == ' ' || extraConfig.charAt(start) == '"')) {
                start++;
            }
            int end = start;
            while (end < extraConfig.length() && extraConfig.charAt(end) != '"' && extraConfig.charAt(end) != ',' && extraConfig.charAt(end) != '}') {
                end++;
            }
            return extraConfig.substring(start, end).replace("\"", "").trim();
        }
        return null;
    }

    /**
     * 获取API实例
     *
     * @param apiName API名称
     * @return 翻译API实例
     */
    public TranslateApi getApi(String apiName) {
        TranslateApi api = apis.get(apiName);
        if (api == null) {
            logger.warn("未找到API: {}, 返回默认API", apiName);
            return apis.get("baidu");
        }
        return api;
    }

    /**
     * 获取所有已注册的API名称
     *
     * @return API名称列表
     */
    public List<String> getRegisteredApis() {
        return List.copyOf(apis.keySet());
    }

    /**
     * 检查API是否已配置
     *
     * @param apiName API名称
     * @return true表示已配置
     */
    public boolean isApiConfigured(String apiName) {
        TranslateApi api = apis.get(apiName);
        return api != null && api.isConfigured();
    }

    /**
     * 获取默认API
     *
     * @return 默认翻译API
     */
    public TranslateApi getDefaultApi() {
        // 优先返回已配置的API
        for (TranslateApi api : apis.values()) {
            if (api.isConfigured()) {
                return api;
            }
        }
        // 如果都没有配置，返回百度翻译
        return apis.get("baidu");
    }
}
