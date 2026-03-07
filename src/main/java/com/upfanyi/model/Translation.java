package com.upfanyi.model;

import java.time.LocalDateTime;

/**
 * 翻译结果模型
 *
 * @author UP翻译团队
 * @version 1.0.0
 */
public class Translation {

    private Long id;
    private String sourceText;
    private String translatedText;
    private String sourceLang;
    private String targetLang;
    private String apiUsed;
    private LocalDateTime createTime;

    public Translation() {
        this.createTime = LocalDateTime.now();
    }

    public Translation(String sourceText, String translatedText, String sourceLang, String targetLang, String apiUsed) {
        this.sourceText = sourceText;
        this.translatedText = translatedText;
        this.sourceLang = sourceLang;
        this.targetLang = targetLang;
        this.apiUsed = apiUsed;
        this.createTime = LocalDateTime.now();
    }

    // Getters and Setters
    public Long getId() {
        return id;
    }

    public void setId(Long id) {
        this.id = id;
    }

    public String getSourceText() {
        return sourceText;
    }

    public void setSourceText(String sourceText) {
        this.sourceText = sourceText;
    }

    public String getTranslatedText() {
        return translatedText;
    }

    public void setTranslatedText(String translatedText) {
        this.translatedText = translatedText;
    }

    public String getSourceLang() {
        return sourceLang;
    }

    public void setSourceLang(String sourceLang) {
        this.sourceLang = sourceLang;
    }

    public String getTargetLang() {
        return targetLang;
    }

    public void setTargetLang(String targetLang) {
        this.targetLang = targetLang;
    }

    public String getApiUsed() {
        return apiUsed;
    }

    public void setApiUsed(String apiUsed) {
        this.apiUsed = apiUsed;
    }

    public LocalDateTime getCreateTime() {
        return createTime;
    }

    public void setCreateTime(LocalDateTime createTime) {
        this.createTime = createTime;
    }

    @Override
    public String toString() {
        return "Translation{" +
                "id=" + id +
                ", sourceText='" + sourceText + '\'' +
                ", translatedText='" + translatedText + '\'' +
                ", sourceLang='" + sourceLang + '\'' +
                ", targetLang='" + targetLang + '\'' +
                ", apiUsed='" + apiUsed + '\'' +
                ", createTime=" + createTime +
                '}';
    }
}
