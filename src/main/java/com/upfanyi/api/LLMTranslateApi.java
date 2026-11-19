package com.upfanyi.api;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.databind.node.ArrayNode;
import com.fasterxml.jackson.databind.node.ObjectNode;
import com.upfanyi.model.ApiConfigField;
import com.upfanyi.model.ApiType;
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
import java.util.concurrent.TimeUnit;

/**
 * LLM翻译API实现
 * 支持OpenAI兼容API和Ollama本地模型
 *
 * @author UP翻译团队
 * @version 1.0.0
 */
public class LLMTranslateApi implements TranslateApi {

    private static final Logger logger = LoggerFactory.getLogger(LLMTranslateApi.class);

    private static final String DEFAULT_OPENAI_ENDPOINT = "https://api.openai.com/v1/chat/completions";
    private static final String DEFAULT_OLLAMA_ENDPOINT = "http://localhost:11434/api/chat";
    private static final String DEFAULT_MODEL = "gpt-3.5-turbo";
    private static final String DEFAULT_SYSTEM_PROMPT = "You are a professional translator. Translate the following text from {source_lang} to {target_lang}. Only output the translation result, no explanations.";

    private final OkHttpClient httpClient;
    private final ObjectMapper objectMapper;

    private ApiType apiType = ApiType.LLM_OPENAI;
    private String endpoint = DEFAULT_OPENAI_ENDPOINT;
    private String apiKey;
    private String model = DEFAULT_MODEL;
    private String systemPrompt = DEFAULT_SYSTEM_PROMPT;

    public LLMTranslateApi() {
        this.httpClient = new OkHttpClient.Builder()
                .connectTimeout(30, TimeUnit.SECONDS)
                .readTimeout(60, TimeUnit.SECONDS)
                .writeTimeout(30, TimeUnit.SECONDS)
                .build();
        this.objectMapper = new ObjectMapper();
    }

    public void setApiType(ApiType apiType) {
        this.apiType = apiType;
        // 根据类型设置默认端点
        if (apiType == ApiType.LLM_OLLAMA && (endpoint == null || endpoint.equals(DEFAULT_OPENAI_ENDPOINT))) {
            this.endpoint = DEFAULT_OLLAMA_ENDPOINT;
        }
    }

    public void setEndpoint(String endpoint) {
        this.endpoint = endpoint;
    }

    public void setApiKey(String apiKey) {
        this.apiKey = apiKey;
    }

    public void setModel(String model) {
        this.model = model;
    }

    public void setSystemPrompt(String systemPrompt) {
        this.systemPrompt = systemPrompt;
    }

    @Override
    public Translation translate(String text, String fromLang, String toLang) throws Exception {
        if (!isConfigured()) {
            throw new IllegalStateException("LLM翻译API未配置，请设置相关参数");
        }

        logger.debug("LLM翻译 ({}): {} -> {}, 文本: {}", model, fromLang, toLang, text);

        String translatedText;
        if (apiType == ApiType.LLM_OLLAMA) {
            translatedText = translateWithOllama(text, fromLang, toLang);
        } else {
            translatedText = translateWithOpenAI(text, fromLang, toLang);
        }

        return new Translation(text, translatedText, fromLang, toLang, getApiName());
    }

    private String translateWithOpenAI(String text, String fromLang, String toLang) throws Exception {
        // 构建请求体
        ObjectNode requestBody = objectMapper.createObjectNode();
        requestBody.put("model", model);
        requestBody.put("temperature", 0.3);
        requestBody.put("max_tokens", 4096);

        ArrayNode messages = requestBody.putArray("messages");

        // 系统消息
        String systemContent = systemPrompt
                .replace("{source_lang}", getLanguageName(fromLang))
                .replace("{target_lang}", getLanguageName(toLang));
        ObjectNode systemMessage = messages.addObject();
        systemMessage.put("role", "system");
        systemMessage.put("content", systemContent);

        // 用户消息
        ObjectNode userMessage = messages.addObject();
        userMessage.put("role", "user");
        userMessage.put("content", text);

        String jsonBody = objectMapper.writeValueAsString(requestBody);
        logger.debug("OpenAI请求: {}", jsonBody);

        Request.Builder requestBuilder = new Request.Builder()
                .url(endpoint)
                .post(RequestBody.create(jsonBody, MediaType.parse("application/json")));

        if (apiKey != null && !apiKey.isEmpty()) {
            requestBuilder.addHeader("Authorization", "Bearer " + apiKey);
        }

        Request request = requestBuilder.build();

        try (Response response = httpClient.newCall(request).execute()) {
            if (!response.isSuccessful()) {
                String errorBody = response.body() != null ? response.body().string() : "Unknown error";
                throw new Exception("LLM翻译请求失败: " + response.code() + " - " + errorBody);
            }

            String responseBody = response.body() != null ? response.body().string() : "";
            logger.debug("OpenAI响应: {}", responseBody);

            return parseOpenAIResponse(responseBody);
        }
    }

