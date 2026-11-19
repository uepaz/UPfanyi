<p align="right">
  <a href="README_en.md"><strong>English</strong></a> | 中文
</p>

---

# UP 翻译

<div align="center">

一款基于 JavaFX 开发的桌面翻译工具，支持多种翻译引擎，提供便捷的翻译体验。

[![Java](https://img.shields.io/badge/Java-17+-blue.svg)](https://openjdk.java.net/)
[![JavaFX](https://img.shields.io/badge/JavaFX-17+-red.svg)](https://openjfx.io/)
[![Maven](https://img.shields.io/badge/Maven-3.6+-orange.svg)](https://maven.apache.org/)
[![License](https://img.shields.io/badge/License-MIT-green.svg)](LICENSE)
[![CI](https://github.com/yourusername/upfanyi/actions/workflows/maven-build.yml/badge.svg)](https://github.com/yourusername/upfanyi/actions/workflows/maven-build.yml)
[![Contributor Covenant](https://img.shields.io/badge/Contributor%20Covenant-2.1-4baaaa.svg)](CODE_OF_CONDUCT.md)

[快速开始](#快速开始) • [功能特性](#功能特性) • [使用方法](#使用方法) • [参与贡献](#参与贡献) • [开发计划](#开发计划)

</div>

---

## 功能特性

- ✅ **多翻译引擎支持** - 百度、有道、DeepL、Google、腾讯、阿里、微软等
- ✅ **剪贴板监听** - 复制文本后自动翻译
- ✅ **全局快捷键** - 自定义快捷键唤起窗口
- ✅ **历史记录** - 自动保存翻译历史
- ✅ **收藏/生词本** - 收藏常用翻译内容
- ✅ **系统托盘** - 最小化到系统托盘
- ✅ **窗口置顶** - 窗口可置顶显示
- ✅ **中英双语** - 支持中英文界面切换

---

## 技术栈

- **语言**: Java 17+
- **UI 框架**: JavaFX 17+
- **构建工具**: Maven
- **数据库**: SQLite
- **HTTP 客户端**: OkHttp
- **JSON 解析**: Jackson
- **日志**: SLF4J + Logback

---

## 快速开始

### 环境要求

- JDK 17 或更高版本
- Maven 3.6+

### 编译运行

```bash
# 克隆项目
git clone https://github.com/yourusername/upfanyi.git
cd upfanyi

# 编译项目
mvn clean compile

# 运行项目
mvn javafx:run
```

### 打包发布

```bash
# 打包为可执行 JAR
mvn clean package

# 运行 JAR
java -jar target/upfanyi-1.0.0.jar
```

---

## 使用方法

### 配置翻译 API

在使用翻译功能之前，需要配置翻译 API 的密钥。

<details>
<summary><strong>百度翻译 API</strong></summary>

1. 访问 [百度翻译开放平台](https://fanyi-api.baidu.com/)
2. 注册并创建应用
3. 获取 App ID 和 Secret Key
4. 在应用设置中填入密钥

</details>

<details>
<summary><strong>有道翻译 API</strong></summary>

1. 访问 [有道智云](https://ai.youdao.com/)
2. 注册并创建应用
3. 获取 App Key 和 App Secret
4. 在应用设置中填入密钥

</details>

<details>
<summary><strong>DeepL API</strong></summary>

1. 访问 [DeepL API Pro](https://www.deepl.com/pro-api)
2. 注册并获取 API Key
3. 在应用设置中填入密钥

</details>

<details>
<summary><strong>腾讯翻译 API</strong></summary>

1. 访问 [腾讯交互翻译](https://fanyi.qq.com/)
2. 注册并获取 SecretId 和 SecretKey
3. 在应用设置中填入密钥

</details>

<details>
<summary><strong>阿里云翻译 API</strong></summary>

1. 访问 [阿里云机器翻译](https://www.aliyun.com/product/mt)
2. 注册并获取 AccessKeyId 和 AccessKeySecret
3. 在应用设置中填入密钥

</details>

### 快捷键

| 快捷键 | 功能 |
|--------|------|
| `Ctrl+Shift+T` | 显示/隐藏翻译窗口 |
| `Ctrl+Shift+C` | 翻译剪贴板内容 |
| `Enter` | 翻译输入的文本 |

---

## 项目结构

```
UPfanyi/
├── src/main/java/com/upfanyi/
│   ├── App.java                 # 应用入口
│   ├── Launcher.java            # JAR 启动器
│   ├── api/                     # 翻译 API 实现
│   ├── config/                  # 配置管理
│   ├── controller/              # 控制器
│   ├── dao/                     # 数据访问层
│   ├── model/                   # 数据模型
│   ├── service/                 # 业务服务层
│   ├── util/                    # 工具类
│   └── view/                    # 视图管理
├── src/main/resources/
│   ├── css/                     # 样式文件
│   ├── db/                      # 数据库脚本
│   ├── fxml/                    # FXML 布局
│   ├── i18n/                    # 国际化资源
│   └── images/                  # 图片资源
└── pom.xml                      # Maven 配置
```

---

## 开发计划

- [ ] 支持更多翻译引擎
- [ ] 图片翻译（OCR）
- [ ] PDF 文档翻译
- [ ] 云端同步功能
- [ ] 浏览器插件版本

---

## 贡献

欢迎提交 Issue 和 Pull Request！

### 提交指南

1. Fork 本仓库
2. 创建特性分支 (`git checkout -b feature/AmazingFeature`)
3. 提交更改 (`git commit -m 'Add some AmazingFeature'`)
4. 推送到分支 (`git push origin feature/AmazingFeature`)
5. 开启 Pull Request

---

## 许可证

本项目采用 MIT 许可证。详见 [LICENSE](LICENSE) 文件。

---

## 联系方式

- **项目主页**: https://github.com/uepaz/upfanyi
- **问题反馈**: https://github.com/uepaz/upfanyi/issues
- **邮箱**: 2182557729@qq.com

---

## 致谢

感谢以下开源项目：
- [JavaFX](https://openjfx.io/) - UI 框架
- [OkHttp](https://square.github.io/okhttp/) - HTTP 客户端
- [Jackson](https://github.com/FasterXML/jackson) - JSON 解析
- [SQLite](https://www.sqlite.org/) - 嵌入式数据库
- [Logback](https://logback.qos.ch/) - 日志框架
- [JNativeHook](https://github.com/kwhat/jnativehook) - 全局快捷键

---

## 赞助

如果这个项目对你有帮助，欢迎通过以下方式支持：

- ⭐ 给项目一个 Star
- 🔗 分享给需要的朋友
- 🐛 报告 Bug 或提交 PR

---

## 常见问题

### Q: 为什么翻译不工作？

A: 请确保你已经正确配置了翻译 API 的密钥。不同的 API 有不同的使用限制，建议阅读对应平台的文档。

### Q: 如何更改界面语言？

A: 在设置页面可以切换界面语言。如果没有生效，请重启应用。

### Q: 支持离线翻译吗？

A: 当前版本不支持离线翻译。所有翻译都需要网络连接来调用翻译 API。

---

<div align="center">

Made with ❤️ by uepaz

⭐ 如果这个项目对你有帮助，请给一个 Star！

</div>
