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

import javax.crypto.Mac;
import javax.crypto.spec.SecretKeySpec;
import java.net.URLEncoder;
import java.nio.charset.StandardCharsets;
import java.util.Arrays;
import java.util.List;

/**
 * 腾讯翻译君API实现
 *
 * API文档: https://cloud.tencent.com/document/product/551
 * 每月500万字符免费额度
 *
 * @author UP翻译团队
 * @version 1.0.0
 */
public class TencentTranslateApi implements TranslateApi {

    private static final Logger logger = LoggerFactory.getLogger(TencentTranslateApi.class);

    private static final String API_URL = "tmt.tencentcloudapi.com";
    private static final String SERVICE = "tmt";
    private static final String HOST = "tmt.tencentcloudapi.com";
    private static final String REGION = "ap-beijing";
    private static final String ACTION = "TextTranslate";

    private final OkHttpClient httpClient;
    private final ObjectMapper objectMapper;

    private String secretId;
    private String secretKey;

    public TencentTranslateApi() {
        this.httpClient = new OkHttpClient();
        this.objectMapper = new ObjectMapper();
    }

    public TencentTranslateApi(String secretId, String secretKey) {
        this();
        this.secretId = secretId;
        this.secretKey = secretKey;
    }

    public void setSecretId(String secretId) {
        this.secretId = secretId;
    }

    public void setSecretKey(String secretKey) {
        this.secretKey = secretKey;
    }

    @Override
    public Translation translate(String text, String fromLang, String toLang) throws Exception {
        if (!isConfigured()) {
            throw new IllegalStateException("腾讯翻译API未配置，请设置SecretId和SecretKey");
        }

        logger.debug("腾讯翻译: {} -> {}, 文本: {}", fromLang, toLang, text);

        // 构建请求参数
        String timestamp = String.valueOf(System.currentTimeMillis() / 1000);
        String payload = buildPayload(text, fromLang, toLang);

        // 生成签名
        String authorization = generateAuthorization(payload, timestamp);

        // 构建请求URL
        String url = "https://" + HOST + "?" + payload;

        Request request = new Request.Builder()
                .url(url)
                .addHeader("Authorization", authorization)
                .addHeader("X-TC-Timestamp", timestamp)
                .addHeader("X-TC-Version", "2018-03-21")
                .addHeader("X-TC-Region", REGION)
                .addHeader("Host", HOST)
                .addHeader("Content-Type", "application/json")
                .get()
                .build();

        try (Response response = httpClient.newCall(request).execute()) {
            if (!response.isSuccessful()) {
                throw new Exception("翻译请求失败: " + response.code());
            }

            String responseBody = response.body().string();
            logger.debug("腾讯翻译响应: {}", responseBody);

            return parseResponse(responseBody, text, fromLang, toLang);
        }
    }

    private String buildPayload(String text, String fromLang, String toLang) throws Exception {
        // 腾讯翻译语言代码转换
        String source = convertLanguageCode(fromLang);
        String target = convertLanguageCode(toLang);

        StringBuilder sb = new StringBuilder();
        sb.append("Action=").append(ACTION);
        sb.append("&Version=2018-03-21");
        sb.append("&Region=").append(REGION);
        sb.append("&Source=").append(source);
        sb.append("&Target=").append(target);
        sb.append("&SourceText=").append(URLEncoder.encode(text, StandardCharsets.UTF_8));
        sb.append("&ProjectId=0");
        return sb.toString();
    }

