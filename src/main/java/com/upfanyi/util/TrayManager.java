package com.upfanyi.util;

import com.upfanyi.App;
import com.upfanyi.config.AppConfig;
import com.upfanyi.util.I18nUtil;
import javafx.application.Platform;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

import java.awt.*;
import java.awt.image.BufferedImage;
import java.awt.event.ActionListener;
import java.awt.event.MouseAdapter;
import java.awt.event.MouseEvent;
import java.net.URL;

/**
 * 系统托盘管理器
 *
 * @author UP翻译团队
 * @version 1.0.0
 */
public class TrayManager {

    private static final Logger logger = LoggerFactory.getLogger(TrayManager.class);

    private static TrayManager instance;

    private SystemTray systemTray;
    private TrayIcon trayIcon;
    private boolean isSupported;

    private TrayManager() {
        isSupported = SystemTray.isSupported();
    }

    public static synchronized TrayManager getInstance() {
        if (instance == null) {
            instance = new TrayManager();
        }
        return instance;
    }

    /**
     * 初始化系统托盘
     */
    public void init() {
        if (!isSupported) {
            logger.warn("系统托盘不支持");
            return;
        }

        try {
            systemTray = SystemTray.getSystemTray();

            // 加载托盘图标
            Image image = loadTrayImage();

            // 创建托盘图标
            trayIcon = new TrayIcon(image, I18nUtil.get("app.title"));
            trayIcon.setImageAutoSize(true);

            // 单击事件 - 显示主窗口
            trayIcon.addMouseListener(new MouseAdapter() {
                @Override
                public void mouseClicked(MouseEvent e) {
                    if (e.getButton() == MouseEvent.BUTTON1 && e.getClickCount() == 1) {
                        Platform.runLater(() -> App.showMainWindow());
                    }
                }
            });

            // 创建弹出菜单
            PopupMenu popup = createPopupMenu();
            trayIcon.setPopupMenu(popup);

            // 添加到系统托盘
            systemTray.add(trayIcon);
            logger.info("系统托盘初始化成功");
        } catch (Exception e) {
            logger.error("系统托盘初始化失败", e);
        }
    }

    /**
     * 加载托盘图标
     */
    private Image loadTrayImage() {
        try {
            URL imageUrl = getClass().getResource("/images/tray_icon.png");
            if (imageUrl != null) {
                return Toolkit.getDefaultToolkit().getImage(imageUrl);
            }
        } catch (Exception e) {
            logger.warn("加载托盘图标失败，使用默认图标", e);
        }

        // 创建默认图标（16x16的蓝色方块）
        BufferedImage image = new BufferedImage(16, 16, BufferedImage.TYPE_INT_ARGB);
        Graphics2D g2d = image.createGraphics();
        g2d.setColor(new Color(65, 105, 225)); // 皇家蓝
        g2d.fillRect(0, 0, 16, 16);
        g2d.setColor(Color.WHITE);
        g2d.setFont(new Font("Arial", Font.BOLD, 10));
        g2d.drawString("译", 2, 12);
        g2d.dispose();
        return image;
    }

    /**
     * 创建弹出菜单
     */
    private PopupMenu createPopupMenu() {
        PopupMenu popup = new PopupMenu();

        // 显示主窗口
        MenuItem showItem = new MenuItem(I18nUtil.get("tray.show"));
        showItem.addActionListener(e -> Platform.runLater(() -> App.showMainWindow()));
        popup.add(showItem);

        popup.addSeparator();

        // 开启/关闭剪贴板监听
        CheckboxMenuItem clipboardItem = new CheckboxMenuItem(I18nUtil.get("tray.clipboard_monitor"));
        clipboardItem.setState(AppConfig.getInstance().isClipboardMonitor());
        clipboardItem.addItemListener(e -> {
            boolean enabled = e.getStateChange() == 1;
            AppConfig.getInstance().setClipboardMonitor(enabled);
            if (enabled) {
                ClipboardManager.getInstance().startMonitoring(null);
            } else {
                ClipboardManager.getInstance().stopMonitoring();
            }
        });
        popup.add(clipboardItem);

        // 开启/关闭常驻顶层
        CheckboxMenuItem topItem = new CheckboxMenuItem(I18nUtil.get("tray.always_on_top"));
        topItem.setState(AppConfig.getInstance().isAlwaysOnTop());
        topItem.addItemListener(e -> {
            boolean enabled = e.getStateChange() == 1;
            AppConfig.getInstance().setAlwaysOnTop(enabled);
            if (App.getPrimaryStage() != null) {
                App.getPrimaryStage().setAlwaysOnTop(enabled);
            }
        });
        popup.add(topItem);

        popup.addSeparator();

        // 设置
        MenuItem settingsItem = new MenuItem(I18nUtil.get("tray.settings"));
        settingsItem.addActionListener(e -> {
            Platform.runLater(() -> {
                // TODO: 打开设置窗口
            });
        });
        popup.add(settingsItem);

        popup.addSeparator();

        // 退出
        MenuItem exitItem = new MenuItem(I18nUtil.get("tray.exit"));
        exitItem.addActionListener(e -> {
            shutdown();
            Platform.exit();
            System.exit(0);
        });
        popup.add(exitItem);

        return popup;
    }

    /**
     * 更新托盘菜单状态
     */
    public void updateMenuState() {
        if (trayIcon == null) {
            return;
        }

        // 重新创建菜单
        PopupMenu popup = createPopupMenu();
        trayIcon.setPopupMenu(popup);
    }

    /**
     * 显示通知
     */
    public void showNotification(String title, String message, TrayIcon.MessageType type) {
        if (trayIcon != null) {
            trayIcon.displayMessage(title, message, type);
        }
    }

    /**
     * 显示信息通知
     */
    public void showInfo(String title, String message) {
        showNotification(title, message, TrayIcon.MessageType.INFO);
    }

    /**
     * 显示错误通知
     */
    public void showError(String title, String message) {
        showNotification(title, message, TrayIcon.MessageType.ERROR);
    }

    /**
     * 显示警告通知
     */
    public void showWarning(String title, String message) {
        showNotification(title, message, TrayIcon.MessageType.WARNING);
    }

    /**
     * 关闭系统托盘
     */
    public void shutdown() {
        if (systemTray != null && trayIcon != null) {
            systemTray.remove(trayIcon);
            logger.info("系统托盘已关闭");
        }
    }

    /**
     * 检查是否支持系统托盘
     */
    public boolean isSupported() {
        return isSupported;
    }
}
