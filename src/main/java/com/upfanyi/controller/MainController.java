package com.upfanyi.controller;

import com.upfanyi.App;
import com.upfanyi.config.AppConfig;
import com.upfanyi.model.Translation;
import com.upfanyi.service.FavoriteService;
import com.upfanyi.service.TranslateService;
import com.upfanyi.util.ClipboardManager;
import com.upfanyi.util.HotkeyManager;
import com.upfanyi.util.I18nUtil;
import com.upfanyi.util.TrayManager;
import javafx.animation.PauseTransition;
import javafx.application.Platform;
import javafx.collections.FXCollections;
import javafx.collections.ObservableList;
import javafx.fxml.FXML;
import javafx.fxml.FXMLLoader;
import javafx.fxml.Initializable;
import javafx.scene.Parent;
import javafx.scene.Scene;
import javafx.scene.control.*;
import javafx.scene.input.KeyCode;
import javafx.scene.input.KeyEvent;
import javafx.scene.layout.HBox;
import javafx.scene.layout.StackPane;
import javafx.scene.layout.VBox;
import javafx.stage.Stage;
import javafx.util.Duration;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

import java.io.IOException;
import java.net.URL;
import java.util.List;
import java.util.Locale;
import java.util.ResourceBundle;

/**
 * 主界面控制器
 *
 * @author UP翻译团队
 * @version 1.0.0
 */
public class MainController implements Initializable {

    private static final Logger logger = LoggerFactory.getLogger(MainController.class);

    @FXML private HBox rootPane;
    @FXML private VBox mainContent;
    @FXML private StackPane sidePanelContainer;
    @FXML private TextArea sourceTextArea;
    @FXML private TextArea resultTextArea;
    @FXML private ComboBox<String> apiComboBox;
    @FXML private ComboBox<String> sourceLangComboBox;
    @FXML private ComboBox<String> targetLangComboBox;
    @FXML private Button translateButton;
    @FXML private Button swapButton;
    @FXML private Button copyButton;
    @FXML private Button favoriteButton;
    @FXML private Button historyButton;
    @FXML private Button settingsButton;
    @FXML private Label statusLabel;

    private final TranslateService translateService;
    private final FavoriteService favoriteService;
    private final AppConfig appConfig;
    private final ClipboardManager clipboardManager;

    private String currentSourceText;
    private String currentTranslatedText;

    private SidePanelController sidePanelController;
    private Stage primaryStage;

    // 自动翻译相关
    private PauseTransition autoTranslateDelay;
    private static final int AUTO_TRANSLATE_DELAY_MS = 800;
    private boolean autoTranslateEnabled = false;

    // 窗口尺寸常量
    private static final double MAIN_WIDTH = 420;
    private static final double SIDE_PANEL_WIDTH = 420;
    private static final double WINDOW_HEIGHT = 500;

    public MainController() {
        this.translateService = TranslateService.getInstance();
        this.favoriteService = FavoriteService.getInstance();
        this.appConfig = AppConfig.getInstance();
        this.clipboardManager = ClipboardManager.getInstance();
    }

    @Override
    public void initialize(URL location, ResourceBundle resources) {
        logger.info("初始化主界面控制器");

        // 初始化API选择器
        initApiComboBox();

        // 初始化语言选择器
        initLanguageComboBoxes();

        // 初始化快捷键
        initHotkeys();

        // 初始化系统托盘
        initTray();

        // 初始化剪贴板监听
        initClipboardMonitor();

        // 初始化自动翻译
        initAutoTranslate();

        // 设置窗口属性
        setupWindow();

        // 设置状态栏
        updateStatus(I18nUtil.get("status.ready"));
    }

    /**
     * 初始化API选择器
     */
    private void initApiComboBox() {
        apiComboBox.setCellFactory(listView -> new ListCell<>() {
            @Override
            protected void updateItem(String item, boolean empty) {
                super.updateItem(item, empty);
                if (empty || item == null) {
                    setText(null);
                } else {
                    setText(translateService.getApiDisplayName(item));
                }
            }
        });
        apiComboBox.setButtonCell(new ListCell<>() {
            @Override
            protected void updateItem(String item, boolean empty) {
                super.updateItem(item, empty);
                if (empty || item == null) {
                    setText(null);
                } else {
                    setText(translateService.getApiDisplayName(item));
                }
            }
        });

        refreshApiComboBox();

        // 添加API选择监听器，保存用户选择
        apiComboBox.valueProperty().addListener((obs, oldVal, newVal) -> {
            if (newVal != null) {
                appConfig.setDefaultApi(newVal);
                appConfig.save();
            }
        });
    }

