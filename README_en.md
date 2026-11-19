# UPfanyi

[中文](README.md)

A Windows AI translation app built with Tauri 2, Vue 3, TypeScript and Rust. Includes LLM translation, history, vocabulary/favorites, configurable global shortcuts, selected-text translation, a system tray, and Chinese/English UI.

## Development

Use Node.js 24, pnpm 11, Rust 1.94+ with the MSVC toolchain, Visual Studio desktop C++ tools and Windows SDK, and WebView2. Node and pnpm versions are recorded in `.node-version` and `package.json`; CI uses Rust 1.94.0. Both dependency lockfiles are checked in.

```powershell
pnpm install --frozen-lockfile
pnpm desktop:dev
```

Build the Windows executable and NSIS installer:

```powershell
pnpm desktop:build
```

Outputs: `src-tauri/target/release/upfanyi.exe` and `src-tauri/target/release/bundle/nsis/`. `pnpm dev` only previews the browser UI; data, translation, and system actions require the desktop runtime.

## AI configuration and data

A fresh first launch has only one built-in channel, “UPfanyi official AI” at https://uptoken.top, using Chat Completions with an empty key and model. The official channel is fixed and shows a Visit official website action on its right. For any other service, choose “Add channel” and enter a unique name. Added channels can be renamed or deleted and display an editable service address. Each channel retains its address, protocol/API, key, model and prompt. Switching preserves drafts; Save settings persists all channels and the active selection together, while Cancel restores all saved values. Channel, protocol, API and language selectors share styled keyboard-accessible popups; editable model choices support filtering and manual entry.

The settings keep separate protocol and API selectors. OpenAI-compatible is the only protocol, and the API can be Chat Completions or Responses. Enter only the service domain, such as `relay.example.com` or `https://relay.example.com`; bare domains use HTTPS. To use local Ollama, manually add a channel named `ollama`, choose Chat Completions, and enter `http://localhost:11434`. A default local server can use a blank key. Enter any required API key, then use the model field below the key to fetch, search, select, or manually enter a model. Official OpenAI endpoints require a key; custom and local services can omit it. An optional system prompt controls translation instructions.

Model discovery runs only when clicked and uses the unsaved draft, as does the separate connection test. Neither writes settings or history. Empty lists and failures preserve the current model; a listed model is not a guarantee of API support. Paths are generated automatically: OpenAI Chat uses `/v1/chat/completions`, Responses uses `/v1/responses`, and discovery uses `/v1/models`. Base URLs ending in `/v1` are accepted without duplication, preserving proxy prefixes, ports and queries. Recognized full API URLs also remain usable; for special full paths with no known model-list route, enter the model manually. Responses requests are non-streaming with `store: false`; refusals and incomplete responses do not become successful translations. Results and history show the model used. Real credentials, quotas and model availability require service-specific testing.

The named-channel edition uses a new native format, usually `%APPDATA%\com.upfanyi.desktop\data-v3\upfanyi.sqlite3` on Windows. The UI displays the actual directory. A new data-v3 starts with empty history and favorites, default preferences and only the official channel. Subsequent launches retain data-v3 records. The app does not discover, read, migrate or delete data-v2 or earlier files. Preferences and the complete channel collection share SQLite. Credentials are stored locally in plaintext; do not share the database.

For isolated development, set `UPFANYI_DATA_DIR` to a dedicated root; the app creates `data-v3/upfanyi.sqlite3` underneath it. Existing native databases must pass identity, version and schema checks; unsupported formats produce an error without automatic replacement. The previous version is preserved on the `legacy/java-v1` branch; inspect it in a separate directory to protect the current working tree.

Default global shortcuts are `Ctrl+Shift+T` to show the window and `Ctrl+Shift+C` to translate selected text. Select text in another window and press and release the shortcut. The app copies the selection before opening the translation window, and does nothing if no new text is copied. Ordinary copying or cutting does not start a translation; `Ctrl+C` is reserved for copying. In Desktop and shortcuts, click a shortcut field and press the new combination. Esc cancels recording and Tab leaves the field. Save settings applies the draft and checks conflicts.

## Checks and documentation

```powershell
pnpm verify
```

`verify` checks lint, formatting, types, frontend production output, frontend/Rust tests, Rust fmt and Clippy. Tests use mock IPC, temporary databases, and local HTTP servers, without real translation credentials. Windows CI runs these checks and builds the installer. Individual commands are defined in `package.json`.

General preferences contain interface language and always-on-top controls; translation language and automatic translation remain on the translation page. The UI offers first-use AI configuration, grouped settings with drafts preserved across navigation, and per-record vocabulary note drafts. Suggested follow-up work includes configuration guidance, secret storage and backup/restore, terminology, streaming output, OCR, and vocabulary review.

License: [MIT](LICENSE).
