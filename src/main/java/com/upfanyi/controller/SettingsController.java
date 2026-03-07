package com.upfanyi.controller;

import com.upfanyi.api.ApiFactory;
import com.upfanyi.api.TranslateApi;
import com.upfanyi.config.AppConfig;
import com.upfanyi.dao.ApiConfigDao;
import com.upfanyi.model.ApiConfig;
import com.upfanyi.model.ApiConfigField;
import com.upfanyi.model.Translation;
import com.upfanyi.util.I18nUtil;
import javafx.application.Platform;
import javafx.fxml.FXML;
import javafx.fxml.Initializable;
import javafx.scene.Node;
import javafx.scene.control.*;
import javafx.scene.layout.HBox;
import javafx.scene.layout.VBox;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

import java.net.URL;
import java.util.*;
import java.util.concurrent.ConcurrentHashMap;

/**
 * 设置界面控制器
 *
 * @author UP翻译团队
 * @version 1.0.0
 */
public class SettingsController implements Initializable {

    private static final Logger logger = LoggerFactory.getLogger(SettingsController.class);

    @FXML private ComboBox<String> languageComboBox;
    @FXML private CheckBox alwaysOnTopCheckBox;
    @FXML private CheckBox minimizeToTrayCheckBox;
    @FXML private CheckBox clipboardMonitorCheckBox;
    @FXML private CheckBox autoTranslateCheckBox;
    @FXML private Spinner<Integer> clipboardMinLengthSpinner;

    @FXML private VBox apiConfigContainer;

    @FXML private Button saveButton;
    @FXML private Button cancelButton;

    private final AppConfig appConfig;
    private final ApiConfigDao apiConfigDao;
    private MainController mainController;

    // 动态API配置UI组件
    private final Map<String, Map<String, TextInputControl>> apiFieldsMap = new ConcurrentHashMap<>();
    private final Map<String, Label> apiStatusLabels = new ConcurrentHashMap<>();
    private final Map<String, Label> apiTestLabels = new ConcurrentHashMap<>();

    public SettingsController() {
        this.appConfig = AppConfig.getInstance();
        this.apiConfigDao = new ApiConfigDao();
    }

    /**
     * 设置主控制器引用
     */
    public void setMainController(MainController mainController) {
        this.mainController = mainController;
    }

    @Override
    public void initialize(URL location, ResourceBundle resources) {
        logger.info("初始化设置界面");

        // 初始化语言选择
        initLanguageComboBox();

        // 加载当前设置
        loadSettings();

        // 动态生成API配置UI
        buildApiConfigUI();
    }

    /**
     * 初始化语言选择框
     */
    private void initLanguageComboBox() {
        languageComboBox.getItems().addAll(
                "中文",
                "English"
        );

        // 设置当前语言
        Locale currentLocale = appConfig.getLocale();
        if (currentLocale.getLanguage().equals("zh")) {
            languageComboBox.setValue("中文");
        } else {
            languageComboBox.setValue("English");
        }
    }

    /**
     * 加载设置
     */
    private void loadSettings() {
        alwaysOnTopCheckBox.setSelected(appConfig.isAlwaysOnTop());
        minimizeToTrayCheckBox.setSelected(appConfig.isMinimizeToTray());
        clipboardMonitorCheckBox.setSelected(appConfig.isClipboardMonitor());
        autoTranslateCheckBox.setSelected(appConfig.isAutoTranslate());
        clipboardMinLengthSpinner.getValueFactory().setValue(appConfig.getClipboardMinLength());
    }

    /**
     * 动态生成API配置UI
     */
    private void buildApiConfigUI() {
        if (apiConfigContainer == null) {
            logger.warn("API配置容器未找到");
            return;
        }

        apiConfigContainer.getChildren().clear();
        apiFieldsMap.clear();
        apiStatusLabels.clear();
        apiTestLabels.clear();

        List<String> apiNames = ApiFactory.getInstance().getRegisteredApis();
        for (String apiName : apiNames) {
            TranslateApi api = ApiFactory.getInstance().getApi(apiName);
            if (api == null) continue;

            TitledPane apiPane = createApiConfigPane(api);
            apiPane.setExpanded(false); // 所有API默认收起

            apiConfigContainer.getChildren().add(apiPane);
        }
    }

