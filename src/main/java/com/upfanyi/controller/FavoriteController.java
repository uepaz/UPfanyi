package com.upfanyi.controller;

import com.upfanyi.model.FavoriteWord;
import com.upfanyi.service.FavoriteService;
import com.upfanyi.util.ClipboardManager;
import com.upfanyi.util.I18nUtil;
import javafx.collections.FXCollections;
import javafx.collections.ObservableList;
import javafx.fxml.FXML;
import javafx.fxml.Initializable;
import javafx.scene.control.*;
import javafx.scene.control.cell.PropertyValueFactory;
import javafx.stage.FileChooser;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

import java.io.File;
import java.io.IOException;
import java.net.URL;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.time.format.DateTimeFormatter;
import java.util.List;
import java.util.ResourceBundle;

/**
 * 收藏/生词本界面控制器
 *
 * @author UP翻译团队
 * @version 1.0.0
 */
public class FavoriteController implements Initializable {

    private static final Logger logger = LoggerFactory.getLogger(FavoriteController.class);

    @FXML private TextField searchField;
    @FXML private Button searchButton;
    @FXML private Button exportButton;

    @FXML private TableView<FavoriteWord> favoriteTableView;
    @FXML private TableColumn<FavoriteWord, String> wordColumn;
    @FXML private TableColumn<FavoriteWord, String> translationColumn;
    @FXML private TableColumn<FavoriteWord, String> timeColumn;

    @FXML private TextArea wordTextArea;
    @FXML private TextArea translationTextArea;
    @FXML private TextArea noteTextArea;
    @FXML private Button copyButton;
    @FXML private Button saveNoteButton;
    @FXML private Button deleteButton;

    private final FavoriteService favoriteService;
    private final ClipboardManager clipboardManager;
    private final ObservableList<FavoriteWord> favoriteList;
    private FavoriteWord selectedWord;

    private static final int PAGE_SIZE = 200;
    private static final DateTimeFormatter TIME_FORMATTER = DateTimeFormatter.ofPattern("yyyy-MM-dd HH:mm");

    public FavoriteController() {
        this.favoriteService = FavoriteService.getInstance();
        this.clipboardManager = ClipboardManager.getInstance();
        this.favoriteList = FXCollections.observableArrayList();
    }

    @Override
    public void initialize(URL location, ResourceBundle resources) {
        logger.info("初始化收藏界面");

        // 初始化表格列
        initTableColumns();

        // 加载收藏
        loadFavorites();

        // 设置表格选择监听
        favoriteTableView.getSelectionModel().selectedItemProperty().addListener(
                (observable, oldValue, newValue) -> showWordDetail(newValue));
    }

    /**
     * 初始化表格列
     */
    private void initTableColumns() {
        wordColumn.setCellValueFactory(new PropertyValueFactory<>("word"));
        translationColumn.setCellValueFactory(new PropertyValueFactory<>("translation"));

        // 时间列格式化
        timeColumn.setCellValueFactory(cellData -> {
            if (cellData.getValue().getCreateTime() != null) {
                return new javafx.beans.property.SimpleStringProperty(
                        cellData.getValue().getCreateTime().format(TIME_FORMATTER));
            }
            return new javafx.beans.property.SimpleStringProperty("");
        });

        // 设置列宽
        wordColumn.prefWidthProperty().bind(favoriteTableView.widthProperty().multiply(0.4));
        translationColumn.prefWidthProperty().bind(favoriteTableView.widthProperty().multiply(0.4));
        timeColumn.prefWidthProperty().bind(favoriteTableView.widthProperty().multiply(0.2));

        // 设置单元格工厂
        wordColumn.setCellFactory(tc -> new TableCell<>() {
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

        translationColumn.setCellFactory(tc -> new TableCell<>() {
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
     * 加载收藏
     */
    private void loadFavorites() {
        List<FavoriteWord> words = favoriteService.getFavoritesPage(PAGE_SIZE, 0);
        favoriteList.clear();
        favoriteList.addAll(words);
        favoriteTableView.setItems(favoriteList);
        logger.info("加载 {} 条收藏", words.size());
    }

    /**
     * 显示单词详情
     */
    private void showWordDetail(FavoriteWord word) {
        this.selectedWord = word;
        if (word != null) {
            wordTextArea.setText(word.getWord());
            translationTextArea.setText(word.getTranslation());
            noteTextArea.setText(word.getNote() != null ? word.getNote() : "");
            // 动态调整TextArea高度
            adjustTextAreaHeight(wordTextArea, word.getWord());
            adjustTextAreaHeight(translationTextArea, word.getTranslation());
        } else {
            wordTextArea.clear();
            translationTextArea.clear();
            noteTextArea.clear();
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
        List<FavoriteWord> words = favoriteService.searchFavorites(keyword, PAGE_SIZE, 0);
        favoriteList.clear();
        favoriteList.addAll(words);
        favoriteTableView.setItems(favoriteList);
    }

    /**
     * 导出到文件
     */
    @FXML
    private void handleExport() {
        FileChooser fileChooser = new FileChooser();
        fileChooser.setTitle("导出收藏");
        fileChooser.setInitialFileName("favorites.txt");
        fileChooser.getExtensionFilters().addAll(
                new FileChooser.ExtensionFilter("文本文件", "*.txt"),
                new FileChooser.ExtensionFilter("所有文件", "*.*")
        );

        File file = fileChooser.showSaveDialog(exportButton.getScene().getWindow());
        if (file != null) {
            try {
                String content = favoriteService.exportFavorites();
                Files.write(file.toPath(), content.getBytes(StandardCharsets.UTF_8));
                logger.info("已导出 {} 条收藏到: {}", favoriteList.size(), file.getAbsolutePath());
            } catch (IOException e) {
                logger.error("导出失败", e);
            }
        }
    }

    /**
     * 复制翻译
     */
    @FXML
    private void handleCopy() {
        if (selectedWord != null) {
            clipboardManager.setClipboardText(selectedWord.getTranslation());
            logger.info("已复制翻译到剪贴板");
        }
    }

    /**
     * 保存备注
     */
    @FXML
    private void handleSaveNote() {
        if (selectedWord != null) {
            String note = noteTextArea.getText().trim();
            favoriteService.addFavorite(selectedWord.getWord(), selectedWord.getTranslation(), note);
            selectedWord.setNote(note);

            // 刷新表格
            favoriteTableView.refresh();
            logger.info("已保存备注");
        }
    }

    /**
     * 删除
     */
    @FXML
    private void handleDelete() {
        if (selectedWord != null) {
            favoriteService.removeFavorite(selectedWord.getId());
            favoriteList.remove(selectedWord);
            showWordDetail(null);
            logger.info("已删除收藏: {}", selectedWord.getWord());
        }
    }

    /**
     * 刷新收藏列表
     */
    public void refresh() {
        loadFavorites();
    }
}
