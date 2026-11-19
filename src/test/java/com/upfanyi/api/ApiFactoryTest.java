package com.upfanyi.api;

import com.upfanyi.config.DatabaseConfig;
import com.upfanyi.dao.ApiConfigDao;
import com.upfanyi.model.ApiConfig;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;

import java.nio.file.Path;
import java.util.List;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNull;

class ApiFactoryTest {

    @TempDir
    Path tempDir;

    @AfterEach
    void tearDown() {
        DatabaseConfig.getInstance().close();
    }

    @Test
    void unknownApiDoesNotFallbackToBaidu() {
        assertNull(ApiFactory.getInstance().getApi("missing-api"));
    }

    @Test
    void registeredApisKeepDisplayOrder() {
        assertEquals(
                List.of("baidu", "youdao", "tencent", "aliyun", "deepl", "google", "microsoft", "llm"),
                ApiFactory.getInstance().getRegisteredApis()
        );
    }

    @Test
    void availableApisUseEnabledDatabasePriority() {
        ApiFactory factory = loadFactoryWithConfigs(
                apiConfig("baidu", true, 30, "baidu-app", "baidu-secret"),
                apiConfig("youdao", true, 10, "youdao-app", "youdao-secret")
        );

        assertEquals(List.of("youdao", "baidu"), factory.getAvailableApis());
    }

    @Test
    void defaultApiSkipsDisabledAndUnconfiguredApis() {
        ApiFactory factory = loadFactoryWithConfigs(
                apiConfig("baidu", false, 1, "baidu-app", "baidu-secret"),
                apiConfig("tencent", true, 2, null, null),
                apiConfig("youdao", true, 3, "youdao-app", "youdao-secret")
        );

        assertEquals("youdao", factory.getDefaultApi().getApiName());
    }

    @Test
    void reloadingEmptyConfigClearsPreviousCredentials() {
        ApiFactory factory = loadFactoryWithConfigs(
                apiConfig("baidu", true, 1, "baidu-app", "baidu-secret")
        );
        assertEquals("baidu", factory.getDefaultApi().getApiName());

        factory = loadFactoryWithConfigs(
                apiConfig("baidu", true, 1, null, null)
        );

        assertFalse(factory.isApiConfigured("baidu"));
    }

    private ApiFactory loadFactoryWithConfigs(ApiConfig... configs) {
        DatabaseConfig.getInstance().init(tempDir.resolve("api-factory-test.db").toString());
        ApiConfigDao dao = new ApiConfigDao();
        disableAllApis(dao);
        for (ApiConfig config : configs) {
            dao.save(config);
        }

        ApiFactory factory = ApiFactory.getInstance();
        factory.setApiConfigDao(dao);
        return factory;
    }

    private void disableAllApis(ApiConfigDao dao) {
        int priority = 100;
        for (String apiName : ApiFactory.getInstance().getRegisteredApis()) {
            dao.save(apiConfig(apiName, false, priority++, null, null));
        }
    }

    private ApiConfig apiConfig(String apiName, boolean enabled, int priority, String apiKey, String apiSecret) {
        ApiConfig config = new ApiConfig();
        config.setApiName(apiName);
        config.setEnabled(enabled);
        config.setPriority(priority);
        config.setApiKey(apiKey);
        config.setApiSecret(apiSecret);
        return config;
    }
}
