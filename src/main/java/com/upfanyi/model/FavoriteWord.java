package com.upfanyi.model;

import java.time.LocalDateTime;

/**
 * 收藏/生词本模型
 *
 * @author UP翻译团队
 * @version 1.0.0
 */
public class FavoriteWord {

    private Long id;
    private String word;
    private String translation;
    private String note;
    private LocalDateTime createTime;

    public FavoriteWord() {
        this.createTime = LocalDateTime.now();
    }

    public FavoriteWord(String word, String translation) {
        this.word = word;
        this.translation = translation;
        this.createTime = LocalDateTime.now();
    }

    public FavoriteWord(String word, String translation, String note) {
        this.word = word;
        this.translation = translation;
        this.note = note;
        this.createTime = LocalDateTime.now();
    }

    // Getters and Setters
    public Long getId() {
        return id;
    }

    public void setId(Long id) {
        this.id = id;
    }

    public String getWord() {
        return word;
    }

    public void setWord(String word) {
        this.word = word;
    }

    public String getTranslation() {
        return translation;
    }

    public void setTranslation(String translation) {
        this.translation = translation;
    }

    public String getNote() {
        return note;
    }

    public void setNote(String note) {
        this.note = note;
    }

    public LocalDateTime getCreateTime() {
        return createTime;
    }

    public void setCreateTime(LocalDateTime createTime) {
        this.createTime = createTime;
    }

    @Override
    public String toString() {
        return "FavoriteWord{" +
                "id=" + id +
                ", word='" + word + '\'' +
                ", translation='" + translation + '\'' +
                ", note='" + note + '\'' +
                ", createTime=" + createTime +
                '}';
    }
}
