package com.upfanyi.model;

/**
 * API类型枚举
 *
 * @author UP翻译团队
 * @version 1.0.0
 */
public enum ApiType {
    TRADITIONAL,    // 传统翻译API（百度、有道等）
    LLM_OPENAI,     // OpenAI兼容API
    LLM_OLLAMA      // Ollama本地模型
}