    /**
     * 创建API配置面板
     */
    private TitledPane createApiConfigPane(TranslateApi api) {
        String apiName = api.getApiName();
        String displayName = api.getDisplayName();

        VBox content = new VBox(8);
        content.setStyle("-fx-padding: 8;");

        // 状态行
        HBox statusRow = new HBox(8);
        statusRow.setStyle("-fx-alignment: CENTER_LEFT;");
        Label nameLabel = new Label(displayName);
        nameLabel.setStyle("-fx-font-weight: bold; -fx-text-fill: #4a90d9; -fx-min-width: 80;");
        Label statusLabel = new Label();
        updateApiStatus(apiName, statusLabel);
        apiStatusLabels.put(apiName, statusLabel);
        statusRow.getChildren().addAll(nameLabel, statusLabel);
        content.getChildren().add(statusRow);

        // 描述
        String description = api.getConfigDescription();
        if (description != null && !description.isEmpty()) {
            Label descLabel = new Label(description);
            descLabel.setStyle("-fx-text-fill: #6c757d; -fx-font-size: 11px; -fx-wrap-text: true;");
            descLabel.setWrapText(true);
            content.getChildren().add(descLabel);
        }

        // 配置字段
        Map<String, TextInputControl> fields = new HashMap<>();
        for (ApiConfigField field : api.getConfigFields()) {
            HBox fieldRow = createFieldRow(api, field, fields);
            content.getChildren().add(fieldRow);
        }
        apiFieldsMap.put(apiName, fields);

        // 加载已保存的配置
        loadApiConfigValues(api, fields);

        // 测试按钮行
        HBox testRow = new HBox(8);
        testRow.setStyle("-fx-alignment: CENTER_LEFT;");
        Button testButton = new Button("测试连接");
        Label testResultLabel = new Label();
        apiTestLabels.put(apiName, testResultLabel);
        testButton.setOnAction(e -> testApi(api, fields, testResultLabel));
        testRow.getChildren().addAll(testButton, testResultLabel);
        content.getChildren().add(testRow);

        // 分隔线
        if (!apiName.equals("llm")) {
            Separator separator = new Separator();
            content.getChildren().add(separator);
        }

        TitledPane titledPane = new TitledPane(displayName, content);
        titledPane.setText(displayName);
        return titledPane;
    }

    /**
     * 创建配置字段行
     */
    private HBox createFieldRow(TranslateApi api, ApiConfigField field, Map<String, TextInputControl> fields) {
        HBox row = new HBox(8);
        row.setStyle("-fx-alignment: CENTER_LEFT;");

        Label label = new Label(field.getDisplayName());
        label.setStyle("-fx-min-width: 80;");

        TextInputControl input;
        if (field.getFieldType() == ApiConfigField.FieldType.PASSWORD) {
            input = new PasswordField();
        } else {
            TextField textField = new TextField();
            textField.setPrefWidth(250);
            input = textField;
        }

        if (field.getPlaceholder() != null) {
            input.setPromptText(field.getPlaceholder());
        }

        if (field.getDefaultValue() != null) {
            input.setText(field.getDefaultValue());
        }

        fields.put(field.getFieldName(), input);

        row.getChildren().addAll(label, input);
        return row;
    }

    /**
     * 加载API配置值
     */
    private void loadApiConfigValues(TranslateApi api, Map<String, TextInputControl> fields) {
        String apiName = api.getApiName();
        ApiConfig config = apiConfigDao.findByName(apiName);

        if (config != null) {
            for (ApiConfigField field : api.getConfigFields()) {
                TextInputControl input = fields.get(field.getFieldName());
                if (input == null) continue;

                String value = getConfigValue(config, field.getFieldName(), api);
                if (value != null && !value.isEmpty()) {
                    input.setText(value);
                }
            }
        }
    }

    /**
     * 获取配置值
     */
    private String getConfigValue(ApiConfig config, String fieldName, TranslateApi api) {
        // 先尝试从扩展配置获取
        if (config.getExtraConfig() != null && !config.getExtraConfig().isEmpty()) {
            // 简单JSON解析
            String json = config.getExtraConfig();
            String key = "\"" + fieldName + "\":";
            int idx = json.indexOf(key);
            if (idx >= 0) {
                int start = idx + key.length();
                while (start < json.length() && (json.charAt(start) == ' ' || json.charAt(start) == '"')) {
                    start++;
                }
                int end = start;
                while (end < json.length() && json.charAt(end) != '"' && json.charAt(end) != ',' && json.charAt(end) != '}') {
                    end++;
                }
                return json.substring(start, end).replace("\"", "").trim();
            }
        }

        // 兼容旧配置格式
        switch (fieldName) {
            case "appId":
            case "appKey":
            case "apiKey":
                return config.getApiKey();
            case "secretKey":
            case "appSecret":
            case "apiSecret":
                return config.getApiSecret();
            case "endpoint":
                return config.getEndpoint();
            case "model":
            case "modelName":
                return config.getModelName();
            default:
                return null;
        }
    }

    /**
     * 更新API状态标签
     */
    private void updateApiStatus(String apiName, Label statusLabel) {
        boolean configured = ApiFactory.getInstance().isApiConfigured(apiName);
        if (configured) {
            statusLabel.setText(I18nUtil.get("api.configured"));
            statusLabel.setStyle("-fx-text-fill: green;");
        } else {
            statusLabel.setText(I18nUtil.get("api.not_configured"));
            statusLabel.setStyle("-fx-text-fill: red;");
        }
    }