    /**
     * 刷新API选择器
     */
    public void refreshApiComboBox() {
        List<String> apis = translateService.getAvailableApis();
        ObservableList<String> apiList = FXCollections.observableArrayList(apis);

        String selectedApi = apiComboBox.getValue();
        apiComboBox.setItems(apiList);

        // 选择默认API
        String defaultApi = appConfig.getDefaultApi();
        if (apis.contains(selectedApi)) {
            apiComboBox.setValue(selectedApi);
        } else if (apis.contains(defaultApi)) {
            apiComboBox.setValue(defaultApi);
        } else if (!apis.isEmpty()) {
            apiComboBox.setValue(apis.get(0));
        } else {
            apiComboBox.setValue(null);
        }
    }

    /**
     * 初始化语言选择器
     */
    private void initLanguageComboBoxes() {
        // 源语言选项
        ObservableList<String> sourceLangs = FXCollections.observableArrayList(
                I18nUtil.get("lang.auto"),
                I18nUtil.get("lang.zh"),
                I18nUtil.get("lang.en"),
                I18nUtil.get("lang.ja"),
                I18nUtil.get("lang.ko"),
                I18nUtil.get("lang.fr"),
                I18nUtil.get("lang.de"),
                I18nUtil.get("lang.es"),
                I18nUtil.get("lang.ru")
        );
        sourceLangComboBox.setItems(sourceLangs);

        // 目标语言选项
        ObservableList<String> targetLangs = FXCollections.observableArrayList(
                I18nUtil.get("lang.zh"),
                I18nUtil.get("lang.en"),
                I18nUtil.get("lang.ja"),
                I18nUtil.get("lang.ko"),
                I18nUtil.get("lang.fr"),
                I18nUtil.get("lang.de"),
                I18nUtil.get("lang.es"),
                I18nUtil.get("lang.ru")
        );
        targetLangComboBox.setItems(targetLangs);

        // 读取保存的语言设置
        String savedSourceLang = appConfig.getDefaultSourceLang();
        String savedTargetLang = appConfig.getDefaultTargetLang();
        sourceLangComboBox.setValue(getLanguageDisplay(savedSourceLang));
        targetLangComboBox.setValue(getLanguageDisplay(savedTargetLang));

        // 添加监听器保存用户选择
        sourceLangComboBox.valueProperty().addListener((obs, oldVal, newVal) -> {
            if (newVal != null) {
                appConfig.setDefaultSourceLang(getLanguageCode(newVal));
                appConfig.save();
            }
        });

        targetLangComboBox.valueProperty().addListener((obs, oldVal, newVal) -> {
            if (newVal != null) {
                appConfig.setDefaultTargetLang(getLanguageCode(newVal));
                appConfig.save();
            }
        });
    }

    /**
     * 初始化快捷键
     */
    private void initHotkeys() {
        // Enter键翻译
        sourceTextArea.addEventFilter(KeyEvent.KEY_PRESSED, event -> {
            if (event.getCode() == KeyCode.ENTER && !event.isShiftDown()) {
                handleTranslate();
                event.consume();
            }
        });

        // 注册全局快捷键
        HotkeyManager hotkeyManager = HotkeyManager.getInstance();
        hotkeyManager.init();
        hotkeyManager.registerHotkey("ctrl shift t", () -> Platform.runLater(() -> App.showMainWindow()));
        hotkeyManager.registerHotkey("ctrl shift c", () -> {
            String clipboardText = clipboardManager.getClipboardText();
            if (clipboardText != null && !clipboardText.isEmpty()) {
                Platform.runLater(() -> {
                    sourceTextArea.setText(clipboardText);
                    handleTranslate();
                    App.showMainWindow();
                });
            }
        });
    }

    /**
     * 初始化系统托盘
     */
    private void initTray() {
        TrayManager trayManager = TrayManager.getInstance();
        if (trayManager.isSupported()) {
            trayManager.init();
        }
    }