    private String translateWithOllama(String text, String fromLang, String toLang) throws Exception {
        // 构建Ollama请求体
        ObjectNode requestBody = objectMapper.createObjectNode();
        requestBody.put("model", model);
        requestBody.put("stream", false);

        ObjectNode options = requestBody.putObject("options");
        options.put("temperature", 0.3);

        ArrayNode messages = requestBody.putArray("messages");

        // 系统消息
        String systemContent = systemPrompt
                .replace("{source_lang}", getLanguageName(fromLang))
                .replace("{target_lang}", getLanguageName(toLang));
        ObjectNode systemMessage = messages.addObject();
        systemMessage.put("role", "system");
        systemMessage.put("content", systemContent);

        // 用户消息
        ObjectNode userMessage = messages.addObject();
        userMessage.put("role", "user");
        userMessage.put("content", text);

        String jsonBody = objectMapper.writeValueAsString(requestBody);
        logger.debug("Ollama请求: {}", jsonBody);

        Request request = new Request.Builder()
                .url(endpoint)
                .post(RequestBody.create(jsonBody, MediaType.parse("application/json")))
                .build();

        try (Response response = httpClient.newCall(request).execute()) {
            if (!response.isSuccessful()) {
                String errorBody = response.body() != null ? response.body().string() : "Unknown error";
                throw new Exception("Ollama翻译请求失败: " + response.code() + " - " + errorBody);
            }

            String responseBody = response.body() != null ? response.body().string() : "";
            logger.debug("Ollama响应: {}", responseBody);

            return parseOllamaResponse(responseBody);
        }
    }

    private String parseOpenAIResponse(String responseBody) throws Exception {
        JsonNode root = objectMapper.readTree(responseBody);
        JsonNode choices = root.get("choices");

        if (choices == null || !choices.isArray() || choices.size() == 0) {
            throw new Exception("LLM翻译结果为空");
        }

        JsonNode message = choices.get(0).get("message");
        if (message == null || message.get("content") == null) {
            throw new Exception("LLM翻译结果格式错误");
        }

        return message.get("content").asText().trim();
    }

    private String parseOllamaResponse(String responseBody) throws Exception {
        JsonNode root = objectMapper.readTree(responseBody);
        JsonNode message = root.get("message");

        if (message == null || message.get("content") == null) {
            throw new Exception("Ollama翻译结果格式错误");
        }

        return message.get("content").asText().trim();
    }

    private String getLanguageName(String code) {
        switch (code) {
            case "auto": return "自动检测语言";
            case "zh": return "中文";
            case "en": return "英语";
            case "ja": return "日语";
            case "ko": return "韩语";
            case "fr": return "法语";
            case "de": return "德语";
            case "es": return "西班牙语";
            case "ru": return "俄语";
            default: return code;
        }
    }

    @Override
    public List<String> getSupportedLanguages() {
        // LLM支持所有主要语言
        return Arrays.asList(
                "auto", "zh", "en", "ja", "ko", "fr", "de", "es", "ru",
                "pt", "it", "ar", "th", "vi", "id", "ms", "nl", "pl"
        );
    }

    @Override
    public String getApiName() {
        return "llm";
    }

    @Override
    public boolean isConfigured() {
        if (!hasText(endpoint) || !hasText(model)) {
            return false;
        }

        if (apiType == ApiType.LLM_OLLAMA) {
            return true;
        }

        // 默认OpenAI端点必须有API Key；其他OpenAI兼容服务允许无鉴权。
        return hasText(apiKey) || !DEFAULT_OPENAI_ENDPOINT.equals(endpoint.trim());
    }

    private boolean hasText(String value) {
        return value != null && !value.trim().isEmpty();
    }

    @Override
    public String getDisplayName() {
        return "LLM翻译";
    }

    @Override
    public ApiType getApiType() {
        return apiType;
    }

    @Override
    public List<ApiConfigField> getConfigFields() {
        return Arrays.asList(
                new ApiConfigField("apiType", "API类型", ApiConfigField.FieldType.TEXT, true, "openai", "openai 或 ollama"),
                new ApiConfigField("endpoint", "API端点", ApiConfigField.FieldType.URL, true, DEFAULT_OPENAI_ENDPOINT, "API地址"),
                new ApiConfigField("apiKey", "API Key", ApiConfigField.FieldType.PASSWORD, false, null, "Ollama可留空"),
                new ApiConfigField("model", "模型名称", ApiConfigField.FieldType.TEXT, true, DEFAULT_MODEL, "如 gpt-3.5-turbo"),
                new ApiConfigField("systemPrompt", "系统提示词", ApiConfigField.FieldType.TEXT, false, DEFAULT_SYSTEM_PROMPT, "翻译提示词模板")
        );
    }

    @Override
    public String getConfigDescription() {
        return "支持OpenAI兼容API和Ollama本地模型，使用大语言模型进行翻译";
    }

    @Override
    public void setConfigValue(String fieldName, String value) {
        switch (fieldName) {
            case "apiType":
                setApiType("ollama".equalsIgnoreCase(value) ? ApiType.LLM_OLLAMA : ApiType.LLM_OPENAI);
                break;
            case "endpoint":
                this.endpoint = value;
                break;
            case "apiKey":
                this.apiKey = value;
                break;
            case "model":
                this.model = value;
                break;
            case "systemPrompt":
                this.systemPrompt = value;
                break;
        }
    }

    @Override
    public String getConfigValue(String fieldName) {
        switch (fieldName) {
            case "apiType":
                return apiType == ApiType.LLM_OLLAMA ? "ollama" : "openai";
            case "endpoint":
                return endpoint;
            case "apiKey":
                return apiKey;
            case "model":
                return model;
            case "systemPrompt":
                return systemPrompt;
            default:
                return null;
        }
    }
}
