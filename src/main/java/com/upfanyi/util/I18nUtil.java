package com.upfanyi.util;

import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

import java.awt.*;
import java.awt.image.BufferedImage;
import java.util.Locale;
import java.util.ResourceBundle;

/**
 * 国际化工具类
 *
 * @author UP翻译团队
 * @version 1.0.0
 */
public class I18nUtil {

    private static final Logger logger = LoggerFactory.getLogger(I18nUtil.class);

    private static final String BUNDLE_NAME = "i18n.messages";

    private static ResourceBundle bundle;
    private static Locale currentLocale;

    /**
     * 初始化国际化
     *
     * @param locale 语言区域
     */
    public static void init(Locale locale) {
        currentLocale = locale;
        try {
            bundle = ResourceBundle.getBundle(BUNDLE_NAME, locale);
            logger.info("国际化初始化成功: {}", locale);
        } catch (Exception e) {
            logger.warn("加载语言资源失败，使用默认语言", e);
            bundle = ResourceBundle.getBundle(BUNDLE_NAME, Locale.CHINA);
        }
    }

    /**
     * 获取翻译文本
     *
     * @param key 资源键
     * @return 翻译后的文本
     */
    public static String get(String key) {
        if (bundle == null) {
            init(Locale.CHINA);
        }
        try {
            return bundle.getString(key);
        } catch (Exception e) {
            logger.warn("未找到翻译资源: {}", key);
            return key;
        }
    }

    /**
     * 获取翻译文本（带参数）
     *
     * @param key  资源键
     * @param args 参数
     * @return 翻译后的文本
     */
    public static String get(String key, Object... args) {
        String template = get(key);
        return String.format(template, args);
    }

    /**
     * 获取当前语言
     */
    public static Locale getCurrentLocale() {
        return currentLocale;
    }

    /**
     * 切换语言
     */
    public static void setLocale(Locale locale) {
        init(locale);
    }

    /**
     * 检查是否为中文
     */
    public static boolean isChinese() {
        return currentLocale != null &&
                (currentLocale.equals(Locale.CHINA) || currentLocale.equals(Locale.CHINESE) ||
                        currentLocale.getLanguage().equals("zh"));
    }
}
