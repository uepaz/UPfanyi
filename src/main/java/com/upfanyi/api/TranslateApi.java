package com.upfanyi.api;

import com.upfanyi.model.ApiConfigField;
import com.upfanyi.model.ApiType;
import com.upfanyi.model.Translation;

import java.util.List;

/**
 * 翻译API接口
 *
 * @author UP翻译团队
 * @version 1.0.0
 */
public interface TranslateApi {

    /**
     * 翻译文本
     *
     * @param text     要翻译的文本
     * @param fromLang 源语言
     * @param toLang   目标语言
     * @return 翻译结果
     * @throws Exception 翻译异常
     */
    Translation translate(String text, String fromLang, String toLang) throws Exception;

    /**
     * 获取支持的语言列表
     *
     * @return 语言代码列表
     */
    List<String> getSupportedLanguages();

    /**
     * 获取API名称
     *
     * @return API名称
     */
    String getApiName();

    /**
     * 检查API是否已配置
     *
     * @return true表示已配置
     */
    boolean isConfigured();

    /**
     * 获取API显示名称
     *
     * @return 显示名称
     */
    String getDisplayName();

    /**
     * 获取API类型
     *
     * @return API类型
     */
    default ApiType getApiType() {
        return ApiType.TRADITIONAL;
    }

    /**
     * 获取配置字段定义
     *
     * @return 配置字段列表
     */
    default List<ApiConfigField> getConfigFields() {
        return List.of();
    }

    /**
     * 获取配置说明
     *
     * @return 配置说明
     */
    default String getConfigDescription() {
        return null;
    }

    /**
     * 设置配置值
     *
     * @param fieldName  字段名
     * @param value      值
     */
    default void setConfigValue(String fieldName, String value) {
        // 默认实现为空，子类可覆盖
    }

    /**
     * 获取配置值
     *
     * @param fieldName 字段名
     * @return 值
     */
    default String getConfigValue(String fieldName) {
        return null;
    }
}
