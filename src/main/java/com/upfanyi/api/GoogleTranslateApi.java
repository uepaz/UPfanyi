package com.upfanyi.api;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import com.upfanyi.model.ApiConfigField;
import com.upfanyi.model.Translation;
import okhttp3.OkHttpClient;
import okhttp3.Request;
import okhttp3.Response;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

import java.net.URLEncoder;
import java.nio.charset.StandardCharsets;
import java.util.Arrays;
import java.util.List;
import java.util.concurrent.TimeUnit;

/**
 * Google Translate API实现
 *
 * API文档: https://cloud.google.com/translate/docs
 * 需要Google Cloud API Key
 *
 * @author UP翻译团队
 * @version 1.0.0
 */
public class GoogleTranslateApi implements TranslateApi {

    private static final Logger logger = LoggerFactory.getLogger(GoogleTranslateApi.class);

    private static final String API_URL = "https://translation.googleapis.com/language/translate/v2";

    private final OkHttpClient httpClient;
    private final ObjectMapper objectMapper;

    private String apiKey;

    public GoogleTranslateApi() {
        this.httpClient = new OkHttpClient.Builder()
                .connectTimeout(10, TimeUnit.SECONDS)
                .readTimeout(30, TimeUnit.SECONDS)
                .writeTimeout(10, TimeUnit.SECONDS)
                .build();
        this.objectMapper = new ObjectMapper();
    }

    public GoogleTranslateApi(String apiKey) {
        this();
        this.apiKey = apiKey;
    }

    public void setApiKey(String apiKey) {
        this.apiKey = apiKey;
    }

    @Override
    public Translation translate(String text, String fromLang, String toLang) throws Exception {
        if (!isConfigured()) {
            throw new IllegalStateException("Google翻译API未配置，请设置API Key");
        }

        logger.debug("Google翻译: {} -> {}, 文本: {}", fromLang, toLang, text);

        // 构建请求URL
        String sourceLang = convertLanguageCode(fromLang);
        String targetLang = convertLanguageCode(toLang);

        StringBuilder urlBuilder = new StringBuilder(API_URL);
        urlBuilder.append("?key=").append(apiKey);
        urlBuilder.append("&q=").append(URLEncoder.encode(text, StandardCharsets.UTF_8));
        urlBuilder.append("&target=").append(targetLang);

        if (!"auto".equals(sourceLang)) {
            urlBuilder.append("&source=").append(sourceLang);
        }

        urlBuilder.append("&format=text");

        Request request = new Request.Builder()
                .url(urlBuilder.toString())
                .get()
                .build();

        try (Response response = httpClient.newCall(request).execute()) {
            if (!response.isSuccessful()) {
                String errorBody = response.body() != null ? response.body().string() : "Unknown error";
                throw new Exception("Google翻译请求失败: " + response.code() + " - " + errorBody);
            }

            String responseBody = response.body() != null ? response.body().string() : "";
            logger.debug("Google翻译响应: {}", responseBody);

            return parseResponse(responseBody, text, fromLang, toLang);
        }
    }

    private String convertLanguageCode(String code) {
        // Google Translate语言代码
        switch (code) {
            case "auto": return "auto";
            case "zh": return "zh-CN";
            case "en": return "en";
            case "ja": return "ja";
            case "ko": return "ko";
            case "fr": return "fr";
            case "de": return "de";
            case "es": return "es";
            case "ru": return "ru";
            case "pt": return "pt";
            case "it": return "it";
            case "vi": return "vi";
            case "th": return "th";
            case "ar": return "ar";
            case "tr": return "tr";
            case "id": return "id";
            case "ms": return "ms";
            case "nl": return "nl";
            case "pl": return "pl";
            default: return code;
        }
    }

    private Translation parseResponse(String responseBody, String sourceText, String fromLang, String toLang) throws Exception {
        JsonNode root = objectMapper.readTree(responseBody);

        JsonNode data = root.get("data");
        if (data == null) {
            // 检查错误
            if (root.has("error")) {
                JsonNode error = root.get("error");
                String message = error.has("message") ? error.get("message").asText() : "Unknown error";
                throw new Exception("Google翻译错误: " + message);
            }
            throw new Exception("Google翻译结果格式错误");
        }

        JsonNode translations = data.get("translations");
        if (translations == null || !translations.isArray() || translations.size() == 0) {
            throw new Exception("Google翻译结果为空");
        }

        StringBuilder translatedText = new StringBuilder();
        String detectedSourceLang = fromLang;

        for (JsonNode node : translations) {
            translatedText.append(node.get("translatedText").asText());
            if (node.has("detectedSourceLanguage")) {
                detectedSourceLang = node.get("detectedSourceLanguage").asText();
            }
        }

        return new Translation(sourceText, translatedText.toString(),
                "auto".equals(fromLang) ? detectedSourceLang : fromLang, toLang, getApiName());
    }

    @Override
    public List<String> getSupportedLanguages() {
        // Google Translate支持100+种语言，这里列出常用的
        return Arrays.asList(
                "auto", "zh", "en", "ja", "ko", "fr", "de", "es", "ru",
                "pt", "it", "vi", "th", "ar", "tr", "id", "ms", "nl", "pl"
        );
    }

    @Override
    public String getApiName() {
        return "google";
    }

    @Override
    public boolean isConfigured() {
        return apiKey != null && !apiKey.isEmpty();
    }

    @Override
    public String getDisplayName() {
        return "Google翻译";
    }

    @Override
    public List<ApiConfigField> getConfigFields() {
        return Arrays.asList(
                new ApiConfigField("apiKey", "API Key", ApiConfigField.FieldType.PASSWORD, true, null, "Google Cloud API Key")
        );
    }

    @Override
    public String getConfigDescription() {
        return "请前往 console.cloud.google.com 启用Translation API并获取密钥";
    }

    @Override
    public void setConfigValue(String fieldName, String value) {
        if ("apiKey".equals(fieldName)) {
            this.apiKey = value;
        }
    }

    @Override
    public String getConfigValue(String fieldName) {
        if ("apiKey".equals(fieldName)) {
            return apiKey;
        }
        return null;
    }
}
