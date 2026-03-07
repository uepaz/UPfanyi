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
 * 百度翻译API实现
 *
 * API文档: https://fanyi-api.baidu.com/doc/21
 *
 * @author UP翻译团队
 * @version 1.0.0
 */
public class BaiduTranslateApi implements TranslateApi {

    private static final Logger logger = LoggerFactory.getLogger(BaiduTranslateApi.class);

    private static final String API_URL = "https://fanyi-api.baidu.com/api/trans/vip/translate";

    private final OkHttpClient httpClient;
    private final ObjectMapper objectMapper;

    private String appId;
    private String secretKey;

    public BaiduTranslateApi() {
        this.httpClient = new OkHttpClient();
        this.objectMapper = new ObjectMapper();
    }

    public BaiduTranslateApi(String appId, String secretKey) {
        this();
        this.appId = appId;
        this.secretKey = secretKey;
    }

    public void setAppId(String appId) {
        this.appId = appId;
    }

    public void setSecretKey(String secretKey) {
        this.secretKey = secretKey;
    }

    @Override
    public Translation translate(String text, String fromLang, String toLang) throws Exception {
        if (!isConfigured()) {
            throw new IllegalStateException("百度翻译API未配置，请设置App ID和Secret Key");
        }

        logger.debug("百度翻译: {} -> {}, 文本: {}", fromLang, toLang, text);

        // 生成随机数和签名
        String salt = String.valueOf(System.currentTimeMillis());
        String sign = generateSign(text, salt);

        // 构建请求URL
        String url = buildUrl(text, fromLang, toLang, salt, sign);

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
            logger.debug("百度翻译响应: {}", responseBody);

            return parseResponse(responseBody, text, fromLang, toLang);
        }
    }

    private String generateSign(String text, String salt) throws Exception {
        String input = appId + text + salt + secretKey;
        MessageDigest md5 = MessageDigest.getInstance("MD5");
        byte[] digest = md5.digest(input.getBytes(StandardCharsets.UTF_8));

        StringBuilder sb = new StringBuilder();
        for (byte b : digest) {
            sb.append(String.format("%02x", b & 0xff));
        }
        return sb.toString();
    }

    private String buildUrl(String text, String fromLang, String toLang, String salt, String sign) throws Exception {
        StringBuilder url = new StringBuilder(API_URL);
        url.append("?q=").append(URLEncoder.encode(text, StandardCharsets.UTF_8));
        url.append("&from=").append(fromLang);
        url.append("&to=").append(toLang);
        url.append("&appid=").append(appId);
        url.append("&salt=").append(salt);
        url.append("&sign=").append(sign);
        return url.toString();
    }

    private Translation parseResponse(String responseBody, String sourceText, String fromLang, String toLang) throws Exception {
        JsonNode root = objectMapper.readTree(responseBody);

        // 检查错误
        if (root.has("error_code")) {
            String errorCode = root.get("error_code").asText();
            String errorMsg = root.has("error_msg") ? root.get("error_msg").asText() : "未知错误";
            throw new Exception("百度翻译错误: " + errorCode + " - " + errorMsg);
        }

        // 解析翻译结果
        JsonNode transResult = root.get("trans_result");
        if (transResult == null || !transResult.isArray() || transResult.size() == 0) {
            throw new Exception("翻译结果为空");
        }

        StringBuilder translatedText = new StringBuilder();
        for (JsonNode node : transResult) {
            translatedText.append(node.get("dst").asText());
        }

        Translation translation = new Translation(sourceText, translatedText.toString(), fromLang, toLang, getApiName());
        return translation;
    }

    @Override
    public List<String> getSupportedLanguages() {
        return Arrays.asList(
                "auto",  // 自动检测
                "zh",    // 中文
                "en",    // 英语
                "yue",   // 粤语
                "wyw",   // 文言文
                "jp",    // 日语
                "kor",   // 韩语
                "fra",   // 法语
                "spa",   // 西班牙语
                "th",    // 泰语
                "ara",   // 阿拉伯语
                "ru",    // 俄语
                "pt",    // 葡萄牙语
                "de",    // 德语
                "it",    // 意大利语
                "el",    // 希腊语
                "nl",    // 荷兰语
                "pl",    // 波兰语
                "bul",   // 保加利亚语
                "est",   // 爱沙尼亚语
                "dan",   // 丹麦语
                "fin",   // 芬兰语
                "cs",    // 捷克语
                "rom",   // 罗马尼亚语
                "slo",   // 斯洛文尼亚语
                "swe",   // 瑞典语
                "hu",    // 匈牙利语
                "cht",   // 繁体中文
                "vie"    // 越南语
        );
    }

    @Override
    public String getApiName() {
        return "baidu";
    }

    @Override
    public boolean isConfigured() {
        return appId != null && !appId.isEmpty() && secretKey != null && !secretKey.isEmpty();
    }

    @Override
    public String getDisplayName() {
        return "百度翻译";
    }

    @Override
    public List<ApiConfigField> getConfigFields() {
        return Arrays.asList(
                new ApiConfigField("appId", "App ID", ApiConfigField.FieldType.TEXT, true, null, "百度翻译App ID"),
                new ApiConfigField("secretKey", "Secret Key", ApiConfigField.FieldType.PASSWORD, true, null, "百度翻译密钥")
        );
    }

    @Override
    public String getConfigDescription() {
        return "请前往 fanyi-api.baidu.com 申请API密钥";
    }

    @Override
    public void setConfigValue(String fieldName, String value) {
        switch (fieldName) {
            case "appId":
                this.appId = value;
                break;
            case "secretKey":
                this.secretKey = value;
                break;
        }
    }

    @Override
    public String getConfigValue(String fieldName) {
        switch (fieldName) {
            case "appId":
                return appId;
            case "secretKey":
                return secretKey;
            default:
                return null;
        }
    }
}
