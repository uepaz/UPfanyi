package com.upfanyi.util;

import com.upfanyi.App;
import com.upfanyi.config.AppConfig;
import javafx.application.Platform;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

import javax.swing.*;
import javax.swing.event.PopupMenuListener;
import javax.swing.event.PopupMenuEvent;
import java.awt.*;
import java.awt.event.ItemEvent;
import java.awt.event.MouseAdapter;
import java.awt.event.MouseEvent;
import java.awt.image.BufferedImage;
import java.net.URL;

/**
 * 系统托盘管理器
 *
 * @author UP 翻译团队
 * @version 1.0.0
 */
public class TrayManager {

    private static final Logger logger = LoggerFactory.getLogger(TrayManager.class);

    private static TrayManager instance;

    private SystemTray systemTray;
    private TrayIcon trayIcon;
    private boolean isSupported;

    // 自定义 Swing 弹出菜单
    private JPopupMenu popupMenu;
    // 用于显示菜单的隐藏窗口
    private JDialog hiddenDialog;

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

            // 创建托盘图标（不设置原生菜单）
            trayIcon = new TrayIcon(image, I18nUtil.get("app.title"));
            trayIcon.setImageAutoSize(true);

            // 创建自定义菜单
            popupMenu = createPopupMenu();

            // 创建隐藏窗口用于显示菜单
            hiddenDialog = new JDialog();
            hiddenDialog.setUndecorated(true);
            hiddenDialog.setSize(1, 1);
            hiddenDialog.setType(Window.Type.UTILITY);
            hiddenDialog.setAlwaysOnTop(true);

            // 鼠标监听器 - 左键显示主窗口，右键显示自定义菜单
            trayIcon.addMouseListener(new MouseAdapter() {
                @Override
                public void mouseClicked(MouseEvent e) {
                    if (e.getButton() == MouseEvent.BUTTON1 && e.getClickCount() == 1) {
                        // 左键单击 - 显示主窗口
                        Platform.runLater(() -> App.showMainWindow());
                    }
                }

                @Override
                public void mousePressed(MouseEvent e) {
                    if (SwingUtilities.isRightMouseButton(e)) {
                        showPopupMenu();
                    }
                }
            });

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