    /**
     * 初始化剪贴板监听
     */
    private void initClipboardMonitor() {
        clipboardManager.setMinTextLength(appConfig.getClipboardMinLength());
        if (appConfig.isClipboardMonitor()) {
            clipboardManager.startMonitoring(this::handleClipboardText);
        }
    }

    /**
     * 更新剪贴板监听状态
     */
    public void updateClipboardMonitor(boolean enabled, int minTextLength) {
        clipboardManager.setMinTextLength(minTextLength);

        if (enabled) {
            clipboardManager.startMonitoring(this::handleClipboardText);
        } else {
            clipboardManager.stopMonitoring();
        }

        appConfig.setClipboardMonitor(enabled);
        appConfig.save();
    }

    /**
     * 初始化自动翻译
     */
    private void initAutoTranslate() {
        autoTranslateEnabled = appConfig.isAutoTranslate();
        autoTranslateDelay = new PauseTransition(Duration.millis(AUTO_TRANSLATE_DELAY_MS));
        autoTranslateDelay.setOnFinished(event -> {
            if (!sourceTextArea.getText().trim().isEmpty() && autoTranslateEnabled) {
                handleTranslate();
            }
        });

        sourceTextArea.textProperty().addListener((obs, oldVal, newVal) -> {
            if (autoTranslateEnabled && !newVal.trim().isEmpty()) {
                autoTranslateDelay.playFromStart();
            }
        });
    }

    /**
     * 更新自动翻译状态
     */
    public void updateAutoTranslate(boolean enabled) {
        this.autoTranslateEnabled = enabled;
        appConfig.setAutoTranslate(enabled);
        appConfig.save();
    }

    /**
     * 获取自动翻译状态
     */
    public boolean isAutoTranslateEnabled() {
        return autoTranslateEnabled;
    }

    /**
     * 处理剪贴板文本
     */
    private void handleClipboardText(String text) {
        Platform.runLater(() -> {
            sourceTextArea.setText(text);
            handleTranslate();
            App.showMainWindow();
        });
    }

    /**
     * 设置窗口属性
     */
    private void setupWindow() {
        // 常驻顶层 - 延迟执行，等待窗口初始化完成
        Platform.runLater(() -> {
            Stage stage = (Stage) rootPane.getScene().getWindow();
            if (stage != null) {
                this.primaryStage = stage;
                stage.setAlwaysOnTop(appConfig.isAlwaysOnTop());
                // 设置窗口最小尺寸
                stage.setMinWidth(MAIN_WIDTH);
                stage.setMinHeight(WINDOW_HEIGHT);

                // 根据内容自适应窗口大小
                rootPane.applyCss();
                rootPane.layout();

                double contentWidth = rootPane.prefWidth(-1);
                double contentHeight = rootPane.prefHeight(contentWidth);

                double decorWidth = stage.getWidth() - stage.getScene().getWidth();
                double decorHeight = stage.getHeight() - stage.getScene().getHeight();

                stage.setWidth(contentWidth + decorWidth);
                stage.setHeight(contentHeight + decorHeight);
            }
        });
    }

    /**
     * 更新窗口置顶状态（供设置页面调用）
     */
    public void updateAlwaysOnTop() {
        Platform.runLater(() -> {
            Stage stage = (Stage) rootPane.getScene().getWindow();
            if (stage != null) {
                stage.setAlwaysOnTop(appConfig.isAlwaysOnTop());
            }
        });
    }

    /**
     * 显示侧边面板（重复点击同一面板则关闭）
     */
    public void showSidePanel(String panelType) {
        // 如果当前面板已显示且是同一类型，则关闭
        if (sidePanelContainer.isVisible() &&
            sidePanelController != null &&
            panelType.equals(sidePanelController.getCurrentPanel())) {
            hideSidePanel();
            return;
        }

        if (sidePanelController == null) {
            // 获取SidePanelController
            try {
                FXMLLoader loader = new FXMLLoader(getClass().getResource("/fxml/side_panel.fxml"));
                loader.setResources(ResourceBundle.getBundle("i18n.messages", I18nUtil.getCurrentLocale()));
                Parent sidePanel = loader.load();
                sidePanelController = loader.getController();
                sidePanelController.setMainController(this);
                sidePanelContainer.getChildren().clear();
                sidePanelContainer.getChildren().add(sidePanel);
            } catch (IOException e) {
                logger.error("加载侧边面板失败", e);
                return;
            }
        }

        sidePanelController.showPanel(panelType);
        sidePanelContainer.setVisible(true);
        sidePanelContainer.setManaged(true);

        // 强制布局计算后调整窗口大小
        requestLayoutAndResize();
    }

