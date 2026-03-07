package com.upfanyi;

/**
 * JAR 启动器
 * 用于解决 JavaFX 模块加载和打包问题
 * 
 * @author UP 翻译团队
 * @version 1.0.0
 */
public class Launcher {
    public static void main(String[] args) {
        // 委托给实际的 Application 类
        App.main(args);
    }
}
