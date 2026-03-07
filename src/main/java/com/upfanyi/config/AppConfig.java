package com.upfanyi.config;

import com.upfanyi.api.ApiFactory;
import com.upfanyi.dao.ApiConfigDao;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

import java.io.*;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.util.Locale;
import java.util.Properties;

/**
 * 应用配置管理类
 *
 * @author UP翻译团队
 * @version 1.0.0
 */
public class AppConfig {

    private static final Logger logger = LoggerFactory.getLogger(AppConfig.class);

    private static final String CONFIG_DIR = System.getProperty("user.home") + "/.upfanyi";
    private static final String CONFIG_FILE = CONFIG_DIR + "/config.properties";
    private static final String DB_FILE = CONFIG_DIR + "/upfanyi.db";

    private static AppConfig instance;

    private Properties properties;

    // 配置键
    private static final String KEY_LOCALE = "app.locale";
    private static final String KEY_MINIMIZE_TO_TRAY = "app.minimizeToTray";
    private static final String KEY_ALWAYS_ON_TOP = "app.alwaysOnTop";
    private static final String KEY_CLIPBOARD_MONITOR = "clipboard.monitor";
    private static final String KEY_CLIPBOARD_MIN_LENGTH = "clipboard.minLength";
    private static final String KEY_HOTKEY_SHOW_WINDOW = "hotkey.showWindow";
    private static final String KEY_HOTKEY_CLIPBOARD_TRANSLATE = "hotkey.clipboardTranslate";
    private static final String KEY_DEFAULT_API = "api.default";
    private static final String KEY_DEFAULT_SOURCE_LANG = "translate.sourceLang";
    private static final String KEY_DEFAULT_TARGET_LANG = "translate.targetLang";
    private static final String KEY_AUTO_DETECT_LANG = "translate.autoDetect";
    private static final String KEY_AUTO_TRANSLATE = "translate.autoTranslate";

    private AppConfig() {
        properties = new Properties();
    }

    public static synchronized AppConfig getInstance() {
        if (instance == null) {
            instance = new AppConfig();
        }
        return instance;
    }

    /**
     * 初始化配置
     */
    public void init() {
        // 创建配置目录
        createConfigDir();

        // 加载配置
        loadConfig();

        // 初始化数据库
        DatabaseConfig.getInstance().init(getDbPath());

        // 初始化API工厂，加载API配置
        ApiFactory.getInstance().setApiConfigDao(new ApiConfigDao());
    }

    private void createConfigDir() {
        try {
            Path configDir = Paths.get(CONFIG_DIR);
            if (!Files.exists(configDir)) {
                Files.createDirectories(configDir);
                logger.info("创建配置目录: {}", CONFIG_DIR);
            }
        } catch (IOException e) {
            logger.error("创建配置目录失败", e);
        }
    }

    private void loadConfig() {
        File configFile = new File(CONFIG_FILE);
        if (configFile.exists()) {
            try (FileInputStream fis = new FileInputStream(configFile)) {
                properties.load(fis);
                logger.info("加载配置文件成功");
            } catch (IOException e) {
                logger.error("加载配置文件失败", e);
                setDefaults();
            }
        } else {
            setDefaults();
            save();
        }
    }

    private void setDefaults() {
        // 设置默认值
        properties.setProperty(KEY_LOCALE, "zh_CN");
        properties.setProperty(KEY_MINIMIZE_TO_TRAY, "true");
        properties.setProperty(KEY_ALWAYS_ON_TOP, "false");
        properties.setProperty(KEY_CLIPBOARD_MONITOR, "false");
        properties.setProperty(KEY_CLIPBOARD_MIN_LENGTH, "1");
        properties.setProperty(KEY_HOTKEY_SHOW_WINDOW, "ctrl shift T");
        properties.setProperty(KEY_HOTKEY_CLIPBOARD_TRANSLATE, "ctrl shift C");
        properties.setProperty(KEY_DEFAULT_API, "baidu");
        properties.setProperty(KEY_DEFAULT_SOURCE_LANG, "auto");
        properties.setProperty(KEY_DEFAULT_TARGET_LANG, "en");
        properties.setProperty(KEY_AUTO_DETECT_LANG, "true");
        properties.setProperty(KEY_AUTO_TRANSLATE, "false");

        logger.info("设置默认配置");
    }

