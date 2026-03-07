package com.upfanyi.api;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import com.upfanyi.model.ApiConfigField;
import com.upfanyi.model.Translation;
import okhttp3.MediaType;
import okhttp3.OkHttpClient;
import okhttp3.Request;
import okhttp3.RequestBody;
import okhttp3.Response;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

import javax.crypto.Mac;
import javax.crypto.spec.SecretKeySpec;
import java.nio.charset.StandardCharsets;
import java.util.Arrays;
import java.util.List;
import java.util.UUID;

/**
 * 阿里云机器翻译API实现
 *
 * API文档: https://help.aliyun.com/document_detail/158244.html
 * 每月100万字符免费额度
 *
 * @author UP翻译团队
 * @version 1.0.0
 */
public class AliyunTranslateApi implements TranslateApi {

    private static final Logger logger = LoggerFactory.getLogger(AliyunTranslateApi.class);

    private static final String API_URL = "https://mt.cn-hangzhou.aliyuncs.com/";
    private static final String API_VERSION = "2018-10-12";

    private final OkHttpClient httpClient;
    private final ObjectMapper objectMapper;

    private String accessKeyId;
    private String accessKeySecret;

    public AliyunTranslateApi() {
        this.httpClient = new OkHttpClient();
        this.objectMapper = new ObjectMapper();
    }

    public AliyunTranslateApi(String accessKeyId, String accessKeySecret) {
        this();
        this.accessKeyId = accessKeyId;
        this.accessKeySecret = accessKeySecret;
    }

    public void setAccessKeyId(String accessKeyId) {
        this.accessKeyId = accessKeyId;
    }

    public void setAccessKeySecret(String accessKeySecret) {
        this.accessKeySecret = accessKeySecret;
    }

    @Override
    public Translation translate(String text, String fromLang, String toLang) throws Exception {
        if (!isConfigured()) {
            throw new IllegalStateException("阿里云翻译API未配置，请设置AccessKeyId和AccessKeySecret");
        }

        logger.debug("阿里云翻译: {} -> {}, 文本: {}", fromLang, toLang, text);

        // 构建请求参数
        String sourceLanguage = convertLanguageCode(fromLang);
        String targetLanguage = convertLanguageCode(toLang);
        String scene = "general";

        // 构建请求体
        String requestBody = String.format(
                "{\"SourceLanguage\":\"%s\",\"TargetLanguage\":\"%s\",\"SourceText\":\"%s\",\"Scene\":\"%s\",\"FormatType\":\"text\"}",
                sourceLanguage, targetLanguage, escapeJson(text), scene
        );

        // 生成签名
        String timestamp = java.time.format.DateTimeFormatter.RFC_1123_DATE_TIME
                .format(java.time.ZonedDateTime.now(java.time.ZoneId.of("GMT")));
        String nonce = UUID.randomUUID().toString();

        // 构建请求
        Request request = new Request.Builder()
                .url(API_URL + "?Action=TranslateGeneral&Version=" + API_VERSION)
                .addHeader("Content-Type", "application/json")
                .addHeader("x-acs-action", "TranslateGeneral")
                .addHeader("x-acs-version", API_VERSION)
                .addHeader("x-acs-date", timestamp)
                .addHeader("x-acs-content-sha256", "UNSIGNED-PAYLOAD")
                .post(RequestBody.create(requestBody, MediaType.parse("application/json")))
                .build();

        try (Response response = httpClient.newCall(request).execute()) {
            if (!response.isSuccessful()) {
                String errorBody = response.body() != null ? response.body().string() : "Unknown error";
                throw new Exception("翻译请求失败: " + response.code() + " - " + errorBody);
            }

            String responseBody = response.body().string();
            logger.debug("阿里云翻译响应: {}", responseBody);

            return parseResponse(responseBody, text, fromLang, toLang);
        }
    }

    private String convertLanguageCode(String code) {
        switch (code) {
            case "auto": return "auto";
            case "zh": return "zh";
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
            default: return code;
        }
    }

    private String escapeJson(String text) {
        return text.replace("\\", "\\\\")
                .replace("\"", "\\\"")
                .replace("\n", "\\n")
                .replace("\r", "\\r")
                .replace("\t", "\\t");
    }

    private Translation parseResponse(String responseBody, String sourceText, String fromLang, String toLang) throws Exception {
        JsonNode root = objectMapper.readTree(responseBody);

        // 检查错误
        if (root.has("Code")) {
            String code = root.get("Code").asText();
            if (!"200".equals(code) && !"Success".equals(code) && !"success".equalsIgnoreCase(code)) {
                String message = root.has("Message") ? root.get("Message").asText() : "Unknown error";
                throw new Exception("阿里云翻译错误: " + code + " - " + message);
            }
        }

        if (root.has("Data") && root.get("Data").has("Translated")) {
            String translatedText = root.get("Data").get("Translated").asText();
            return new Translation(sourceText, translatedText, fromLang, toLang, getApiName());
        }

        throw new Exception("阿里云翻译结果格式错误");
    }

    @Override
    public List<String> getSupportedLanguages() {
        return Arrays.asList(
                "auto", "zh", "en", "ja", "ko", "fr", "de", "es", "ru",
                "pt", "it", "vi", "th", "ar", "tr", "id", "ms"
        );
    }

    @Override
    public String getApiName() {
        return "aliyun";
    }

    @Override
    public boolean isConfigured() {
        return accessKeyId != null && !accessKeyId.isEmpty()
                && accessKeySecret != null && !accessKeySecret.isEmpty();
    }

    @Override
    public String getDisplayName() {
        return "阿里云翻译";
    }

    @Override
    public List<ApiConfigField> getConfigFields() {
        return Arrays.asList(
                new ApiConfigField("accessKeyId", "AccessKeyId", ApiConfigField.FieldType.TEXT, true, null, "阿里云AccessKeyId"),
                new ApiConfigField("accessKeySecret", "AccessKeySecret", ApiConfigField.FieldType.PASSWORD, true, null, "阿里云AccessKeySecret")
        );
    }

    @Override
    public String getConfigDescription() {
        return "请前往 aliyun.com 申请API密钥，每月100万字符免费额度";
    }

    @Override
    public void setConfigValue(String fieldName, String value) {
        switch (fieldName) {
            case "accessKeyId":
                this.accessKeyId = value;
                break;
            case "accessKeySecret":
                this.accessKeySecret = value;
                break;
        }
    }

    @Override
    public String getConfigValue(String fieldName) {
        switch (fieldName) {
            case "accessKeyId":
                return accessKeyId;
            case "accessKeySecret":
                return accessKeySecret;
            default:
                return null;
        }
    }
}
