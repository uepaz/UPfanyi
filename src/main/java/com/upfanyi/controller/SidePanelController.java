package com.upfanyi.controller;

import com.upfanyi.util.I18nUtil;
import javafx.fxml.FXML;
import javafx.fxml.FXMLLoader;
import javafx.fxml.Initializable;
import javafx.scene.Parent;
import javafx.scene.control.Label;
import javafx.scene.layout.StackPane;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

import java.io.IOException;
import java.net.URL;
import java.util.ResourceBundle;

/**
 * 侧边面板控制器
 *
 * @author UP翻译团队
 * @version 1.0.0
 */
public class SidePanelController implements Initializable {

    private static final Logger logger = LoggerFactory.getLogger(SidePanelController.class);

    @FXML private Label panelTitle;
    @FXML private StackPane panelContent;

    private String currentPanel;
    private MainController mainController;

    @Override
    public void initialize(URL location, ResourceBundle resources) {
        logger.info("初始化侧边面板");
    }

    /**
     * 设置主控制器引用
     */
    public void setMainController(MainController mainController) {
        this.mainController = mainController;
    }

    /**
     * 获取当前显示的面板类型
     */
    public String getCurrentPanel() {
        return currentPanel;
    }

    /**
     * 显示指定类型的面板
     */
    public void showPanel(String panelType) {
        try {
            String fxmlPath = "/fxml/" + panelType + ".fxml";
            FXMLLoader loader = new FXMLLoader(getClass().getResource(fxmlPath));
            loader.setResources(ResourceBundle.getBundle("i18n.messages", I18nUtil.getCurrentLocale()));
            Parent content = loader.load();

            panelContent.getChildren().clear();
            panelContent.getChildren().add(content);

            // 设置标题并刷新面板
            switch (panelType) {
                case "history":
                    panelTitle.setText(I18nUtil.get("history.title"));
                    HistoryController historyController = loader.getController();
                    if (historyController != null) {
                        historyController.refresh();
                    }
                    break;
                case "favorite":
                    panelTitle.setText(I18nUtil.get("favorite.title"));
                    FavoriteController favoriteController = loader.getController();
                    if (favoriteController != null) {
                        favoriteController.refresh();
                    }
                    break;
                case "settings":
                    panelTitle.setText(I18nUtil.get("settings.title"));
                    SettingsController settingsController = loader.getController();
                    if (settingsController != null && mainController != null) {
                        settingsController.setMainController(mainController);
                    }
                    break;
            }

            currentPanel = panelType;
            logger.info("已加载面板: {}", panelType);

        } catch (IOException e) {
            logger.error("加载面板失败: {}", panelType, e);
        }
    }

    /**
     * 刷新当前面板
     */
    public void refresh() {
        if (currentPanel != null) {
            String temp = currentPanel;
            currentPanel = null;
            showPanel(temp);
        }
    }
}
