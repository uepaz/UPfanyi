package com.upfanyi.controller;

import com.upfanyi.model.HistoryRecord;
import com.upfanyi.service.HistoryService;
import com.upfanyi.util.ClipboardManager;
import com.upfanyi.util.I18nUtil;
import javafx.collections.FXCollections;
import javafx.collections.ObservableList;
import javafx.fxml.FXML;
import javafx.fxml.Initializable;
import javafx.scene.control.*;
import javafx.scene.control.cell.PropertyValueFactory;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

import java.net.URL;
import java.time.format.DateTimeFormatter;
import java.util.List;
import java.util.ResourceBundle;

/**
 * 历史记录界面控制器
 *
 * @author UP翻译团队
 * @version 1.0.0
 */
public class HistoryController implements Initializable {

    private static final Logger logger = LoggerFactory.getLogger(HistoryController.class);

    @FXML private TextField searchField;
    @FXML private Button searchButton;
    @FXML private Button clearAllButton;

    @FXML private TableView<HistoryRecord> historyTableView;
    @FXML private TableColumn<HistoryRecord, String> sourceColumn;
    @FXML private TableColumn<HistoryRecord, String> resultColumn;
    @FXML private TableColumn<HistoryRecord, String> apiColumn;
    @FXML private TableColumn<HistoryRecord, String> timeColumn;

    @FXML private TextArea sourceTextArea;
    @FXML private TextArea resultTextArea;
    @FXML private Button copyButton;
    @FXML private Button deleteButton;

    private final HistoryService historyService;
    private final ClipboardManager clipboardManager;
    private final ObservableList<HistoryRecord> historyList;
    private HistoryRecord selectedRecord;

    private static final int PAGE_SIZE = 200;
    private static final DateTimeFormatter TIME_FORMATTER = DateTimeFormatter.ofPattern("yyyy-MM-dd HH:mm");

    public HistoryController() {
        this.historyService = HistoryService.getInstance();
        this.clipboardManager = ClipboardManager.getInstance();
        this.historyList = FXCollections.observableArrayList();
    }

    @Override
    public void initialize(URL location, ResourceBundle resources) {
        logger.info("初始化历史记录界面");

        // 初始化表格列
        initTableColumns();

        // 加载历史记录
        loadHistory();

        // 设置表格选择监听
        historyTableView.getSelectionModel().selectedItemProperty().addListener(
                (observable, oldValue, newValue) -> showRecordDetail(newValue));
    }

    /**
     * 初始化表格列
     */
    private void initTableColumns() {
        sourceColumn.setCellValueFactory(new PropertyValueFactory<>("sourceText"));
        resultColumn.setCellValueFactory(new PropertyValueFactory<>("translatedText"));
        apiColumn.setCellValueFactory(new PropertyValueFactory<>("apiUsed"));

        // 时间列格式化
        timeColumn.setCellValueFactory(cellData -> {
            if (cellData.getValue().getCreateTime() != null) {
                return new javafx.beans.property.SimpleStringProperty(
                        cellData.getValue().getCreateTime().format(TIME_FORMATTER));
            }
            return new javafx.beans.property.SimpleStringProperty("");
        });

        // 设置列宽
        sourceColumn.prefWidthProperty().bind(historyTableView.widthProperty().multiply(0.35));
        resultColumn.prefWidthProperty().bind(historyTableView.widthProperty().multiply(0.35));
        apiColumn.prefWidthProperty().bind(historyTableView.widthProperty().multiply(0.15));
        timeColumn.prefWidthProperty().bind(historyTableView.widthProperty().multiply(0.15));

        // 设置单元格工厂，处理长文本
        sourceColumn.setCellFactory(tc -> new TableCell<>() {
            @Override
            protected void updateItem(String item, boolean empty) {
                super.updateItem(item, empty);
                if (empty || item == null) {
                    setText(null);
                    setTooltip(null);
                } else {
                    setText(item.length() > 30 ? item.substring(0, 30) + "..." : item);
                    setTooltip(new Tooltip(item));
                }
            }
        });

        resultColumn.setCellFactory(tc -> new TableCell<>() {
            @Override
            protected void updateItem(String item, boolean empty) {
                super.updateItem(item, empty);
                if (empty || item == null) {
                    setText(null);
                    setTooltip(null);
                } else {
                    setText(item.length() > 30 ? item.substring(0, 30) + "..." : item);
                    setTooltip(new Tooltip(item));
                }
            }
        });
    }

    /**
     * 加载历史记录
     */
    private void loadHistory() {
        List<HistoryRecord> records = historyService.getHistoryPage(PAGE_SIZE, 0);
        historyList.clear();
        historyList.addAll(records);
        historyTableView.setItems(historyList);
        logger.info("加载 {} 条历史记录", records.size());
    }

    /**
     * 显示记录详情
     */
    private void showRecordDetail(HistoryRecord record) {
        this.selectedRecord = record;
        if (record != null) {
            sourceTextArea.setText(record.getSourceText());
            resultTextArea.setText(record.getTranslatedText());
            // 动态调整TextArea高度
            adjustTextAreaHeight(sourceTextArea, record.getSourceText());
            adjustTextAreaHeight(resultTextArea, record.getTranslatedText());
        } else {
            sourceTextArea.clear();
            resultTextArea.clear();
        }
    }

    /**
     * 动态调整TextArea高度（3-8行）
     */
    private void adjustTextAreaHeight(TextArea textArea, String text) {
        if (text == null || text.isEmpty()) {
            textArea.setPrefRowCount(3);
            return;
        }
        // 计算行数：每40个字符约一行（考虑中英文混合）
        int lineCount = Math.max(3, Math.min(8, (text.length() / 40) + 1));
        textArea.setPrefRowCount(lineCount);
    }

    /**
     * 搜索
     */
    @FXML
    private void handleSearch() {
        String keyword = searchField.getText().trim();
        List<HistoryRecord> records = historyService.searchHistory(keyword, PAGE_SIZE, 0);
        historyList.clear();
        historyList.addAll(records);
        historyTableView.setItems(historyList);
    }

    /**
     * 清空历史
     */
    @FXML
    private void handleClearAll() {
        historyService.clearAllHistory();
        historyList.clear();
        historyTableView.setItems(historyList);
        showRecordDetail(null);
        logger.info("已清空所有历史记录");
    }

    /**
     * 复制结果
     */
    @FXML
    private void handleCopy() {
        if (selectedRecord != null) {
            clipboardManager.setClipboardText(selectedRecord.getTranslatedText());
            logger.info("已复制翻译结果到剪贴板");
        }
    }

    /**
     * 删除记录
     */
    @FXML
    private void handleDelete() {
        if (selectedRecord != null) {
            historyService.deleteHistory(selectedRecord.getId());
            historyList.remove(selectedRecord);
            showRecordDetail(null);
            logger.info("已删除历史记录: {}", selectedRecord.getId());
        }
    }

    /**
     * 刷新历史记录
     */
    public void refresh() {
        loadHistory();
    }
}