        // 创建默认图标（16x16 的蓝色方块）
        BufferedImage image = new BufferedImage(16, 16, BufferedImage.TYPE_INT_ARGB);
        Graphics2D g2d = image.createGraphics();
        g2d.setColor(new Color(65, 105, 225)); // 皇家蓝
        g2d.fillRect(0, 0, 16, 16);
        g2d.setColor(Color.WHITE);
        g2d.setFont(new Font("Microsoft YaHei", Font.BOLD, 10));
        g2d.drawString("译", 2, 12);
        g2d.dispose();
        return image;
    }

    /**
     * 在鼠标位置显示弹出菜单
     */
    private void showPopupMenu() {
        SwingUtilities.invokeLater(() -> {
            if (popupMenu == null) {
                popupMenu = createPopupMenu();
            }

            // 更新菜单状态
            updateMenuState();

            // 获取鼠标位置
            Point mousePoint = MouseInfo.getPointerInfo().getLocation();

            // 设置隐藏窗口位置并显示
            hiddenDialog.setLocation(mousePoint.x, mousePoint.y);
            hiddenDialog.setVisible(true);
            hiddenDialog.toFront();

            // 在隐藏窗口上显示菜单（从鼠标位置开始）
            popupMenu.show(hiddenDialog.getContentPane(), 0, 0);
        });
    }

    /**
     * 创建自定义弹出菜单（使用 Swing 组件，支持中文字体）
     */
    private JPopupMenu createPopupMenu() {
        JPopupMenu menu = new JPopupMenu();

        // 设置中文字体 - 这是修复乱码的关键
        Font chineseFont = new Font("Microsoft YaHei", Font.PLAIN, 12);

        // 监听菜单取消事件，隐藏隐藏窗口
        menu.addPopupMenuListener(new javax.swing.event.PopupMenuListener() {
            @Override
            public void popupMenuWillBecomeVisible(javax.swing.event.PopupMenuEvent e) {}

            @Override
            public void popupMenuWillBecomeInvisible(javax.swing.event.PopupMenuEvent e) {
                SwingUtilities.invokeLater(() -> {
                    if (hiddenDialog != null) {
                        hiddenDialog.setVisible(false);
                    }
                });
            }

            @Override
            public void popupMenuCanceled(javax.swing.event.PopupMenuEvent e) {
                SwingUtilities.invokeLater(() -> {
                    if (hiddenDialog != null) {
                        hiddenDialog.setVisible(false);
                    }
                });
            }
        });

        // 显示主窗口
        JMenuItem showItem = new JMenuItem(I18nUtil.get("tray.show"));
        showItem.setFont(chineseFont);
        showItem.addActionListener(e -> Platform.runLater(() -> App.showMainWindow()));
        menu.add(showItem);

        menu.addSeparator();

        // 开启/关闭剪贴板监听
        JCheckBoxMenuItem clipboardItem = new JCheckBoxMenuItem(I18nUtil.get("tray.clipboard_monitor"));
        clipboardItem.setFont(chineseFont);
        clipboardItem.setSelected(AppConfig.getInstance().isClipboardMonitor());
        clipboardItem.addItemListener(e -> {
            boolean enabled = e.getStateChange() == ItemEvent.SELECTED;
            AppConfig.getInstance().setClipboardMonitor(enabled);
            // 在 JavaFX 线程上启动/停止剪贴板监听
            Platform.runLater(() -> {
                if (enabled) {
                    ClipboardManager.getInstance().startMonitoring(null);
                } else {
                    ClipboardManager.getInstance().stopMonitoring();
                }
            });
        });
        menu.add(clipboardItem);

        // 开启/关闭常驻顶层
        JCheckBoxMenuItem topItem = new JCheckBoxMenuItem(I18nUtil.get("tray.always_on_top"));
        topItem.setFont(chineseFont);
        topItem.setSelected(AppConfig.getInstance().isAlwaysOnTop());
        topItem.addItemListener(e -> {
            boolean enabled = e.getStateChange() == ItemEvent.SELECTED;
            AppConfig.getInstance().setAlwaysOnTop(enabled);
            // 必须在 JavaFX 线程上设置窗口置顶
            Platform.runLater(() -> {
                if (App.getPrimaryStage() != null) {
                    App.getPrimaryStage().setAlwaysOnTop(enabled);
                }
            });
        });
        menu.add(topItem);

        menu.addSeparator();

        // 设置
        JMenuItem settingsItem = new JMenuItem(I18nUtil.get("tray.settings"));
        settingsItem.setFont(chineseFont);
        settingsItem.addActionListener(e -> {
            Platform.runLater(() -> {
                try {
                    // 先显示主窗口
                    App.showMainWindow();

                    // 然后打开设置面板
                    var scene = App.getPrimaryStage().getScene();
                    if (scene != null) {
                        Object controller = scene.getUserData();
                        if (controller instanceof com.upfanyi.controller.MainController) {
                            ((com.upfanyi.controller.MainController) controller).showSidePanel("settings");
                            return;
                        }
                    }
                } catch (Exception ex) {
                    logger.error("打开设置面板失败", ex);
                    App.showMainWindow();
                }
            });
        });
        menu.add(settingsItem);

        menu.addSeparator();

        // 退出
        JMenuItem exitItem = new JMenuItem(I18nUtil.get("tray.exit"));
        exitItem.setFont(chineseFont);
        exitItem.addActionListener(e -> {
            shutdown();
            Platform.exit();
            System.exit(0);
        });
        menu.add(exitItem);

        // 设置菜单样式 - 使其更像系统菜单
        menu.setBorder(BorderFactory.createLineBorder(new Color(200, 200, 200), 1));

        return menu;
    }

    /**
     * 更新托盘菜单状态
     */
    public void updateMenuState() {
        if (popupMenu == null) {
            return;
        }

        // 更新复选框状态
        for (int i = 0; i < popupMenu.getComponentCount(); i++) {
            Component comp = popupMenu.getComponent(i);
            if (comp instanceof JCheckBoxMenuItem) {
                JCheckBoxMenuItem cbItem = (JCheckBoxMenuItem) comp;
                if (cbItem.getText().equals(I18nUtil.get("tray.clipboard_monitor"))) {
                    cbItem.setSelected(AppConfig.getInstance().isClipboardMonitor());
                } else if (cbItem.getText().equals(I18nUtil.get("tray.always_on_top"))) {
                    cbItem.setSelected(AppConfig.getInstance().isAlwaysOnTop());
                }
            }
        }
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
        if (hiddenDialog != null) {
            hiddenDialog.dispose();
        }
    }

    /**
     * 检查是否支持系统托盘
     */
    public boolean isSupported() {
        return isSupported;
    }
}
