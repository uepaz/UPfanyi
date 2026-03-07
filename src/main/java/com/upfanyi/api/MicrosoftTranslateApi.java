package com.upfanyi.api;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.databind.node.ArrayNode;
import com.fasterxml.jackson.databind.node.ObjectNode;
import com.upfanyi.model.ApiConfigField;
import com.upfanyi.model.Translation;
import okhttp3.MediaType;
import okhttp3.OkHttpClient;
import okhttp3.Request;
import okhttp3.RequestBody;
import okhttp3.Response;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

import java.util.Arrays;
import java.util.List;

/**
 * Microsoft Translator API实现
 *
 * API文档: https://learn.microsoft.com/azure/cognitive-services/translator/
 * 每月200万字符免费额度
 *
 * @author UP翻译团队
 * @version 1.0.0
 */
public class MicrosoftTranslateApi implements TranslateApi {

    private static final Logger logger = LoggerFactory.getLogger(MicrosoftTranslateApi.class);

    private static final String API_URL = "https://api.cognitive.microsofttranslator.com/translate?api-version=3.0";

    private final OkHttpClient httpClient;
    private final ObjectMapper objectMapper;

    private String subscriptionKey;
    private String region = "global";

    public MicrosoftTranslateApi() {
        this.httpClient = new OkHttpClient();
        this.objectMapper = new ObjectMapper();
    }

    public MicrosoftTranslateApi(String subscriptionKey, String region) {
        this();
        this.subscriptionKey = subscriptionKey;
        this.region = region != null && !region.isEmpty() ? region : "global";
    }

    public void setSubscriptionKey(String subscriptionKey) {
        this.subscriptionKey = subscriptionKey;
    }

    public void setRegion(String region) {
        this.region = region != null && !region.isEmpty() ? region : "global";
    }

    @Override
    public Translation translate(String text, String fromLang, String toLang) throws Exception {
        if (!isConfigured()) {
            throw new IllegalStateException("微软翻译API未配置，请设置Subscription Key");
        }

        logger.debug("微软翻译: {} -> {}, 文本: {}", fromLang, toLang, text);

        // 构建URL
        String targetLang = convertLanguageCode(toLang);
        StringBuilder urlBuilder = new StringBuilder(API_URL);
        urlBuilder.append("&to=").append(targetLang);

        if (!"auto".equals(fromLang)) {
            String sourceLang = convertLanguageCode(fromLang);
            urlBuilder.append("&from=").append(sourceLang);
        }

        // 构建请求体
        ObjectNode requestBody = objectMapper.createObjectNode();
        ArrayNode texts = requestBody.putArray("");
        ObjectNode textObj = texts.addObject();
        textObj.put("Text", text);

        String jsonBody = objectMapper.writeValueAsString(requestBody);
        logger.debug("微软翻译请求: {}", jsonBody);

        Request request = new Request.Builder()
                .url(urlBuilder.toString())
                .addHeader("Ocp-Apim-Subscription-Key", subscriptionKey)
                .addHeader("Ocp-Apim-Subscription-Region", region)
                .addHeader("Content-Type", "application/json")
                .post(RequestBody.create(jsonBody, MediaType.parse("application/json")))
                .build();

        try (Response response = httpClient.newCall(request).execute()) {
            if (!response.isSuccessful()) {
                String errorBody = response.body() != null ? response.body().string() : "Unknown error";
                throw new Exception("微软翻译请求失败: " + response.code() + " - " + errorBody);
            }

            String responseBody = response.body().string();
            logger.debug("微软翻译响应: {}", responseBody);

            return parseResponse(responseBody, text, fromLang, toLang);
        }
    }

    private String convertLanguageCode(String code) {
        // Microsoft Translator语言代码
        switch (code) {
            case "auto": return "auto";
            case "zh": return "zh-Hans";
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

        if (!root.isArray() || root.size() == 0) {
            // 检查错误
            if (root.has("error")) {
                JsonNode error = root.get("error");
                String message = error.has("message") ? error.get("message").asText() : "Unknown error";
                throw new Exception("微软翻译错误: " + message);
            }
            throw new Exception("微软翻译结果格式错误");
        }

        JsonNode firstResult = root.get(0);
        JsonNode translations = firstResult.get("translations");

        if (translations == null || !translations.isArray() || translations.size() == 0) {
            throw new Exception("微软翻译结果为空");
        }

        String translatedText = translations.get(0).get("text").asText();

        // 获取检测到的源语言
        String detectedSourceLang = fromLang;
        if ("auto".equals(fromLang) && firstResult.has("detectedLanguage")) {
            detectedSourceLang = firstResult.get("detectedLanguage").get("language").asText();
        }

        return new Translation(sourceText, translatedText,
                "auto".equals(fromLang) ? detectedSourceLang : fromLang, toLang, getApiName());
    }

    @Override
    public List<String> getSupportedLanguages() {
        return Arrays.asList(
                "auto", "zh", "en", "ja", "ko", "fr", "de", "es", "ru",
                "pt", "it", "vi", "th", "ar", "tr", "id", "ms", "nl", "pl"
        );
    }

    @Override
    public String getApiName() {
        return "microsoft";
    }

    @Override
    public boolean isConfigured() {
        return subscriptionKey != null && !subscriptionKey.isEmpty();
    }

    @Override
    public String getDisplayName() {
        return "微软翻译";
    }

    @Override
    public List<ApiConfigField> getConfigFields() {
        return Arrays.asList(
                new ApiConfigField("subscriptionKey", "Subscription Key", ApiConfigField.FieldType.PASSWORD, true, null, "Azure认知服务密钥"),
                new ApiConfigField("region", "Region", ApiConfigField.FieldType.TEXT, false, "global", "Azure区域，如eastasia")
        );
    }

    @Override
    public String getConfigDescription() {
        return "请前往 azure.microsoft.com 申请Azure认知服务，每月200万字符免费额度";
    }

    @Override
    public void setConfigValue(String fieldName, String value) {
        switch (fieldName) {
            case "subscriptionKey":
                this.subscriptionKey = value;
                break;
            case "region":
                this.region = value != null && !value.isEmpty() ? value : "global";
                break;
        }
    }

    @Override
    public String getConfigValue(String fieldName) {
        switch (fieldName) {
            case "subscriptionKey":
                return subscriptionKey;
            case "region":
                return region;
            default:
                return null;
        }
    }
}
