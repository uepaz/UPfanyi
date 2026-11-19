package com.upfanyi.util;

import com.github.kwhat.jnativehook.GlobalScreen;
import com.github.kwhat.jnativehook.NativeHookException;
import com.github.kwhat.jnativehook.keyboard.NativeKeyEvent;
import com.github.kwhat.jnativehook.keyboard.NativeKeyListener;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

import java.util.HashSet;
import java.util.List;
import java.util.Map;
import java.util.Set;
import java.util.concurrent.ConcurrentHashMap;

/**
 * 全局快捷键管理器
 * 使用JNativeHook监听全局键盘事件
 *
 * @author UP翻译团队
 * @version 1.0.0
 */
public class HotkeyManager implements NativeKeyListener {

    private static final Logger logger = LoggerFactory.getLogger(HotkeyManager.class);

    private static HotkeyManager instance;

    private final Map<String, Runnable> hotkeyCallbacks;
    private final Set<Integer> pressedKeys;
    private boolean isInitialized = false;

    private HotkeyManager() {
        this.hotkeyCallbacks = new ConcurrentHashMap<>();
        this.pressedKeys = new HashSet<>();
    }

    public static synchronized HotkeyManager getInstance() {
        if (instance == null) {
            instance = new HotkeyManager();
        }
        return instance;
    }

    /**
     * 初始化快捷键管理器
     */
    public void init() {
        if (isInitialized) {
            logger.warn("快捷键管理器已初始化");
            return;
        }

        try {
            // 注册原生钩子
            GlobalScreen.registerNativeHook();
            GlobalScreen.addNativeKeyListener(this);
            isInitialized = true;
            logger.info("快捷键管理器初始化成功");
        } catch (NativeHookException e) {
            logger.error("快捷键管理器初始化失败", e);
        }
    }

    /**
     * 注册快捷键
     *
     * @param keyCombo 快捷键组合，如 "ctrl shift T"
     * @param callback 回调函数
     */
    public void registerHotkey(String keyCombo, Runnable callback) {
        String normalizedKey = normalizeKeyCombo(keyCombo);
        hotkeyCallbacks.put(normalizedKey, callback);
        logger.info("注册快捷键: {}", normalizedKey);
    }

    /**
     * 注销快捷键
     */
    public void unregisterHotkey(String keyCombo) {
        String normalizedKey = normalizeKeyCombo(keyCombo);
        hotkeyCallbacks.remove(normalizedKey);
        logger.info("注销快捷键: {}", normalizedKey);
    }

    /**
     * 规范化快捷键组合
     */
    private String normalizeKeyCombo(String keyCombo) {
        // 将快捷键字符串规范化为小写，并按字母顺序排序
        String[] parts = keyCombo.toLowerCase().split("[\\s+]+");
        Set<String> keySet = new HashSet<>();
        for (String part : parts) {
            keySet.add(part.trim());
        }
        return String.join("+", keySet.stream().sorted().toList());
    }

    /**
     * 将按键代码转换为快捷键字符串
     */
    private String buildKeyCombo() {
        Set<String> keys = new HashSet<>();

        for (int keyCode : pressedKeys) {
            String keyName = keyCodeToString(keyCode);
            if (keyName != null) {
                keys.add(keyName);
            }
        }

        if (keys.isEmpty()) {
            return null;
        }

        return String.join("+", keys.stream().sorted().toList());
    }

    /**
     * 按键代码转字符串
     */
    private String keyCodeToString(int keyCode) {
        // 修饰键 - 使用统一的常量
        if (keyCode == NativeKeyEvent.VC_CONTROL) {
            return "ctrl";
        }
        if (keyCode == NativeKeyEvent.VC_SHIFT) {
            return "shift";
        }
        if (keyCode == NativeKeyEvent.VC_ALT) {
            return "alt";
        }
        if (keyCode == NativeKeyEvent.VC_META) {
            return "meta";
        }

        // 字母键
        if (keyCode >= NativeKeyEvent.VC_A && keyCode <= NativeKeyEvent.VC_Z) {
            return String.valueOf((char) ('a' + (keyCode - NativeKeyEvent.VC_A)));
        }

        // 数字键
        if (keyCode >= NativeKeyEvent.VC_0 && keyCode <= NativeKeyEvent.VC_9) {
            return String.valueOf((char) ('0' + (keyCode - NativeKeyEvent.VC_0)));
        }

        // 功能键
        if (keyCode >= NativeKeyEvent.VC_F1 && keyCode <= NativeKeyEvent.VC_F12) {
            return "f" + (keyCode - NativeKeyEvent.VC_F1 + 1);
        }

        return null;
    }

    @Override
    public void nativeKeyPressed(NativeKeyEvent nativeKeyEvent) {
        int keyCode = nativeKeyEvent.getKeyCode();
        pressedKeys.add(keyCode);

        String keyCombo = buildKeyCombo();
        if (keyCombo != null && hotkeyCallbacks.containsKey(keyCombo)) {
            logger.debug("触发快捷键: {}", keyCombo);
            Runnable callback = hotkeyCallbacks.get(keyCombo);
            if (callback != null) {
                // 在新线程中执行回调，避免阻塞事件处理
                new Thread(callback).start();
            }
        }
    }

    @Override
    public void nativeKeyReleased(NativeKeyEvent nativeKeyEvent) {
        pressedKeys.remove(nativeKeyEvent.getKeyCode());
    }

    @Override
    public void nativeKeyTyped(NativeKeyEvent nativeKeyEvent) {
        // 不需要处理
    }

    /**
     * 关闭快捷键管理器
     */
    public void shutdown() {
        try {
            GlobalScreen.removeNativeKeyListener(this);
            GlobalScreen.unregisterNativeHook();
            isInitialized = false;
            logger.info("快捷键管理器已关闭");
        } catch (NativeHookException e) {
            logger.error("关闭快捷键管理器失败", e);
        }
    }

    /**
     * 获取已注册的快捷键列表
     */
    public Set<String> getRegisteredHotkeys() {
        return new HashSet<>(hotkeyCallbacks.keySet());
    }
}