    /**
     * 请求布局更新并调整窗口大小
     */
    private void requestLayoutAndResize() {
        // 先请求布局更新
        rootPane.requestLayout();

        // 使用双层 Platform.runLater 确保布局完成后再调整窗口
        Platform.runLater(() -> {
            rootPane.applyCss();
            rootPane.layout();

            Platform.runLater(() -> {
                Stage stage = primaryStage != null ? primaryStage :
                              (Stage) rootPane.getScene().getWindow();
                if (stage != null) {
                    // 使用固定计算：主内容宽度 + 侧边面板宽度
                    double mainWidth = mainContent.getWidth();
                    double sideWidth = sidePanelContainer.isVisible() ? SIDE_PANEL_WIDTH : 0;

                    // 获取窗口装饰边框
                    double decorWidth = stage.getWidth() - stage.getScene().getWidth();
                    double decorHeight = stage.getHeight() - stage.getScene().getHeight();

                    double newWidth = mainWidth + sideWidth + decorWidth;
                    double newHeight = Math.max(stage.getHeight(), stage.getMinHeight());

                    logger.info("显示侧边面板，主内容宽度: {}, 侧边面板宽度: {}, 新窗口宽度: {}", mainWidth, sideWidth, newWidth);
                    stage.setWidth(newWidth);
                    stage.setHeight(newHeight);
                }
            });
        });
    }

    /**
     * 隐藏侧边面板
     */
    public void hideSidePanel() {
        sidePanelContainer.setVisible(false);
        sidePanelContainer.setManaged(false);

        // 强制布局计算后调整窗口大小
        rootPane.requestLayout();

        Platform.runLater(() -> {
            rootPane.applyCss();
            rootPane.layout();

            Platform.runLater(() -> {
                Stage stage = primaryStage != null ? primaryStage :
                              (Stage) rootPane.getScene().getWindow();
                if (stage != null) {
                    // 使用固定计算：仅主内容宽度
                    double mainWidth = mainContent.getWidth();

                    // 获取窗口装饰边框
                    double decorWidth = stage.getWidth() - stage.getScene().getWidth();
                    double decorHeight = stage.getHeight() - stage.getScene().getHeight();

                    double newWidth = mainWidth + decorWidth;
                    double newHeight = Math.max(stage.getHeight(), stage.getMinHeight());

                    logger.info("隐藏侧边面板，主内容宽度: {}, 新窗口宽度: {}", mainWidth, newWidth);
                    stage.setWidth(newWidth);
                    stage.setHeight(newHeight);
                }
            });
        });
    }

    /**
     * 翻译按钮点击事件
     */
    @FXML
    private void handleTranslate() {
        String text = sourceTextArea.getText().trim();
        if (text.isEmpty()) {
            updateStatus(I18nUtil.get("status.empty_source"));
            return;
        }

        String api = apiComboBox.getValue();
        String sourceLang = getLanguageCode(sourceLangComboBox.getValue());
        String targetLang = getLanguageCode(targetLangComboBox.getValue());

        translateButton.setDisable(true);
        updateStatus(I18nUtil.get("status.translating"));

        // 异步翻译
        new Thread(() -> {
            try {
                Translation result = translateService.translate(text, sourceLang, targetLang, api);
                currentSourceText = text;
                currentTranslatedText = result.getTranslatedText();

                Platform.runLater(() -> {
                    resultTextArea.setText(currentTranslatedText);
                    updateStatus(I18nUtil.get("status.success"));
                    translateButton.setDisable(false);

                    // 更新收藏按钮状态
                    updateFavoriteButton();
                });
            } catch (Exception e) {
                logger.error("翻译失败", e);
                Platform.runLater(() -> {
                    resultTextArea.setText(I18nUtil.get("error.translate_failed") + ": " + e.getMessage());
                    updateStatus(I18nUtil.get("status.error"));
                    translateButton.setDisable(false);
                });
            }
        }).start();
    }

