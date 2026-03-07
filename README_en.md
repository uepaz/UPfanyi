<p align="right">
  <a href="README.md"><strong>中文</strong></a> | English
</p>

---

# UP Translator

<div align="center">

A desktop translation tool built with JavaFX, supporting multiple translation engines for a seamless translation experience.

[![Java](https://img.shields.io/badge/Java-17+-blue.svg)](https://openjdk.java.net/)
[![JavaFX](https://img.shields.io/badge/JavaFX-17+-red.svg)](https://openjfx.io/)
[![Maven](https://img.shields.io/badge/Maven-3.6+-orange.svg)](https://maven.apache.org/)
[![License](https://img.shields.io/badge/License-MIT-green.svg)](LICENSE)
[![CI](https://github.com/yourusername/upfanyi/actions/workflows/maven-build.yml/badge.svg)](https://github.com/yourusername/upfanyi/actions/workflows/maven-build.yml)
[![Contributor Covenant](https://img.shields.io/badge/Contributor%20Covenant-2.1-4baaaa.svg)](CODE_OF_CONDUCT.md)

[Quick Start](#quick-start) • [Features](#features) • [Usage](#usage) • [Contributing](#contributing) • [Roadmap](#roadmap)

</div>

---

## Features

- ✅ **Multiple Translation Engines** - Baidu, Youdao, DeepL, Google, Tencent, Alibaba, Microsoft
- ✅ **Clipboard Monitoring** - Auto-translate on copy
- ✅ **Global Hotkeys** - Custom shortcuts to show window
- ✅ **History** - Automatically save translation records
- ✅ **Favorites** - Save frequently used translations
- ✅ **System Tray** - Minimize to system tray
- ✅ **Always on Top** - Keep window on top
- ✅ **Bilingual UI** - Chinese and English interface support

---

## Tech Stack

- **Language**: Java 17+
- **UI Framework**: JavaFX 17+
- **Build Tool**: Maven
- **Database**: SQLite
- **HTTP Client**: OkHttp
- **JSON Parsing**: Jackson
- **Logging**: SLF4J + Logback

---

## Quick Start

### Requirements

- JDK 17 or higher
- Maven 3.6+

### Build & Run

```bash
# Clone the repository
git clone https://github.com/yourusername/upfanyi.git
cd upfanyi

# Compile the project
mvn clean compile

# Run the application
mvn javafx:run
```

### Package & Distribute

```bash
# Package as executable JAR
mvn clean package

# Run the JAR
java -jar target/upfanyi-1.0.0.jar
```

---

## Usage

### Configure Translation APIs

Before using the translation features, you need to configure your API credentials.

<details>
<summary><strong>Baidu Translate API</strong></summary>

1. Visit [Baidu Translate Open Platform](https://fanyi-api.baidu.com/)
2. Register and create an application
3. Get App ID and Secret Key
4. Enter credentials in the app settings

</details>

<details>
<summary><strong>Youdao Translate API</strong></summary>

1. Visit [Youdao AI Cloud](https://ai.youdao.com/)
2. Register and create an application
3. Get App Key and App Secret
4. Enter credentials in the app settings

</details>

<details>
<summary><strong>DeepL API</strong></summary>

1. Visit [DeepL API Pro](https://www.deepl.com/pro-api)
2. Register and get API Key
3. Enter API Key in the app settings

</details>

<details>
<summary><strong>Tencent Translate API</strong></summary>

1. Visit [Tencent Interactive Translation](https://fanyi.qq.com/)
2. Register and get SecretId and SecretKey
3. Enter credentials in the app settings

</details>

<details>
<summary><strong>Alibaba Cloud Translate API</strong></summary>

1. Visit [Alibaba Cloud Machine Translation](https://www.aliyun.com/product/mt)
2. Register and get AccessKeyId and AccessKeySecret
3. Enter credentials in the app settings

</details>

### Keyboard Shortcuts

| Shortcut | Function |
|----------|----------|
| `Ctrl+Shift+T` | Show/Hide translation window |
| `Ctrl+Shift+C` | Translate clipboard content |
| `Enter` | Translate input text |

---

## Project Structure

```
UPfanyi/
├── src/main/java/com/upfanyi/
│   ├── App.java                 # Application entry point
│   ├── Launcher.java            # JAR launcher
│   ├── api/                     # Translation API implementations
│   ├── config/                  # Configuration management
│   ├── controller/              # FXML controllers
│   ├── dao/                     # Data access layer
│   ├── model/                   # Data models
│   ├── service/                 # Business logic services
│   ├── util/                    # Utility classes
│   └── view/                    # View management
├── src/main/resources/
│   ├── css/                     # Stylesheets
│   ├── db/                      # Database schemas
│   ├── fxml/                    # FXML layouts
│   ├── i18n/                    # Internationalization bundles
│   └── images/                  # Image assets
└── pom.xml                      # Maven configuration
```

---

## Roadmap

- [ ] Support more translation engines
- [ ] Image translation with OCR
- [ ] PDF document translation
- [ ] Cloud synchronization
- [ ] Browser extension

---

## Contributing

Issues and Pull Requests are welcome!

### Guidelines

1. Fork this repository
2. Create a feature branch (`git checkout -b feature/AmazingFeature`)
3. Commit your changes (`git commit -m 'Add some AmazingFeature'`)
4. Push to the branch (`git push origin feature/AmazingFeature`)
5. Open a Pull Request

---

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

---

## Contact

- **Project Homepage**: https://github.com/uepaz/upfanyi
- **Issues**: https://github.com/uepaz/upfanyi/issues
- **Email**: 2182557729@qq.com

---

<div align="center">

Made with ❤️ by uepaz

⭐ If this project helps you, please give a Star!

</div>