    /**
     * 保存设置
     */
    @FXML
    private void handleSave() {
        logger.info("保存设置");

        // 保存语言设置
        String language = languageComboBox.getValue();
        if ("中文".equals(language)) {
            appConfig.setLocale(Locale.CHINA);
            I18nUtil.setLocale(Locale.CHINA);
        } else {
            appConfig.setLocale(Locale.US);
            I18nUtil.setLocale(Locale.US);
        }

        appConfig.setAlwaysOnTop(alwaysOnTopCheckBox.isSelected());
        appConfig.setMinimizeToTray(minimizeToTrayCheckBox.isSelected());
        appConfig.setClipboardMonitor(clipboardMonitorCheckBox.isSelected());
        appConfig.setAutoTranslate(autoTranslateCheckBox.isSelected());
        appConfig.setClipboardMinLength(clipboardMinLengthSpinner.getValue());

        // 保存所有API配置
        saveAllApiConfigs();

        // 保存到文件
        appConfig.save();

        // 更新状态标签
        for (Map.Entry<String, Label> entry : apiStatusLabels.entrySet()) {
            updateApiStatus(entry.getKey(), entry.getValue());
        }

        // 更新主窗口置顶状态
        if (mainController != null) {
            mainController.updateAlwaysOnTop();
            mainController.updateAutoTranslate(autoTranslateCheckBox.isSelected());
            mainController.refreshSidePanel();
        }

        // 关闭面板
        if (mainController != null) {
            mainController.hideSidePanel();
        }
    }

    /**
     * 保存所有API配置
     */
    private void saveAllApiConfigs() {
        for (Map.Entry<String, Map<String, TextInputControl>> entry : apiFieldsMap.entrySet()) {
            String apiName = entry.getKey();
            Map<String, TextInputControl> fields = entry.getValue();

            ApiConfig config = new ApiConfig();
            config.setApiName(apiName);
            config.setEnabled(true);

            TranslateApi api = ApiFactory.getInstance().getApi(apiName);
            if (api == null) continue;

            // 收集配置值
            StringBuilder extraConfig = new StringBuilder("{");
            boolean first = true;

            for (ApiConfigField field : api.getConfigFields()) {
                TextInputControl input = fields.get(field.getFieldName());
                if (input == null) continue;

                String value = input.getText();
                if (value != null && !value.isEmpty()) {
                    // 设置到API实例
                    api.setConfigValue(field.getFieldName(), value);

                    // 保存到配置对象
                    switch (field.getFieldName()) {
                        case "appId":
                        case "appKey":
                        case "apiKey":
                            config.setApiKey(value);
                            break;
                        case "secretKey":
                        case "appSecret":
                        case "apiSecret":
                            config.setApiSecret(value);
                            break;
                        case "endpoint":
                            config.setEndpoint(value);
                            break;
                        case "model":
                        case "modelName":
                            config.setModelName(value);
                            break;
                        default:
                            // 其他字段存入extraConfig
                            if (!first) extraConfig.append(",");
                            extraConfig.append("\"").append(field.getFieldName()).append("\":\"").append(value.replace("\"", "\\\"")).append("\"");
                            first = false;
                            break;
                    }
                }
            }

            extraConfig.append("}");
            if (extraConfig.length() > 2) {
                config.setExtraConfig(extraConfig.toString());
            }

            // 保存到数据库
            apiConfigDao.save(config);
        }
    }

    /**
     * 取消
     */
    @FXML
    private void handleCancel() {
        if (mainController != null) {
            mainController.hideSidePanel();
        }
    }

    /**
     * 测试API连接
     */
    private void testApi(TranslateApi api, Map<String, TextInputControl> fields, Label resultLabel) {
        // 先应用配置
        for (ApiConfigField field : api.getConfigFields()) {
            TextInputControl input = fields.get(field.getFieldName());
            if (input != null) {
                api.setConfigValue(field.getFieldName(), input.getText());
            }
        }

        resultLabel.setText("测试中...");
        resultLabel.setStyle("-fx-text-fill: gray;");

        // 异步测试
        new Thread(() -> {
            try {
                Translation result = api.translate("hello", "en", "zh");
                Platform.runLater(() -> {
                    resultLabel.setText("✓ 连接成功: " + result.getTranslatedText());
                    resultLabel.setStyle("-fx-text-fill: green;");
                });
            } catch (Exception e) {
                Platform.runLater(() -> {
                    resultLabel.setText("✗ 连接失败: " + e.getMessage());
                    resultLabel.setStyle("-fx-text-fill: red;");
                });
            }
        }).start();
    }
}