    /**
     * 交换语言按钮点击事件
     */
    @FXML
    private void handleSwapLanguage() {
        String sourceLang = sourceLangComboBox.getValue();
        String targetLang = targetLangComboBox.getValue();

        // 不交换"自动检测"
        if (!sourceLang.equals(I18nUtil.get("lang.auto"))) {
            sourceLangComboBox.setValue(targetLang);
            targetLangComboBox.setValue(sourceLang);

            // 同时交换文本
            String sourceText = sourceTextArea.getText();
            String resultText = resultTextArea.getText();
            sourceTextArea.setText(resultText);
            resultTextArea.setText(sourceText);
        }
    }

    /**
     * 复制按钮点击事件
     */
    @FXML
    private void handleCopy() {
        String text = resultTextArea.getText();
        if (!text.isEmpty()) {
            clipboardManager.setClipboardText(text);
            updateStatus(I18nUtil.get("status.copied"));
        }
    }

    /**
     * 收藏按钮点击事件
     */
    @FXML
    private void handleFavorite() {
        if (currentSourceText == null || currentTranslatedText == null) {
            return;
        }

        if (favoriteService.isFavorited(currentSourceText)) {
            favoriteService.removeFavoriteByWord(currentSourceText);
            updateStatus(I18nUtil.get("status.favorite_removed"));
        } else {
            favoriteService.addFavorite(currentSourceText, currentTranslatedText);
            updateStatus(I18nUtil.get("status.favorite_added"));
        }
        updateFavoriteButton();
    }

    /**
     * 历史记录按钮点击事件
     */
    @FXML
    private void handleHistory() {
        showSidePanel("history");
    }

    /**
     * 收藏列表按钮点击事件
     */
    @FXML
    private void handleFavoriteList() {
        showSidePanel("favorite");
    }

    /**
     * 设置按钮点击事件
     */
    @FXML
    private void handleSettings() {
        showSidePanel("settings");
    }

    /**
     * 更新收藏按钮状态
     */
    private void updateFavoriteButton() {
        if (currentSourceText != null && favoriteService.isFavorited(currentSourceText)) {
            favoriteButton.setText(I18nUtil.get("button.unfavorite"));
        } else {
            favoriteButton.setText(I18nUtil.get("button.favorite"));
        }
    }

    /**
     * 更新状态栏
     */
    private void updateStatus(String message) {
        statusLabel.setText(message);
    }

    /**
     * 获取语言代码
     */
    private String getLanguageCode(String language) {
        if (language.equals(I18nUtil.get("lang.auto"))) return "auto";
        if (language.equals(I18nUtil.get("lang.zh"))) return "zh";
        if (language.equals(I18nUtil.get("lang.en"))) return "en";
        if (language.equals(I18nUtil.get("lang.ja"))) return "ja";
        if (language.equals(I18nUtil.get("lang.ko"))) return "ko";
        if (language.equals(I18nUtil.get("lang.fr"))) return "fr";
        if (language.equals(I18nUtil.get("lang.de"))) return "de";
        if (language.equals(I18nUtil.get("lang.es"))) return "es";
        if (language.equals(I18nUtil.get("lang.ru"))) return "ru";
        return "auto";
    }

    /**
     * 根据语言代码获取显示文本
     */
    private String getLanguageDisplay(String code) {
        if ("auto".equals(code)) return I18nUtil.get("lang.auto");
        if ("zh".equals(code)) return I18nUtil.get("lang.zh");
        if ("en".equals(code)) return I18nUtil.get("lang.en");
        if ("ja".equals(code)) return I18nUtil.get("lang.ja");
        if ("ko".equals(code)) return I18nUtil.get("lang.ko");
        if ("fr".equals(code)) return I18nUtil.get("lang.fr");
        if ("de".equals(code)) return I18nUtil.get("lang.de");
        if ("es".equals(code)) return I18nUtil.get("lang.es");
        if ("ru".equals(code)) return I18nUtil.get("lang.ru");
        return I18nUtil.get("lang.auto");
    }

    /**
     * 刷新侧边面板（用于语言切换后）
     */
    public void refreshSidePanel() {
        if (sidePanelController != null) {
            sidePanelController.refresh();
        }
    }
}
