package com.upfanyi.api;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import com.upfanyi.dao.ApiConfigDao;
import com.upfanyi.model.ApiConfig;
import com.upfanyi.model.ApiConfigField;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

import java.util.ArrayList;
import java.util.LinkedHashMap;
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
    private static final ObjectMapper objectMapper = new ObjectMapper();

    private final Map<String, TranslateApi> apis;
    private final Map<String, ApiConfig> apiConfigs;
    private ApiConfigDao apiConfigDao;

    private ApiFactory() {
        apis = new LinkedHashMap<>();
        apiConfigs = new LinkedHashMap<>();
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
            apiConfigs.clear();
            for (ApiConfig config : configs) {
                if (apis.containsKey(config.getApiName())) {
                    apiConfigs.put(config.getApiName(), config);
                }
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
            String fieldName = field.getFieldName();
            String value = getConfigValue(config, fieldName);

            api.setConfigValue(fieldName, value);
        }

        logger.info("已配置API: {}", config.getApiName());
    }

    /**
     * 从配置记录中读取字段值，优先使用新的extraConfig完整字段映射，再兼容旧列。
     */
    private String getConfigValue(ApiConfig config, String fieldName) {
        String value = extractFromExtraConfig(config.getExtraConfig(), fieldName);
        if (value != null) {
            return value;
        }

        if (isApiKeyField(fieldName)) {
            return config.getApiKey();
        }
        if (isApiSecretField(fieldName)) {
            return config.getApiSecret();
        }
        if ("endpoint".equals(fieldName)) {
            return config.getEndpoint();
        }
        if ("model".equals(fieldName) || "modelName".equals(fieldName)) {
            return config.getModelName();
        }

        return null;
    }

    /**
     * 从extraConfig JSON中提取字段值
     */
    private String extractFromExtraConfig(String extraConfig, String fieldName) {
        if (extraConfig == null || extraConfig.isEmpty()) {
            return null;
        }

        try {
            JsonNode root = objectMapper.readTree(extraConfig);
            JsonNode value = root.get(fieldName);
            return value != null && !value.isNull() ? value.asText() : null;
        } catch (Exception e) {
            logger.warn("解析API扩展配置失败: {}", e.getMessage());
        }

        return null;
    }

    private boolean isApiKeyField(String fieldName) {
        return "apiKey".equals(fieldName)
                || "appKey".equals(fieldName)
                || "appId".equals(fieldName)
                || "secretId".equals(fieldName)
                || "accessKeyId".equals(fieldName)
                || "authKey".equals(fieldName)
                || "subscriptionKey".equals(fieldName);
    }

    private boolean isApiSecretField(String fieldName) {
        return "apiSecret".equals(fieldName)
                || "secretKey".equals(fieldName)
                || "appSecret".equals(fieldName)
                || "accessKeySecret".equals(fieldName);
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
            logger.warn("未找到API: {}", apiName);
        }
        return api;
    }

    /**
     * 获取启用的API实例
     *
     * @param apiName API名称
     * @return 启用的翻译API实例，禁用或不存在时返回null
     */
    public TranslateApi getEnabledApi(String apiName) {
        if (!isApiEnabled(apiName)) {
            logger.warn("API已禁用: {}", apiName);
            return null;
        }
        return getApi(apiName);
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
     * 获取已启用的API名称，按数据库priority排序。
     *
     * @return 已启用API名称列表
     */
    public List<String> getAvailableApis() {
        return getApisByConfigurationOrder(true);
    }

    /**
     * 获取按配置排序的所有API名称。
     *
     * @return API名称列表
     */
    public List<String> getConfiguredApiOrder() {
        return getApisByConfigurationOrder(false);
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
     * 检查API是否已启用。缺少数据库配置的API按启用处理，以兼容旧数据库。
     *
     * @param apiName API名称
     * @return true表示已启用
     */
    public boolean isApiEnabled(String apiName) {
        if (!apis.containsKey(apiName)) {
            return false;
        }

        ApiConfig config = apiConfigs.get(apiName);
        return config == null || config.isEnabled();
    }

    /**
     * 获取默认API
     *
     * @return 默认翻译API
     */
    public TranslateApi getDefaultApi() {
        // 优先返回按priority排序后的已启用且已配置API
        for (String apiName : getAvailableApis()) {
            TranslateApi api = apis.get(apiName);
            if (api.isConfigured()) {
                return api;
            }
        }

        // 如果都没有配置，返回第一个启用API，方便UI提示用户补配置。
        List<String> availableApis = getAvailableApis();
        if (!availableApis.isEmpty()) {
            return apis.get(availableApis.get(0));
        }

        return null;
    }

    private List<String> getApisByConfigurationOrder(boolean enabledOnly) {
        List<String> apiNames = new ArrayList<>();

        for (String apiName : apiConfigs.keySet()) {
            if (apis.containsKey(apiName) && (!enabledOnly || isApiEnabled(apiName))) {
                apiNames.add(apiName);
            }
        }

        for (String apiName : apis.keySet()) {
            if (!apiConfigs.containsKey(apiName) && (!enabledOnly || isApiEnabled(apiName))) {
                apiNames.add(apiName);
            }
        }

        return List.copyOf(apiNames);
    }
}
