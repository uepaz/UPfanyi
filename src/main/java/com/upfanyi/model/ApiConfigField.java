package com.upfanyi.model;

/**
 * API配置字段定义
 *
 * @author UP翻译团队
 * @version 1.0.0
 */
public class ApiConfigField {

    public enum FieldType {
        TEXT,       // 普通文本
        PASSWORD,   // 密码字段
        URL         // URL地址
    }

    private final String fieldName;       // 字段名（如 appId, appSecret）
    private final String displayName;     // 显示名称
    private final FieldType fieldType;    // 字段类型
    private final boolean required;       // 是否必填
    private final String defaultValue;    // 默认值
    private final String placeholder;     // 占位符提示

    public ApiConfigField(String fieldName, String displayName, FieldType fieldType, boolean required) {
        this(fieldName, displayName, fieldType, required, null, null);
    }

    public ApiConfigField(String fieldName, String displayName, FieldType fieldType, boolean required, String defaultValue, String placeholder) {
        this.fieldName = fieldName;
        this.displayName = displayName;
        this.fieldType = fieldType;
        this.required = required;
        this.defaultValue = defaultValue;
        this.placeholder = placeholder;
    }

    public String getFieldName() {
        return fieldName;
    }

    public String getDisplayName() {
        return displayName;
    }

    public FieldType getFieldType() {
        return fieldType;
    }

    public boolean isRequired() {
        return required;
    }

    public String getDefaultValue() {
        return defaultValue;
    }

    public String getPlaceholder() {
        return placeholder;
    }
}
