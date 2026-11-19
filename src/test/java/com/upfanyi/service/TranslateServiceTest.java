package com.upfanyi.service;

import com.upfanyi.api.ApiFactory;
import com.upfanyi.config.DatabaseConfig;
import com.upfanyi.dao.ApiConfigDao;
import com.upfanyi.model.ApiConfig;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;

import java.nio.file.Path;
import java.util.List;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;

class TranslateServiceTest {

    @TempDir
    Path tempDir;

    @AfterEach
    void tearDown() {
        DatabaseConfig.getInstance().close();
    }

    @Test
    void availableApisComeFromEnabledFactoryConfig() {
        DatabaseConfig.getInstance().init(tempDir.resolve("translate-service-test.db").toString());
        ApiConfigDao dao = new ApiConfigDao();
        disableAllApis(dao);
        dao.save(apiConfig("baidu", true, 20));
        dao.save(apiConfig("youdao", true, 10));
        ApiFactory.getInstance().setApiConfigDao(dao);

        assertEquals(List.of("youdao", "baidu"), TranslateService.getInstance().getAvailableApis());
    }

    @Test
    void explicitDisabledApiFailsBeforeTranslationRequest() {
        DatabaseConfig.getInstance().init(tempDir.resolve("disabled-api-test.db").toString());
        ApiConfigDao dao = new ApiConfigDao();
        disableAllApis(dao);
        dao.save(apiConfig("baidu", false, 1));
        ApiFactory.getInstance().setApiConfigDao(dao);

        IllegalStateException error = assertThrows(
                IllegalStateException.class,
                () -> TranslateService.getInstance().translate("hello", "en", "zh", "baidu")
        );

        assertEquals("翻译API已禁用: 百度翻译", error.getMessage());
    }

    private void disableAllApis(ApiConfigDao dao) {
        int priority = 100;
        for (String apiName : ApiFactory.getInstance().getRegisteredApis()) {
            dao.save(apiConfig(apiName, false, priority++));
        }
    }

    private ApiConfig apiConfig(String apiName, boolean enabled, int priority) {
        ApiConfig config = new ApiConfig();
        config.setApiName(apiName);
        config.setEnabled(enabled);
        config.setPriority(priority);
        return config;
    }
}