    /**
     * 保存配置
     */
    public void save() {
        try (FileOutputStream fos = new FileOutputStream(CONFIG_FILE)) {
            properties.store(fos, "UP翻译配置文件");
            logger.info("保存配置文件成功");
        } catch (IOException e) {
            logger.error("保存配置文件失败", e);
        }
    }

    // ==================== Getters & Setters ====================

    public String getDbPath() {
        return DB_FILE;
    }

    public Locale getLocale() {
        String localeStr = properties.getProperty(KEY_LOCALE, "zh_CN");
        String[] parts = localeStr.split("_");
        if (parts.length >= 2) {
            return new Locale(parts[0], parts[1]);
        }
        return Locale.CHINA;
    }

    public void setLocale(Locale locale) {
        properties.setProperty(KEY_LOCALE, locale.toString());
    }

    public boolean isMinimizeToTray() {
        return Boolean.parseBoolean(properties.getProperty(KEY_MINIMIZE_TO_TRAY, "true"));
    }

    public void setMinimizeToTray(boolean minimizeToTray) {
        properties.setProperty(KEY_MINIMIZE_TO_TRAY, String.valueOf(minimizeToTray));
    }

    public boolean isAlwaysOnTop() {
        return Boolean.parseBoolean(properties.getProperty(KEY_ALWAYS_ON_TOP, "false"));
    }

    public void setAlwaysOnTop(boolean alwaysOnTop) {
        properties.setProperty(KEY_ALWAYS_ON_TOP, String.valueOf(alwaysOnTop));
    }

    public boolean isClipboardMonitor() {
        return Boolean.parseBoolean(properties.getProperty(KEY_CLIPBOARD_MONITOR, "false"));
    }

    public void setClipboardMonitor(boolean clipboardMonitor) {
        properties.setProperty(KEY_CLIPBOARD_MONITOR, String.valueOf(clipboardMonitor));
    }

    public int getClipboardMinLength() {
        return Integer.parseInt(properties.getProperty(KEY_CLIPBOARD_MIN_LENGTH, "1"));
    }

    public void setClipboardMinLength(int length) {
        properties.setProperty(KEY_CLIPBOARD_MIN_LENGTH, String.valueOf(length));
    }

    public String getHotkeyShowWindow() {
        return properties.getProperty(KEY_HOTKEY_SHOW_WINDOW, "ctrl shift T");
    }

    public void setHotkeyShowWindow(String hotkey) {
        properties.setProperty(KEY_HOTKEY_SHOW_WINDOW, hotkey);
    }

    public String getHotkeyClipboardTranslate() {
        return properties.getProperty(KEY_HOTKEY_CLIPBOARD_TRANSLATE, "ctrl shift C");
    }

    public void setHotkeyClipboardTranslate(String hotkey) {
        properties.setProperty(KEY_HOTKEY_CLIPBOARD_TRANSLATE, hotkey);
    }

    public String getDefaultApi() {
        return properties.getProperty(KEY_DEFAULT_API, "baidu");
    }

    public void setDefaultApi(String api) {
        properties.setProperty(KEY_DEFAULT_API, api);
    }

    public String getDefaultSourceLang() {
        return properties.getProperty(KEY_DEFAULT_SOURCE_LANG, "auto");
    }

    public void setDefaultSourceLang(String lang) {
        properties.setProperty(KEY_DEFAULT_SOURCE_LANG, lang);
    }

    public String getDefaultTargetLang() {
        return properties.getProperty(KEY_DEFAULT_TARGET_LANG, "en");
    }

    public void setDefaultTargetLang(String lang) {
        properties.setProperty(KEY_DEFAULT_TARGET_LANG, lang);
    }

    public boolean isAutoDetectLang() {
        return Boolean.parseBoolean(properties.getProperty(KEY_AUTO_DETECT_LANG, "true"));
    }

    public void setAutoDetectLang(boolean autoDetect) {
        properties.setProperty(KEY_AUTO_DETECT_LANG, String.valueOf(autoDetect));
    }

    public boolean isAutoTranslate() {
        return Boolean.parseBoolean(properties.getProperty(KEY_AUTO_TRANSLATE, "false"));
    }

    public void setAutoTranslate(boolean autoTranslate) {
        properties.setProperty(KEY_AUTO_TRANSLATE, String.valueOf(autoTranslate));
    }
}
