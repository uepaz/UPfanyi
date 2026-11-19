# UP翻译

[English](README_en.md)

Windows AI 翻译工具，使用 Tauri 2、Vue 3、TypeScript 与 Rust。支持大模型翻译、历史记录、收藏/生词本、全局快捷键、选中文字快捷翻译、托盘和中英文界面。

## 运行与构建

开发环境：Node.js 24、pnpm 11、Rust 1.94+ MSVC 工具链、Visual Studio C++ 桌面开发工具及 Windows SDK、WebView2。Node 与 pnpm 版本见 `.node-version`、`package.json`；依赖版本由两个锁文件固定，CI 固定 Rust 1.94.0。

```powershell
pnpm install --frozen-lockfile
pnpm desktop:dev
```

Windows 打包：

```powershell
pnpm desktop:build
```

可执行程序位于 `src-tauri/target/release/upfanyi.exe`，NSIS 安装包位于 `src-tauri/target/release/bundle/nsis/`。

`pnpm dev` 仅启动浏览器预览；翻译、数据库和系统操作需要桌面运行环境，浏览器中会显示提示。

## 使用

首次启动仅预置并默认使用“UP翻译官方 AI”（https://uptoken.top），接口为 Chat Completions，密钥和模型需填写。官方渠道固定保留，右侧显示“访问官方网站”入口。非官方服务一律从“添加渠道”创建，填写不重名的名称；新增渠道可重命名、删除，右侧填写服务地址。每个渠道分别保存地址、协议/接口、密钥、模型和提示词。切换渠道保留本轮草稿，点击“保存设置”统一保存全部渠道和当前选择；“取消”恢复全部已保存内容。渠道、协议、接口及语言选择框采用统一弹层，支持键盘选择；模型使用同样式的可编辑候选弹层，支持筛选和手填。

第一次使用时，打开 AI 配置，在顶部选择协议类型与接口类型：协议类型仅提供 OpenAI 兼容，右侧接口类型可选 Chat Completions 或 Responses。服务地址只需填写中转域名，例如 `relay.example.com` 或 `https://relay.example.com`，裸域名默认 HTTPS；连接本机 Ollama 时，可手动添加名为 `ollama` 的普通渠道，选择 Chat Completions，填写 `http://localhost:11434`；默认本机服务可留空密钥。填写所需密钥后，在密钥下方点击“获取模型”，搜索选择或直接手填模型名称。支持自定义系统提示词。偏好页提供界面语言与窗口置顶，语言和自动翻译在翻译页操作。设置分为偏好、AI 翻译和桌面快捷键，切换页面保留草稿，取消才丢弃修改。Enter 翻译，Shift+Enter 换行；自动翻译采用输入防抖，只有最新请求更新当前结果。

历史支持搜索、分页、重用和删除；收藏支持备注、搜索和 UTF-8 文本导出。默认快捷键为 `Ctrl+Shift+T` 显示窗口、`Ctrl+Shift+C` 翻译选中文字。在其它窗口选中文字后按下并松开快捷键，应用先复制选区，再弹出窗口翻译。没有复制到新文本时不翻译。普通复制和剪切不会触发翻译，`Ctrl+C` 保留给复制。在“桌面与快捷键”中点击对应输入框，再按下组合键即可修改草稿；Esc 取消录制，Tab 离开，点击“保存设置”后生效并检查冲突。启用“关闭到托盘”后，点击窗口关闭按钮隐藏窗口，通过托盘恢复或退出。

用户渠道按服务要求填写密钥；官方 OpenAI 地址必须提供密钥。模型列表和连接测试使用当前草稿，不保存配置或历史。模型列表只在点击时获取，空列表或失败不会清空已填模型；列表不保证模型支持所选接口，可另行测试连接。结果与历史显示实际模型；真实认证、额度和模型可用性需用对应账户验证。

程序根据所选接口自动补路径：OpenAI Chat 使用 `/v1/chat/completions`，Responses 使用 `/v1/responses`，模型列表使用 `/v1/models`。可填带 `/v1` 的基础地址，支持代理前缀、端口和查询参数，不重复拼接版本路径。已填可识别的完整接口地址仍可使用；特殊完整路径无法推导模型列表时继续手填。Responses 使用非流式文本请求，显式发送 `store: false`，拒绝或未完成响应不会保存为成功翻译。

## 本地数据

当前命名渠道版本使用独立原生格式，Windows 默认路径通常为 `%APPDATA%\com.upfanyi.desktop\data-v3\upfanyi.sqlite3`，实际目录在设置页显示。首次建立 data-v3 时设置、渠道、历史和收藏从空白开始，仅预置官方渠道；后续启动保留 data-v3 的记录。应用不探测、读取、迁移或删除 data-v2 及更早版本文件。偏好和完整渠道集合统一保存在 SQLite。

测试时使用专用目录，避免读取真实数据：

```powershell
$env:UPFANYI_DATA_DIR = Join-Path $PWD '.task-artifacts\dev-data'
pnpm desktop:dev
```

应用在指定根目录下创建 `data-v3/upfanyi.sqlite3`。凭据以明文保存在本地 SQLite，勿提交或分享数据库。已有原生数据库必须通过标识、版本、结构和配置检查，格式不匹配时会报错，应用不会自动覆盖。旧版本源码保存在 `legacy/java-v1` 分支，需要查阅时使用单独目录，避免覆盖当前工作区。

## 验证

```powershell
pnpm verify
```

`verify` 包含格式、lint、类型、前端与 Rust 测试、前端生产构建，以及 Rust fmt 和 Clippy。前端测试使用 mock IPC；Rust 使用临时 SQLite 和本地 HTTP 服务。离线测试不调用真实翻译服务。分项命令见 `package.json`。

后续建议优先完善 AI 配置体验、密钥保护与备份恢复，再考虑术语库、流式翻译、截图 OCR 和收藏复习。

许可证：[MIT](LICENSE)。
