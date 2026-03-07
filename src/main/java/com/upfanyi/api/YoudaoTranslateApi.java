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
import java.security.MessageDigest;
import java.util.Arrays;
import java.util.List;

/**
 * 有道翻译API实现
 *
 * API文档: https://ai.youdao.com/doc.s#guide
 *
 * @author UP翻译团队
 * @version 1.0.0
 */
public class YoudaoTranslateApi implements TranslateApi {

    private static final Logger logger = LoggerFactory.getLogger(YoudaoTranslateApi.class);

    private static final String API_URL = "https://openapi.youdao.com/api";

    private final OkHttpClient httpClient;
    private final ObjectMapper objectMapper;

    private String appKey;
    private String appSecret;

    public YoudaoTranslateApi() {
        this.httpClient = new OkHttpClient();
        this.objectMapper = new ObjectMapper();
    }

    public YoudaoTranslateApi(String appKey, String appSecret) {
        this();
        this.appKey = appKey;
        this.appSecret = appSecret;
    }

    public void setAppKey(String appKey) {
        this.appKey = appKey;
    }

    public void setAppSecret(String appSecret) {
        this.appSecret = appSecret;
    }

    @Override
    public Translation translate(String text, String fromLang, String toLang) throws Exception {
        if (!isConfigured()) {
            throw new IllegalStateException("有道翻译API未配置，请设置App Key和App Secret");
        }

        logger.debug("有道翻译: {} -> {}, 文本: {}", fromLang, toLang, text);

        // 生成随机数、时间戳和签名
        String salt = String.valueOf(System.currentTimeMillis());
        String curtime = String.valueOf(System.currentTimeMillis() / 1000);
        String sign = generateSign(text, salt, curtime);

        // 构建请求URL
        String url = buildUrl(text, fromLang, toLang, salt, curtime, sign);

        // 发送请求
        Request request = new Request.Builder()
                .url(url)
                .get()
                .build();

        try (Response response = httpClient.newCall(request).execute()) {
            if (!response.isSuccessful()) {
                throw new Exception("翻译请求失败: " + response.code());
            }

            String responseBody = response.body().string();
            logger.debug("有道翻译响应: {}", responseBody);

            return parseResponse(responseBody, text, fromLang, toLang);
        }
    }

    private String generateSign(String text, String salt, String curtime) throws Exception {
        String input = appKey + getInput(text) + salt + curtime + appSecret;
        MessageDigest sha256 = MessageDigest.getInstance("SHA-256");
        byte[] digest = sha256.digest(input.getBytes(StandardCharsets.UTF_8));

        StringBuilder sb = new StringBuilder();
        for (byte b : digest) {
            sb.append(String.format("%02x", b & 0xff));
        }
        return sb.toString();
    }

    private String getInput(String text) {
        if (text.length() <= 20) {
            return text;
        }
        return text.substring(0, 10) + text.length() + text.substring(text.length() - 10);
    }

    private String buildUrl(String text, String fromLang, String toLang, String salt, String curtime, String sign) throws Exception {
        StringBuilder url = new StringBuilder(API_URL);
        url.append("?q=").append(URLEncoder.encode(text, StandardCharsets.UTF_8));
        url.append("&from=").append(fromLang);
        url.append("&to=").append(toLang);
        url.append("&appKey=").append(appKey);
        url.append("&salt=").append(salt);
        url.append("&curtime=").append(curtime);
        url.append("&sign=").append(sign);
        url.append("&signType=v3");
        return url.toString();
    }

    private Translation parseResponse(String responseBody, String sourceText, String fromLang, String toLang) throws Exception {
        JsonNode root = objectMapper.readTree(responseBody);

        // 检查错误
        String errorCode = root.get("errorCode").asText();
        if (!"0".equals(errorCode)) {
            throw new Exception("有道翻译错误: " + errorCode);
        }

        // 解析翻译结果
        JsonNode translation = root.get("translation");
        if (translation == null || !translation.isArray() || translation.size() == 0) {
            throw new Exception("翻译结果为空");
        }

        StringBuilder translatedText = new StringBuilder();
        for (JsonNode node : translation) {
            translatedText.append(node.asText());
        }

        Translation result = new Translation(sourceText, translatedText.toString(), fromLang, toLang, getApiName());
        return result;
    }

    @Override
    public List<String> getSupportedLanguages() {
        return Arrays.asList(
                "auto",   // 自动检测
                "zh-CHS", // 中文简体
                "zh-CHT", // 中文繁体
                "en",     // 英语
                "ja",     // 日语
                "ko",     // 韩语
                "fr",     // 法语
                "es",     // 西班牙语
                "pt",     // 葡萄牙语
                "it",     // 意大利语
                "de",     // 德语
                "ru",     // 俄语
                "ar",     // 阿拉伯语
                "th",     // 泰语
                "vi",     // 越南语
                "id",     // 印尼语
                "ms",     // 马来语
                "nl",     // 荷兰语
                "pl",     // 波兰语
                "sv",     // 瑞典语
                "tr",     // 土耳其语
                "hu",     // 匈牙利语
                "cs",     // 捷克语
                "da",     // 丹麦语
                "fi",     // 芬兰语
                "he",     // 希伯来语
                "el",     // 希腊语
                "no",     // 挪威语
                "ro",     // 罗马尼亚语
                "sk"      // 斯洛伐克语
        );
    }

    @Override
    public String getApiName() {
        return "youdao";
    }

    @Override
    public boolean isConfigured() {
        return appKey != null && !appKey.isEmpty() && appSecret != null && !appSecret.isEmpty();
    }

    @Override
    public String getDisplayName() {
        return "有道翻译";
    }

    @Override
    public List<ApiConfigField> getConfigFields() {
        return Arrays.asList(
                new ApiConfigField("appKey", "App Key", ApiConfigField.FieldType.TEXT, true, null, "有道翻译App Key"),
                new ApiConfigField("appSecret", "App Secret", ApiConfigField.FieldType.PASSWORD, true, null, "有道翻译密钥")
        );
    }

    @Override
    public String getConfigDescription() {
        return "请前往 ai.youdao.com 申请API密钥";
    }

    @Override
    public void setConfigValue(String fieldName, String value) {
        switch (fieldName) {
            case "appKey":
                this.appKey = value;
                break;
            case "appSecret":
                this.appSecret = value;
                break;
        }
    }

    @Override
    public String getConfigValue(String fieldName) {
        switch (fieldName) {
            case "appKey":
                return appKey;
            case "appSecret":
                return appSecret;
            default:
                return null;
        }
    }
}
