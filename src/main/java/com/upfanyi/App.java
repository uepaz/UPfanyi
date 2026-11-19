package com.upfanyi;

import com.upfanyi.config.AppConfig;
import com.upfanyi.config.DatabaseConfig;
import com.upfanyi.util.I18nUtil;
import com.upfanyi.view.ViewManager;
import javafx.application.Application;
import javafx.application.Platform;
import javafx.scene.Scene;
import javafx.scene.image.Image;
import javafx.stage.Stage;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

import java.util.Locale;

/**
 * UP翻译 - 桌面翻译工具主入口
 *
 * @author UP翻译团队
 * @version 1.0.0
 */
public class App extends Application {

    private static final Logger logger = LoggerFactory.getLogger(App.class);

    private static Stage primaryStage;

    /**
     * 获取主舞台
     */
    public static Stage getPrimaryStage() {
        return primaryStage;
    }

    @Override
    public void init() throws Exception {
        super.init();
        logger.info("初始化UP翻译应用...");

        // 初始化配置
        AppConfig.getInstance().init();

        // 初始化国际化
        Locale locale = AppConfig.getInstance().getLocale();
        I18nUtil.init(locale);

        logger.info("应用初始化完成");
    }

    @Override
    public void start(Stage stage) throws Exception {
        primaryStage = stage;

        logger.info("启动UP翻译主界面...");

        // 加载主界面
        ViewManager viewManager = ViewManager.getInstance();
        Scene scene = viewManager.loadMainScene();

        // 存储 MainController 引用到 scene 的 userData
        Object controller = viewManager.getController("main");
        scene.setUserData(controller);

        // 设置舞台属性
        stage.setTitle(I18nUtil.get("app.title"));
        stage.setScene(scene);
        stage.setMinWidth(420);
        stage.setMinHeight(500);
        stage.setResizable(true);

        // 设置窗口图标
        try {
            Image icon = new Image(getClass().getResourceAsStream("/images/icon.png"));
            stage.getIcons().add(icon);
        } catch (Exception e) {
            logger.warn("无法加载应用图标", e);
        }

        // 窗口关闭事件
        stage.setOnCloseRequest(event -> {
            event.consume();
            minimizeToTray();
        });

        // 显示窗口
        stage.show();

        logger.info("UP翻译启动成功");
    }

    @Override
    public void stop() throws Exception {
        logger.info("关闭UP翻译应用...");

        // 保存配置
        AppConfig.getInstance().save();

        // 关闭数据库连接
        DatabaseConfig.getInstance().close();

        // 退出应用
        Platform.exit();
        System.exit(0);
    }

    /**
     * 最小化到系统托盘
     */
    private void minimizeToTray() {
        // 如果支持系统托盘，则最小化到托盘
        if (AppConfig.getInstance().isMinimizeToTray()) {
            primaryStage.hide();
            logger.info("窗口已最小化到系统托盘");
        } else {
            Platform.exit();
            System.exit(0);
        }
    }

    /**
     * 显示主窗口
     */
    public static void showMainWindow() {
        if (primaryStage != null) {
            Platform.runLater(() -> {
                primaryStage.show();
                primaryStage.setIconified(false);
                primaryStage.toFront();
                primaryStage.requestFocus();
                primaryStage.setAlwaysOnTop(AppConfig.getInstance().isAlwaysOnTop());
            });
        }
    }

    public static void main(String[] args) {
        launch(args);
    }
}
