package com.upfanyi.api;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
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

import javax.crypto.Mac;
import javax.crypto.spec.SecretKeySpec;
import java.nio.charset.StandardCharsets;
import java.security.MessageDigest;
import java.util.Arrays;
import java.util.List;
import java.util.UUID;
import java.util.concurrent.TimeUnit;

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
        this.httpClient = new OkHttpClient.Builder()
                .connectTimeout(10, TimeUnit.SECONDS)
                .readTimeout(30, TimeUnit.SECONDS)
                .writeTimeout(10, TimeUnit.SECONDS)
                .build();
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

        String sourceLanguage = convertLanguageCode(fromLang);
        String targetLanguage = convertLanguageCode(toLang);

        // 使用 Jackson 构建请求体
        ObjectNode bodyNode = objectMapper.createObjectNode();
        bodyNode.put("SourceLanguage", sourceLanguage);
        bodyNode.put("TargetLanguage", targetLanguage);
        bodyNode.put("SourceText", text);
        bodyNode.put("Scene", "general");
        bodyNode.put("FormatType", "text");
        String jsonBody = objectMapper.writeValueAsString(bodyNode);

        // 生成签名所需参数
        String timestamp = java.time.format.DateTimeFormatter.RFC_1123_DATE_TIME
                .format(java.time.ZonedDateTime.now(java.time.ZoneId.of("GMT")));
        String nonce = UUID.randomUUID().toString();

        // ACS3-HMAC-SHA256 签名
        String authorization = generateAuthorization(jsonBody, timestamp, nonce);

        Request request = new Request.Builder()
                .url(API_URL + "?Action=TranslateGeneral&Version=" + API_VERSION)
                .addHeader("Content-Type", "application/json")
                .addHeader("x-acs-action", "TranslateGeneral")
                .addHeader("x-acs-version", API_VERSION)
                .addHeader("x-acs-date", timestamp)
                .addHeader("x-acs-content-sha256", sha256Hex(jsonBody))
                .addHeader("x-acs-signature-nonce", nonce)
                .addHeader("x-acs-signature-version", "ACS3-HMAC-SHA256")
                .addHeader("Authorization", authorization)
                .post(RequestBody.create(jsonBody, MediaType.parse("application/json")))
                .build();

        try (Response response = httpClient.newCall(request).execute()) {
            if (!response.isSuccessful()) {
                String errorBody = response.body() != null ? response.body().string() : "Unknown error";
                throw new Exception("翻译请求失败: " + response.code() + " - " + errorBody);
            }

            String responseBody = response.body() != null ? response.body().string() : "";
            logger.debug("阿里云翻译响应: {}", responseBody);

            return parseResponse(responseBody, text, fromLang, toLang);
        }
    }

    private String generateAuthorization(String jsonBody, String timestamp, String nonce) throws Exception {
        // Step 1: 构建规范请求
        String httpRequestMethod = "POST";
        String canonicalUri = "/";
        String canonicalQueryString = "Action=TranslateGeneral&Version=" + API_VERSION;
        String canonicalHeaders = "content-type:application/json\n"
                + "host:" + "mt.cn-hangzhou.aliyuncs.com" + "\n"
                + "x-acs-action:TranslateGeneral\n"
                + "x-acs-content-sha256:" + sha256Hex(jsonBody) + "\n"
                + "x-acs-date:" + timestamp + "\n"
                + "x-acs-signature-nonce:" + nonce + "\n"
                + "x-acs-signature-version:ACS3-HMAC-SHA256\n"
                + "x-acs-version:" + API_VERSION + "\n";
        String signedHeaders = "content-type;host;x-acs-action;x-acs-content-sha256;x-acs-date;x-acs-signature-nonce;x-acs-signature-version;x-acs-version";
        String hashedPayload = sha256Hex(jsonBody);

        String canonicalRequest = httpRequestMethod + "\n"
                + canonicalUri + "\n"
                + canonicalQueryString + "\n"
                + canonicalHeaders + "\n"
                + signedHeaders + "\n"
                + hashedPayload;

        // Step 2: 构建待签名字符串
        String stringToSign = "ACS3-HMAC-SHA256\n" + sha256Hex(canonicalRequest);

        // Step 3: 计算签名
        String signature = hmacSha256Hex(accessKeySecret, stringToSign);

        // Step 4: 构建 Authorization
        return "ACS3-HMAC-SHA256 Credential=" + accessKeyId
                + ",SignedHeaders=" + signedHeaders
                + ",Signature=" + signature;
    }

    private String sha256Hex(String input) throws Exception {
        MessageDigest md = MessageDigest.getInstance("SHA-256");
        byte[] digest = md.digest(input.getBytes(StandardCharsets.UTF_8));
        return bytesToHex(digest);
    }

    private String hmacSha256Hex(String key, String data) throws Exception {
        Mac mac = Mac.getInstance("HmacSHA256");
        mac.init(new SecretKeySpec(key.getBytes(StandardCharsets.UTF_8), "HmacSHA256"));
        byte[] digest = mac.doFinal(data.getBytes(StandardCharsets.UTF_8));
        return bytesToHex(digest);
    }

    private String bytesToHex(byte[] bytes) {
        StringBuilder sb = new StringBuilder();
        for (byte b : bytes) {
            sb.append(String.format("%02x", b & 0xff));
        }
        return sb.toString();
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
