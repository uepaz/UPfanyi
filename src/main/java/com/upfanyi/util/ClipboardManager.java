package com.upfanyi.util;

import com.upfanyi.config.AppConfig;
import javafx.application.Platform;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

import java.awt.*;
import java.awt.datatransfer.Clipboard;
import java.awt.datatransfer.DataFlavor;
import java.awt.datatransfer.StringSelection;
import java.util.function.Consumer;

/**
 * 剪贴板管理器
 * 监听系统剪贴板变化，实现复制即翻译功能
 *
 * @author UP翻译团队
 * @version 1.0.0
 */
public class ClipboardManager {

    private static final Logger logger = LoggerFactory.getLogger(ClipboardManager.class);

    private static ClipboardManager instance;

    private final Clipboard clipboard;
    private String lastContent;
    private boolean isMonitoring = false;
    private Thread monitorThread;
    private Consumer<String> onTextCopied;
    private int minTextLength = 1;

    private ClipboardManager() {
        this.clipboard = Toolkit.getDefaultToolkit().getSystemClipboard();
    }

    public static synchronized ClipboardManager getInstance() {
        if (instance == null) {
            instance = new ClipboardManager();
        }
        return instance;
    }

    /**
     * 开始监听剪贴板
     *
     * @param onTextCopied 文本复制回调
     */
    public void startMonitoring(Consumer<String> onTextCopied) {
        if (isMonitoring) {
            logger.warn("剪贴板监听已在运行");
            return;
        }

        this.onTextCopied = onTextCopied;
        this.isMonitoring = true;

        monitorThread = new Thread(() -> {
            logger.info("开始监听剪贴板");

            while (isMonitoring) {
                try {
                    // 检查剪贴板内容
                    String content = getClipboardText();

                    if (content != null && !content.equals(lastContent) && content.length() >= minTextLength) {
                        lastContent = content;
                        logger.debug("检测到新剪贴板内容: {}", content.substring(0, Math.min(50, content.length())));

                        // 在JavaFX应用线程中执行回调
                        final String text = content;
                        Platform.runLater(() -> {
                            if (this.onTextCopied != null) {
                                this.onTextCopied.accept(text);
                            }
                        });
                    }

                    // 每500ms检查一次
                    Thread.sleep(500);
                } catch (InterruptedException e) {
                    Thread.currentThread().interrupt();
                    break;
                } catch (Exception e) {
                    logger.error("监听剪贴板时出错", e);
                }
            }

            logger.info("停止监听剪贴板");
        });

        monitorThread.setDaemon(true);
        monitorThread.start();

        AppConfig.getInstance().setClipboardMonitor(true);
    }

    /**
     * 停止监听剪贴板
     */
    public void stopMonitoring() {
        isMonitoring = false;
        if (monitorThread != null) {
            monitorThread.interrupt();
            monitorThread = null;
        }
        AppConfig.getInstance().setClipboardMonitor(false);
        logger.info("剪贴板监听已停止");
    }

    /**
     * 检查是否正在监听
     */
    public boolean isMonitoring() {
        return isMonitoring;
    }

    /**
     * 设置最小文本长度
     */
    public void setMinTextLength(int length) {
        this.minTextLength = length;
        AppConfig.getInstance().setClipboardMinLength(length);
    }

    /**
     * 获取剪贴板文本
     */
    public String getClipboardText() {
        try {
            if (clipboard.isDataFlavorAvailable(DataFlavor.stringFlavor)) {
                return (String) clipboard.getData(DataFlavor.stringFlavor);
            }
        } catch (Exception e) {
            logger.error("获取剪贴板内容失败", e);
        }
        return null;
    }

    /**
     * 设置剪贴板文本
     */
    public void setClipboardText(String text) {
        try {
            StringSelection selection = new StringSelection(text);
            clipboard.setContents(selection, null);
            lastContent = text; // 更新最后内容，避免触发回调
            logger.debug("已复制到剪贴板: {}", text.substring(0, Math.min(50, text.length())));
        } catch (Exception e) {
            logger.error("设置剪贴板内容失败", e);
        }
    }

    /**
     * 清除剪贴板内容
     */
    public void clearClipboard() {
        try {
            clipboard.setContents(new StringSelection(""), null);
            lastContent = "";
        } catch (Exception e) {
            logger.error("清除剪贴板失败", e);
        }
    }
}
