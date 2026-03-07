package com.upfanyi.view;

import com.upfanyi.util.I18nUtil;
import javafx.fxml.FXMLLoader;
import javafx.scene.Parent;
import javafx.scene.Scene;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

import java.io.IOException;
import java.util.HashMap;
import java.util.Map;

/**
 * 视图管理器
 * 负责加载和管理FXML视图
 *
 * @author UP翻译团队
 * @version 1.0.0
 */
public class ViewManager {

    private static final Logger logger = LoggerFactory.getLogger(ViewManager.class);

    private static ViewManager instance;

    private final Map<String, ViewInfo> viewCache;
    private Scene mainScene;
    private double windowWidth = 420;
    private double windowHeight = 500;

    private ViewManager() {
        viewCache = new HashMap<>();
    }

    public static synchronized ViewManager getInstance() {
        if (instance == null) {
            instance = new ViewManager();
        }
        return instance;
    }

    /**
     * 加载主场景
     */
    public Scene loadMainScene() throws IOException {
        logger.info("加载主界面");
        Parent root = loadView("main");
        mainScene = new Scene(root, windowWidth, windowHeight);
        mainScene.getStylesheets().add(getClass().getResource("/css/style.css").toExternalForm());
        return mainScene;
    }

    /**
     * 加载视图
     *
     * @param viewName 视图名称（不含.fxml扩展名）
     * @return 视图的根节点
     */
    public Parent loadView(String viewName) throws IOException {
        ViewInfo viewInfo = viewCache.get(viewName);

        if (viewInfo == null) {
            logger.debug("加载视图: {}", viewName);
            FXMLLoader loader = new FXMLLoader(getClass().getResource("/fxml/" + viewName + ".fxml"));
            loader.setResources(I18nUtil.getCurrentLocale() != null ?
                    java.util.ResourceBundle.getBundle("i18n.messages", I18nUtil.getCurrentLocale()) :
                    java.util.ResourceBundle.getBundle("i18n.messages"));

            Parent root = loader.load();
            Object controller = loader.getController();

            viewInfo = new ViewInfo(root, controller, loader);
            viewCache.put(viewName, viewInfo);
        }

        return viewInfo.getRoot();
    }

    /**
     * 获取视图控制器
     */
    @SuppressWarnings("unchecked")
    public <T> T getController(String viewName) {
        ViewInfo viewInfo = viewCache.get(viewName);
        return viewInfo != null ? (T) viewInfo.getController() : null;
    }

    /**
     * 重新加载视图（用于语言切换等场景）
     */
    public Parent reloadView(String viewName) throws IOException {
        viewCache.remove(viewName);
        return loadView(viewName);
    }

    /**
     * 清除视图缓存
     */
    public void clearCache() {
        viewCache.clear();
        logger.debug("视图缓存已清除");
    }

    /**
     * 获取主场景
     */
    public Scene getMainScene() {
        return mainScene;
    }

    /**
     * 设置默认窗口大小
     */
    public void setDefaultSize(double width, double height) {
        this.windowWidth = width;
        this.windowHeight = height;
    }

    /**
     * 获取默认窗口宽度
     */
    public double getWindowWidth() {
        return windowWidth;
    }

    /**
     * 获取默认窗口高度
     */
    public double getWindowHeight() {
        return windowHeight;
    }

    /**
     * 视图信息内部类
     */
    private static class ViewInfo {
        private final Parent root;
        private final Object controller;
        private final FXMLLoader loader;

        public ViewInfo(Parent root, Object controller, FXMLLoader loader) {
            this.root = root;
            this.controller = controller;
            this.loader = loader;
        }

        public Parent getRoot() {
            return root;
        }

        public Object getController() {
            return controller;
        }

        public FXMLLoader getLoader() {
            return loader;
        }
    }
}
