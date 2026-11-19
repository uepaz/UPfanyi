<p align="right"><a href="README.md">简体中文</a> · English</p>

<p align="center"><img src="src-tauri/icons/128x128.png" width="72" height="72" alt="" /></p>

<h1 align="center">UP Translate</h1>

<p align="center">An AI translator for Windows. Select a passage on a web page or in a document and press a shortcut to read the translation beside your pointer.</p>

<p align="center">
  <a href="#features">Features</a> ·
  <a href="#quick-start">Quick start</a> ·
  <a href="#keyboard-shortcuts">Keyboard shortcuts</a> ·
  <a href="#common-questions">Common questions</a>
</p>

## Features

- **Read alongside the original.** Pin the popup to keep it in view. When the translation finishes, use the copy icon to copy it. To try different wording, expand **View original**, edit the text, and click the translate icon.
- **Translate longer passages.** Type or paste text into the main window and press `Enter` to see the translation appear as the model writes it. Turn on **Auto translate** to start after a pause in typing. The main window and popup share your target language and remember it between sessions.
- **Choose your AI service.** Switch between the built-in channel and your own providers. Each channel remembers its model, API key, and system prompt.
- **Keep useful translations.** Search **History** to revisit a translation or choose **Translate again** to keep working on it. Click the star beside a result in the main window to add it to your **Wordbook**, where you can write notes or export entries as a text file.

## Quick start

### Install on Windows

On 64-bit Windows, run `UPfanyi_2.0.0_x64-setup.exe` and follow the installer. If the releases page only lists 1.x, you can build the installer from the 2.0 source using the steps below. [View published releases](https://github.com/uepaz/UPfanyi/releases)

<details>
<summary>Build the Windows installer from the 2.0 source</summary>

Install Node.js 24, pnpm 11, Rust 1.94 or later with the MSVC toolchain, the Visual Studio Desktop development with C++ workload, the Windows SDK, and WebView2. From the project root, run these commands.

```powershell
pnpm install --frozen-lockfile
pnpm desktop:build
```

The installer is written to `src-tauri/target/release/bundle/nsis/`. Run `pnpm verify` to check the code and run the tests.

</details>

### Your first translation

The interface starts in Simplified Chinese. To switch to English, open **设置 → 偏好**, set **界面语言** to **English**, and click **保存设置**.

1. Open **Settings → AI translation**. **UPfanyi official AI** is selected on first launch. Enter the API key for that channel. [Visit the official website](https://uptoken.top)
2. Click **Get models** and choose an available model, or enter its name manually.
3. You can use **Test connection** to check the configuration. Click **Save settings** to apply it.
4. Select a sentence in a web page or document. Press and release `Ctrl+Shift+C` to see its translation in a popup beside your pointer.

## Keyboard shortcuts

| Default shortcut | Action                                                              |
| ---------------- | ------------------------------------------------------------------- |
| `Ctrl+Shift+C`   | Translate the selected text                                         |
| `Ctrl+Shift+T`   | Show the main window                                                |
| `Enter`          | Translate the text in the main window’s source field                |
| `Shift+Enter`    | Insert a new line in the source field                               |
| `Esc`            | Dismiss the popup. If a language list is open, close the list first |

To change either global shortcut, open **Settings → Desktop & shortcuts**, click its field, press the new key combination, and select **Save settings**.

## Common questions

### How do I use my own AI service?

Under **Settings → AI translation**, open the channel list, choose **Add channel**, and give it a name. For the service address, enter the domain, such as `relay.example.com`. Choose the OpenAI-compatible API your service supports, either `Chat Completions` or `Responses`. Enter a key if required, choose a model, and save. If the service doesn't list its models, enter the model ID from your provider and click **Test connection**.

### Can I translate selected text after closing the main window?

By default, closing the main window sends the app to the system tray. The selection shortcut remains available. To quit the app, choose **Exit** from the tray menu.

### Where do my text and API keys go?

The app sends the source text to your selected AI service for translation. History, wordbook entries, and settings are stored on your computer. Open **Local data directory** at the bottom of **Settings** to see the location. The database stores API keys in plain text, so keep it and any backups private.

## Feedback and license

Ideas for future versions include glossaries, translating text from screenshots, and wordbook review. Share how you would use these features or report a problem on [GitHub Issues](https://github.com/uepaz/UPfanyi/issues).

This project uses the [MIT License](LICENSE).
