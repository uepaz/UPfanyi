package com.upfanyi.api;

import com.upfanyi.model.ApiType;
import org.junit.jupiter.api.Test;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

class LLMTranslateApiTest {

    @Test
    void defaultOpenAiEndpointWithoutApiKeyIsNotConfigured() {
        LLMTranslateApi api = new LLMTranslateApi();

        assertFalse(api.isConfigured());
    }

    @Test
    void defaultOpenAiEndpointWithApiKeyIsConfigured() {
        LLMTranslateApi api = new LLMTranslateApi();

        api.setConfigValue("apiKey", "test-key");

        assertTrue(api.isConfigured());
    }

    @Test
    void ollamaConfigurationDoesNotRequireApiKey() {
        LLMTranslateApi api = new LLMTranslateApi();

        api.setConfigValue("apiType", "ollama");

        assertEquals(ApiType.LLM_OLLAMA, api.getApiType());
        assertTrue(api.isConfigured());
    }

    @Test
    void customOpenAiCompatibleEndpointCanOmitApiKey() {
        LLMTranslateApi api = new LLMTranslateApi();

        api.setConfigValue("endpoint", "http://localhost:8080/v1/chat/completions");

        assertTrue(api.isConfigured());
    }
}
