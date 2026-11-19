package com.upfanyi.dao;

import java.time.LocalDateTime;

final class DaoTimeUtil {

    private DaoTimeUtil() {
    }

    static LocalDateTime parseLocalDateTime(String value) {
        if (value == null || value.isBlank()) {
            return null;
        }

        String normalized = value.trim();
        if (normalized.indexOf('T') < 0) {
            normalized = normalized.replace(' ', 'T');
        }
        return LocalDateTime.parse(normalized);
    }
}
