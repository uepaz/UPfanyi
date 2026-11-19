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

import java.nio.charset.StandardCharsets;
import java.util.Arrays;
import java.util.List;
import java.util.concurrent.TimeUnit;

/**
 * DeepL翻译API实现
 *
 * API文档: https://www.deepl.com/docs-api
 * 免费版50万字符/月
 *
 * @author UP翻译团队
 * @version 1.0.0
 */
public class DeepLTranslateApi implements TranslateApi {

    private static final Logger logger = LoggerFactory.getLogger(DeepLTranslateApi.class);

    private static final String API_URL_FREE = "https://api-free.deepl.com/v2/translate";
    private static final String API_URL_PRO = "https://api.deepl.com/v2/translate";

    private final OkHttpClient httpClient;
    private final ObjectMapper objectMapper;

    private String authKey;
    private boolean usePro = false;

    public DeepLTranslateApi() {
        this.httpClient = new OkHttpClient.Builder()
                .connectTimeout(10, TimeUnit.SECONDS)
                .readTimeout(30, TimeUnit.SECONDS)
                .writeTimeout(10, TimeUnit.SECONDS)
                .build();
        this.objectMapper = new ObjectMapper();
    }

    public DeepLTranslateApi(String authKey) {
        this();
        this.authKey = authKey;
        this.usePro = !isFreeKey(authKey);
    }

    public void setAuthKey(String authKey) {
        this.authKey = authKey;
        this.usePro = !isFreeKey(authKey);
    }

    private boolean isFreeKey(String key) {
        // DeepL Free key 以 ":fx" 结尾
        return key != null && key.endsWith(":fx");
    }

    @Override
    public Translation translate(String text, String fromLang, String toLang) throws Exception {
        if (!isConfigured()) {
            throw new IllegalStateException("DeepL翻译API未配置，请设置Auth Key");
        }

        logger.debug("DeepL翻译: {} -> {}, 文本: {}", fromLang, toLang, text);

        String apiUrl = usePro ? API_URL_PRO : API_URL_FREE;

        // 构建请求体
        ObjectNode requestBody = objectMapper.createObjectNode();
        ArrayNode texts = requestBody.putArray("text");
        texts.add(text);

        // DeepL语言代码
        String sourceLang = convertSourceLanguageCode(fromLang);
        String targetLang = convertTargetLanguageCode(toLang);

        if (!"auto".equals(sourceLang)) {
            requestBody.put("source_lang", sourceLang);
        }
        requestBody.put("target_lang", targetLang);

        String jsonBody = objectMapper.writeValueAsString(requestBody);
        logger.debug("DeepL请求: {}", jsonBody);

        Request request = new Request.Builder()
                .url(apiUrl)
                .addHeader("Authorization", "DeepL-Auth-Key " + authKey)
                .addHeader("Content-Type", "application/json")
                .post(RequestBody.create(jsonBody, MediaType.parse("application/json")))
                .build();

        try (Response response = httpClient.newCall(request).execute()) {
            if (!response.isSuccessful()) {
                String errorBody = response.body() != null ? response.body().string() : "Unknown error";
                throw new Exception("DeepL翻译请求失败: " + response.code() + " - " + errorBody);
            }

            String responseBody = response.body() != null ? response.body().string() : "";
            logger.debug("DeepL响应: {}", responseBody);

            return parseResponse(responseBody, text, fromLang, toLang);
        }
    }

    private String convertSourceLanguageCode(String code) {
        switch (code) {
            case "auto": return "auto";
            case "zh": return "ZH";
            case "en": return "EN";
            case "ja": return "JA";
            case "ko": return "KO";
            case "fr": return "FR";
            case "de": return "DE";
            case "es": return "ES";
            case "ru": return "RU";
            case "pt": return "PT";
            case "it": return "IT";
            case "nl": return "NL";
            case "pl": return "PL";
            case "tr": return "TR";
            case "ar": return "AR";
            default: return code.toUpperCase();
        }
    }

    private String convertTargetLanguageCode(String code) {
        switch (code) {
            case "zh": return "ZH"; // DeepL会根据内容自动选择简体或繁体
            case "en": return "EN-GB"; // 默认英式英语
            case "pt": return "PT-PT"; // 默认葡萄牙语
            default: return convertSourceLanguageCode(code);
        }
    }

    private Translation parseResponse(String responseBody, String sourceText, String fromLang, String toLang) throws Exception {
        JsonNode root = objectMapper.readTree(responseBody);

        JsonNode translations = root.get("translations");
        if (translations == null || !translations.isArray() || translations.size() == 0) {
            throw new Exception("DeepL翻译结果为空");
        }

        StringBuilder translatedText = new StringBuilder();
        for (JsonNode node : translations) {
            translatedText.append(node.get("text").asText());
        }

        return new Translation(sourceText, translatedText.toString(), fromLang, toLang, getApiName());
    }

    @Override
    public List<String> getSupportedLanguages() {
        return Arrays.asList(
                "auto", "zh", "en", "ja", "ko", "fr", "de", "es", "ru",
                "pt", "it", "nl", "pl", "tr", "ar"
        );
    }

    @Override
    public String getApiName() {
        return "deepl";
    }

    @Override
    public boolean isConfigured() {
        return authKey != null && !authKey.isEmpty();
    }

    @Override
    public String getDisplayName() {
        return "DeepL";
    }

    @Override
    public List<ApiConfigField> getConfigFields() {
        return Arrays.asList(
                new ApiConfigField("authKey", "Auth Key", ApiConfigField.FieldType.PASSWORD, true, null, "DeepL API密钥")
        );
    }

    @Override
    public String getConfigDescription() {
        return "请前往 deepl.com/pro-api 申请API密钥，免费版50万字符/月";
    }

    @Override
    public void setConfigValue(String fieldName, String value) {
        if ("authKey".equals(fieldName)) {
            setAuthKey(value);
        }
    }

    @Override
    public String getConfigValue(String fieldName) {
        if ("authKey".equals(fieldName)) {
            return authKey;
        }
        return null;
    }
}
