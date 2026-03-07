package com.upfanyi.model;

/**
 * API配置模型
 *
 * @author UP翻译团队
 * @version 1.0.0
 */
public class ApiConfig {

    private Long id;
    private String apiName;
    private String apiKey;
    private String apiSecret;
    private String endpoint;
    private String modelName;
    private String extraConfig;  // JSON格式存储额外配置
    private boolean enabled;
    private int priority;

    public ApiConfig() {
        this.enabled = true;
        this.priority = 0;
    }

    public ApiConfig(String apiName, String apiKey, String apiSecret) {
        this.apiName = apiName;
        this.apiKey = apiKey;
        this.apiSecret = apiSecret;
        this.enabled = true;
        this.priority = 0;
    }

    // Getters and Setters
    public Long getId() {
        return id;
    }

    public void setId(Long id) {
        this.id = id;
    }

    public String getApiName() {
        return apiName;
    }

    public void setApiName(String apiName) {
        this.apiName = apiName;
    }

    public String getApiKey() {
        return apiKey;
    }

    public void setApiKey(String apiKey) {
        this.apiKey = apiKey;
    }

    public String getApiSecret() {
        return apiSecret;
    }

    public void setApiSecret(String apiSecret) {
        this.apiSecret = apiSecret;
    }

    public String getEndpoint() {
        return endpoint;
    }

    public void setEndpoint(String endpoint) {
        this.endpoint = endpoint;
    }

    public String getModelName() {
        return modelName;
    }

    public void setModelName(String modelName) {
        this.modelName = modelName;
    }

    public String getExtraConfig() {
        return extraConfig;
    }

    public void setExtraConfig(String extraConfig) {
        this.extraConfig = extraConfig;
    }

    public boolean isEnabled() {
        return enabled;
    }

    public void setEnabled(boolean enabled) {
        this.enabled = enabled;
    }

    public int getPriority() {
        return priority;
    }

    public void setPriority(int priority) {
        this.priority = priority;
    }

    @Override
    public String toString() {
        return "ApiConfig{" +
                "id=" + id +
                ", apiName='" + apiName + '\'' +
                ", apiKey='" + (apiKey != null ? "****" : "null") + '\'' +
                ", apiSecret='" + (apiSecret != null ? "****" : "null") + '\'' +
                ", endpoint='" + endpoint + '\'' +
                ", modelName='" + modelName + '\'' +
                ", enabled=" + enabled +
                ", priority=" + priority +
                '}';
    }
}
