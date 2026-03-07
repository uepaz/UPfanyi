# UP 翻译 - Windows 打包指南

## 快速开始

### 方式一：运行 JAR 文件（需要 Java 环境）

1. **打包项目**
   ```bash
   mvn clean package
   ```

2. **运行 JAR**
   ```bash
   java --module-path "%JAVAFX_HOME%/lib" --add-modules javafx.controls,javafx.fxml,javafx.graphics,javafx.base,javafx.swing -jar target/upfanyi-1.0.0.jar
   ```

---

### 方式二：生成 Windows 安装包（推荐 - 无需 Java 环境）

#### 前置要求

1. **JDK 17 或更高版本**
   ```bash
   java -version
   ```
   如果未安装，请从 https://adoptium.net/ 下载

2. **JavaFX SDK**
   - 已放在项目 `temp-javafx` 目录下
   - 或自行下载：https://github.com/openjdk/jfx/releases
   - 下载 `javafx-sdk-21.0.10.zip`

3. **WiX Toolset（用于创建 MSI 安装程序）**
   ```bash
   dotnet tool install --global wix
   ```

#### 打包步骤

1. **准备 JavaFX SDK**
   - 将 `javafx-sdk-21.0.10.zip` 放在 `temp-javafx/` 目录
   - 解压到 `temp-javafx/javafx-sdk-21.0.10/`

2. **安装 WiX Toolset（可选，用于创建 MSI）**
   ```bash
   dotnet tool install --global wix
   ```

3. **运行打包脚本**
   ```bash
   build-exe.bat
   ```

4. **等待打包完成**
   - 脚本会自动：
     - 编译项目
     - 创建应用程序镜像（绿色版）
     - 创建 MSI 安装程序（如果已安装 WiX）

5. **打包完成后输出**
   - **绿色版**: `target\dist\UP-Translation\UP-Translation.exe`
   - **安装程序**: `target\dist\UP-Translation-1.0.0.msi`

---

## 打包流程详解

```
┌─────────────────────────────────────────────────────────────┐
│  步骤 1: 检查 Java 环境                                      │
│  - 验证 JDK 17+ 已安装                                       │
└─────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────┐
│  步骤 2: Maven 编译                                          │
│  - mvn clean package -DskipTests                            │
│  - 生成 target/upfanyi-1.0.0.jar                            │
└─────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────┐
│  步骤 3: 准备分发文件                                        │
│  - 创建 target/distribute/                                  │
│  - 复制 JAR 文件                                             │
└─────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────┐
│  步骤 4: jpackage 打包                                       │
│  - 捆绑 JRE (Java 运行时)                                    │
│  - 捆绑 JavaFX 运行时库                                      │
│  - 生成 UP-Translation.exe                                  │
└─────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────┐
│  步骤 5: WiX 打包 (可选)                                     │
│  - 创建 MSI Windows 安装程序                                 │
│  - 添加开始菜单快捷方式                                      │
│  - 支持卸载                                                  │
└─────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────┐
│  步骤 6: 完成                                                │
│  - 绿色版：target\dist\UP-Translation\                      │
│  - 安装程序：target\dist\UP-Translation-1.0.0.msi           │
└─────────────────────────────────────────────────────────────┘
```

---

## 目录结构

```
UPfanyi/
├── src/                      # 源代码
├── target/                   # 构建输出
│   ├── upfanyi-1.0.0.jar    # FAT JAR（包含所有依赖）
│   ├── distribute/          # 临时分发目录
│   └── dist/                # 最终输出目录
│       ├── UP-Translation/  # 绿色版（包含 JRE+JavaFX）
│       │   ├── UP-Translation.exe
│       │   ├── app/         # 应用程序
│       │   └── runtime/     # 捆绑的 JRE
│       └── UP-Translation-1.0.0.msi  # MSI 安装程序
├── temp-javafx/             # JavaFX SDK（打包用）
│   └── javafx-sdk-21.0.10/
├── build-exe.bat            # 打包脚本
└── pom.xml                  # Maven 配置
```

---

## 分发方式

### 方案 A：分发绿色版（推荐）

1. 复制整个 `target\dist\UP-Translation\` 目录
2. 压缩成 ZIP 文件
3. 发送给用户
4. 用户解压后直接运行 `UP-Translation.exe`

**优点**：
- 用户无需安装
- 不依赖系统 Java 环境
- 便携，可放在 U 盘

### 方案 B：分发安装程序

1. 发送 `target\dist\UP-Translation-1.0.0.msi`
2. 用户运行安装程序
3. 自动创建开始菜单和桌面快捷方式

**优点**：
- 安装规范
- 自动创建快捷方式
- 可通过 Windows 设置卸载

---

## 常见问题

### Q: jpackage 命令找不到
**A**: 确保使用 JDK 17 或更高版本。JDK 8 不包含 jpackage 工具。

### Q: wix 命令找不到
**A**: 运行以下命令安装 WiX Toolset：
```bash
dotnet tool install --global wix
```

### Q: 打包后运行黑屏/闪退
**A**:
1. 检查日志：`%APPDATA%\UP-Translation\logs\`
2. 确认 JavaFX 模块已正确捆绑
3. 尝试直接运行绿色版 exe 文件查看错误信息

### Q: 打包体积太大
**A**: jpackage 会捆绑完整的 JRE 和 JavaFX，这是正常的。运行时大小约 150-200MB。

### Q: 可以在没有 Java 的电脑上运行吗
**A**: 可以！打包的程序已包含完整的 Java 运行时，用户无需安装 Java。

---

## 技术说明

- **打包工具**: jpackage (JDK 17+) + WiX Toolset v6
- **JavaFX 版本**: 21.0.10
- **JVM 要求**: Java 17+
- **输出格式**: app-image (绿色版) + MSI (安装程序)
- **捆绑内容**: JRE + JavaFX + 应用程序

---

## 给开发者的提示

1. **本地测试**：打包前先运行 `mvn javafx:run` 确保应用正常
2. **清理构建**：定期运行 `mvn clean` 清理旧构建
3. **版本管理**：更新 `pom.xml` 中的版本号以区分不同版本
4. **图标设置**：添加 `.ico` 文件后，可在 jpackage 命令中配置 `--icon` 参数

---

## 参考链接

- [FireGiant WiX 快速入门](https://docs.firegiant.com/quick-start/)
- [WiX Toolset 文档](https://wixtoolset.org/docs/)
- [JavaFX 下载](https://github.com/openjdk/jfx/releases)
- [jpackage 文档](https://docs.oracle.com/en/java/javase/17/docs/specs/man/jpackage.html)