    private String convertLanguageCode(String code) {
        // 腾讯翻译使用不同的语言代码
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
            case "ms": return "ms";
            case "ar": return "ar";
            case "tr": return "tr";
            default: return code;
        }
    }

    private String generateAuthorization(String payload, String timestamp) throws Exception {
        String algorithm = "TC3-HMAC-SHA256";
        String date = java.time.LocalDate.now().toString();

        // Step 1: 拼接规范请求串
        String httpRequestMethod = "GET";
        String canonicalUri = "/";
        String canonicalQueryString = payload;
        String canonicalHeaders = "content-type:application/json\nhost:" + HOST + "\n";
        String signedHeaders = "content-type;host";
        String hashedRequestPayload = sha256Hex("");

        String canonicalRequest = httpRequestMethod + "\n"
                + canonicalUri + "\n"
                + canonicalQueryString + "\n"
                + canonicalHeaders + "\n"
                + signedHeaders + "\n"
                + hashedRequestPayload;

        // Step 2: 拼接待签名字符串
        String credentialScope = date + "/" + SERVICE + "/tc3_request";
        String hashedCanonicalRequest = sha256Hex(canonicalRequest);
        String stringToSign = algorithm + "\n"
                + timestamp + "\n"
                + credentialScope + "\n"
                + hashedCanonicalRequest;

        // Step 3: 计算签名
        byte[] secretDate = hmacSha256(("TC3" + secretKey).getBytes(StandardCharsets.UTF_8), date);
        byte[] secretService = hmacSha256(secretDate, SERVICE);
        byte[] secretSigning = hmacSha256(secretService, "tc3_request");
        String signature = bytesToHex(hmacSha256(secretSigning, stringToSign));

        // Step 4: 拼接 Authorization
        String authorization = algorithm + " "
                + "Credential=" + secretId + "/" + credentialScope + ", "
                + "SignedHeaders=" + signedHeaders + ", "
                + "Signature=" + signature;

        return authorization;
    }

    private String sha256Hex(String input) throws Exception {
        java.security.MessageDigest md = java.security.MessageDigest.getInstance("SHA-256");
        byte[] digest = md.digest(input.getBytes(StandardCharsets.UTF_8));
        return bytesToHex(digest);
    }

    private byte[] hmacSha256(byte[] key, String data) throws Exception {
        Mac mac = Mac.getInstance("HmacSHA256");
        mac.init(new SecretKeySpec(key, "HmacSHA256"));
        return mac.doFinal(data.getBytes(StandardCharsets.UTF_8));
    }

    private String bytesToHex(byte[] bytes) {
        StringBuilder sb = new StringBuilder();
        for (byte b : bytes) {
            sb.append(String.format("%02x", b & 0xff));
        }
        return sb.toString();
    }

    private Translation parseResponse(String responseBody, String sourceText, String fromLang, String toLang) throws Exception {
        JsonNode root = objectMapper.readTree(responseBody);

        // 检查错误
        if (root.has("Response")) {
            JsonNode response = root.get("Response");
            if (response.has("Error")) {
                String code = response.get("Error").get("Code").asText();
                String message = response.get("Error").get("Message").asText();
                throw new Exception("腾讯翻译错误: " + code + " - " + message);
            }

            if (response.has("TargetText")) {
                String translatedText = response.get("TargetText").asText();
                return new Translation(sourceText, translatedText, fromLang, toLang, getApiName());
            }
        }

        throw new Exception("腾讯翻译结果格式错误");
    }

    @Override
    public List<String> getSupportedLanguages() {
        return Arrays.asList(
                "auto", "zh", "en", "ja", "ko", "fr", "de", "es", "ru",
                "pt", "it", "vi", "th", "ms", "ar", "tr", "id"
        );
    }

    @Override
    public String getApiName() {
        return "tencent";
    }

    @Override
    public boolean isConfigured() {
        return secretId != null && !secretId.isEmpty()
                && secretKey != null && !secretKey.isEmpty();
    }

    @Override
    public String getDisplayName() {
        return "腾讯翻译君";
    }

    @Override
    public List<ApiConfigField> getConfigFields() {
        return Arrays.asList(
                new ApiConfigField("secretId", "SecretId", ApiConfigField.FieldType.TEXT, true, null, "腾讯云SecretId"),
                new ApiConfigField("secretKey", "SecretKey", ApiConfigField.FieldType.PASSWORD, true, null, "腾讯云SecretKey")
        );
    }

    @Override
    public String getConfigDescription() {
        return "请前往 cloud.tencent.com 申请API密钥，每月500万字符免费额度";
    }

    @Override
    public void setConfigValue(String fieldName, String value) {
        switch (fieldName) {
            case "secretId":
                this.secretId = value;
                break;
            case "secretKey":
                this.secretKey = value;
                break;
        }
    }

    @Override
    public String getConfigValue(String fieldName) {
        switch (fieldName) {
            case "secretId":
                return secretId;
            case "secretKey":
                return secretKey;
            default:
                return null;
        }
    }
}
