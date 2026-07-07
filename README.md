# PetDrawer

<p align="center">
  <strong>桌面宠物、快捷入口、AI 伴侣、翻译和音乐播放器的一体化 Windows 桌面工具。</strong>
</p>

<p align="center">
  <a href="#中文">中文</a> · <a href="#english">English</a>
</p>

<p align="center">
  <img alt="Tauri" src="https://img.shields.io/badge/Tauri-v2-24C8DB?logo=tauri&logoColor=white">
  <img alt="Vue" src="https://img.shields.io/badge/Vue-3-42b883?logo=vue.js&logoColor=white">
  <img alt="TypeScript" src="https://img.shields.io/badge/TypeScript-5-3178c6?logo=typescript&logoColor=white">
  <img alt="Rust" src="https://img.shields.io/badge/Rust-2021-b7410e?logo=rust&logoColor=white">
  <img alt="Platform" src="https://img.shields.io/badge/platform-Windows-blue">
</p>

---

## 中文

PetDrawer 是一个基于 **Tauri v2 + Vue 3 + TypeScript + Rust** 的桌面宠物工具。它把桌宠、快捷入口抽屉、AI 伴侣、本机记忆、划选翻译、音乐播放器、在线音乐和 Codex 状态提醒整合到一个轻量桌面应用里。

它的核心目标不是做一个单纯的启动器，而是做一个可以常驻桌面的个人工作与陪伴入口：点一下打开工具，右键呼出菜单，聊天时保留本机记忆，听歌时进入沉浸舞台，Codex 工作完成时由宠物给出低打扰提醒。

### 亮点

- **桌宠即入口**：透明置顶宠物窗口，支持点击、拖动、右键菜单、托盘控制和多状态动画。
- **快捷入口抽屉**：统一管理软件、文件夹、文件和网站，支持分类、标签、搜索、常用、管理员启动和高清 exe 图标提取。
- **AI 伴侣系统**：支持多角色档案、人设、模型覆盖、本机长期记忆、短期摘要、好感度和打字机式回复。
- **隐私优先**：AI Key、Base URL、聊天配置、快捷入口、本机路径、音乐库、宠物素材和平台凭据只保存到用户本机。
- **独立翻译**：支持独立翻译窗口和划选翻译快捷键，选中文本和译文不写入聊天、记忆或历史记录。
- **音乐播放器**：支持本机音乐导入、metadata、封面、歌词、标签、收藏、队列、迷你播放器和贴边自动隐藏。
- **在线音乐接入**：支持网易云音乐和酷狗音乐的登录、歌单、搜索、歌词、临时在线播放、音质偏好和不可播放自动跳过。
- **沉浸音乐舞台**：Canvas / WebGL 可视化、星河漫游、地面 DJ、WebGL 歌词、自由镜头、只看舞台、本机壁纸和多套沉浸主题。
- **Codex 状态聚合**：聚合多个 Codex 任务状态，用宠物动作、角标和悬浮气泡显示等待、运行、完成或失败。
- **双主题界面**：内置清爽默认主题和 `animal-island` 动物岛主题，主要窗口、弹窗、控件和状态提示均已适配。

### 功能概览

| 模块 | 能力 |
| --- | --- |
| 桌面宠物 | 透明窗口、置顶、拖动、右键菜单、托盘、动作绑定、多状态动画 |
| 快捷入口 | 软件、文件夹、文件、网站、分类、标签、搜索、常用、管理员启动 |
| 图标管理 | exe 自动提取、ico / lnk / exe 手动选择、默认文件夹/文件/网站图标 |
| AI 伴侣 | 多角色、人设、全局规则、模型覆盖、表情、打字机回复、本机记忆 |
| 好感度 | 可选启用、关系阶段、心情、信任度、亲密度、关系变化日志 |
| 翻译 | 独立窗口、源/目标语言、直译/润色、划选翻译、PDF 剪贴板增强 |
| 音乐 | 本机导入、metadata、封面、歌词、分类、标签、收藏、队列、迷你窗口 |
| 在线音乐 | 网易云、酷狗、二维码登录、歌单、搜索、歌词、临时播放、音质切换 |
| 沉浸模式 | WebGL 星河、地面 DJ、WebGL 歌词、自由镜头、舞台参数、壁纸 |
| Codex | App Server 状态接入、多任务聚合、完成角标、低打扰悬浮提醒 |

### 隐私与本机数据

PetDrawer 默认把个人数据当作本机运行时数据处理：

- AI API Key、Base URL、模型配置和聊天规则只保存到本机 `config.json`。
- 快捷入口、软件路径、文件夹路径、网站、打开历史和图标只保存到本机 `apps.json`、`icons/` 等目录。
- 聊天记录、长期记忆、短期摘要和好感度记录保存在本机 `pet-memory.db`。
- 本机音乐路径、分类、播放列表、导入资源、壁纸、宠物素材和平台凭据不写入源码或文档示例。
- 在线音乐播放 URL、Cookie、token、设备标识和搜索结果只按功能需要在本机运行态或本机配置中处理。
- 仓库不会提交 `.env*`、真实配置、密钥、token、证书、本机应用数据、打包缓存或用户导入资源。

发布和修改前的隐私检查规则见 [docs/RELEASE_PRIVACY_CHECKLIST.md](docs/RELEASE_PRIVACY_CHECKLIST.md)。

### 下载与安装

普通用户建议直接下载 Release 中的安装包或 exe，不需要安装 Node.js、Rust 或开发工具链。

1. 打开项目的 [GitHub Releases](https://github.com/JYHjyh001/desktop_pet/releases)。
2. 下载最新版本的安装包或免安装 exe。
3. 双击运行 PetDrawer。
4. 如果 Windows 弹出安全提示，请确认文件来源后选择继续运行。
5. 启动后桌面会出现宠物窗口，点击宠物打开或隐藏快捷入口抽屉。

常见运行依赖：

- Windows 10 / Windows 11。
- Microsoft Edge WebView2 Runtime。多数 Windows 10/11 环境已内置。

### 快速使用

1. 点击桌面宠物，打开快捷入口抽屉。
2. 在抽屉中添加软件、文件夹、文件或网站。
3. 在设置中配置主题、AI 接口、伴侣、记忆、窗口置顶和音乐偏好。
4. 右键宠物可打开菜单，进入聊天、翻译、故事、音乐播放器等窗口。
5. 在音乐窗口导入本机歌曲，或登录支持的平台读取歌单和歌词。
6. 开启沉浸模式后，可使用 WebGL 舞台、歌词、壁纸和舞台参数。

### 开发运行

开发环境建议使用 Windows。

准备：

- Node.js 和 npm。
- Rust 工具链。
- Microsoft C++ Build Tools。
- Microsoft Edge WebView2 Runtime。

安装依赖：

```bash
npm install
```

启动 Tauri 开发环境：

```bash
npm run tauri:dev
```

前端构建：

```bash
npm run build
```

打包桌面应用：

```bash
npm run tauri:build
```

Windows 下完整重启桌宠 UI 时，请优先使用：

```bash
npm run tauri:build
src-tauri\target\release\pet_drawer.exe
```

不要把单独 `cargo build --release` 当作完整 UI 打包方式；它可能没有携带最新前端资源。

### 技术栈

- **Tauri v2**：桌面窗口、托盘、系统能力、Rust 后端命令。
- **Vue 3**：前端窗口和交互界面。
- **TypeScript**：前端类型和业务逻辑。
- **Rust**：本机数据、窗口控制、启动器、音乐平台接入、AI 请求适配。
- **SQLite + FTS5**：本机长期记忆和聊天记录检索。
- **Three.js / WebGL**：沉浸音乐舞台和 3D 歌词。

### 文档索引

- 已完成能力：[docs/completed-features.md](docs/completed-features.md)
- 未完成能力：[docs/unfinished-features.md](docs/unfinished-features.md)
- 后续路线：[docs/next-development-plan.md](docs/next-development-plan.md)
- 音乐需求：[docs/music-feature-requirements.md](docs/music-feature-requirements.md)
- AI 宠物素材计划：[docs/ai-pet-material-generation-requirements.md](docs/ai-pet-material-generation-requirements.md)
- Windows 图标提取说明：[docs/windows-executable-icon-extraction-method.md](docs/windows-executable-icon-extraction-method.md)
- 版本记录：[docs/releases](docs/releases)

### 项目状态

当前项目仍在持续迭代。近期重点：

- `hatch-pet` / Codex pet atlas 到 PetDrawer 的本机离线转换器。
- Codex 状态聚合在真实连接模式下的回归验证。
- 音乐播放状态与宠物动画联动。
- 拆分超大的音乐窗口和后端音乐平台模块。

---

## English

PetDrawer is a Windows desktop companion app built with **Tauri v2, Vue 3, TypeScript, and Rust**. It combines a desktop pet, an app launcher drawer, AI companions, local memory, selection translation, a music player, online music integrations, immersive WebGL stages, and Codex task status notifications.

The goal is to make the desktop pet a useful everyday entry point: click it to open tools, right-click for actions, chat with local memory, listen to music in an immersive stage, and get low-interruption Codex completion cues from the pet.

### Highlights

- **Pet as the entry point**: transparent always-on-top pet window with dragging, right-click menu, tray control, action bindings, and multi-state animations.
- **Launcher drawer**: manage apps, folders, files, and websites with categories, tags, search, favorites, admin launch, and high-resolution exe icon extraction.
- **AI companion system**: multiple personas, prompts, model overrides, local long-term memory, short-term summaries, favorability, and typewriter replies.
- **Privacy-first design**: API keys, base URLs, chat settings, launcher paths, local music, imported assets, and platform credentials are local user data.
- **Translation tools**: standalone translator window plus global selection translation shortcut; selected text and translations are not saved into chat, memory, or history.
- **Music player**: local import, metadata, covers, lyrics, tags, favorites, queue, mini player, and edge auto-hide.
- **Online music**: NetEase Cloud Music and Kugou Music login, playlists, search, lyrics, temporary playback, quality preferences, and unavailable-track auto-skip.
- **Immersive music stage**: Canvas/WebGL visualizers, galaxy stage, ground-DJ terrain, WebGL lyrics, free camera, stage-only mode, local wallpapers, and themes.
- **Codex status aggregation**: aggregate multiple Codex task states and surface waiting/running/completed/failed states through pet actions, badges, and hover bubbles.
- **Two built-in themes**: clean default theme and `animal-island`, both covering major windows, dialogs, controls, empty states, and status UI.

### Feature Overview

| Area | Features |
| --- | --- |
| Desktop pet | Transparent window, always-on-top mode, dragging, right-click menu, tray, action bindings, multi-state animations |
| Launcher | Apps, folders, files, websites, categories, tags, search, favorites, admin launch |
| Icons | exe extraction, ico / lnk / exe manual source, default icons for files/folders/websites |
| AI companion | Personas, prompt rules, model override, emoji, typewriter reply, local memory |
| Favorability | Optional relationship system with mood, trust, intimacy, and change logs |
| Translation | Translator window, language selection, plain/polished mode, selection translation, PDF clipboard enhancement |
| Music | Local import, metadata, covers, lyrics, categories, tags, favorites, queue, mini player |
| Online music | NetEase, Kugou, QR login, playlists, search, lyrics, temporary playback, quality switching |
| Immersive mode | WebGL galaxy, ground-DJ terrain, WebGL lyrics, free camera, stage tuning, wallpapers |
| Codex | App Server status integration, multi-task aggregation, completion badge, hover notifications |

### Privacy

PetDrawer treats personal data as local runtime data:

- AI API keys, base URLs, models, and chat rules are stored only in the local `config.json`.
- Launcher entries, local paths, website entries, usage history, and icons are stored only in local app data.
- Chat history, long-term memory, short-term summaries, and favorability logs are stored in local `pet-memory.db`.
- Local music paths, imported assets, wallpapers, pet skins, playlists, and platform credentials are not written to source code or README examples.
- Playback URLs, cookies, tokens, device identifiers, and search results are handled only as local runtime/config data when required.
- The repository must not include `.env*`, real user config, secrets, tokens, certificates, app data, build cache, or imported user assets.

See [docs/RELEASE_PRIVACY_CHECKLIST.md](docs/RELEASE_PRIVACY_CHECKLIST.md) for the release privacy checklist.

### Download

End users should download packaged builds from [GitHub Releases](https://github.com/JYHjyh001/desktop_pet/releases). Node.js, Rust, and build tools are only required for development.

1. Download the latest installer or portable exe.
2. Run PetDrawer.
3. If Windows shows a security prompt, continue only after verifying the source.
4. Click the desktop pet to open or hide the launcher drawer.

Runtime requirement:

- Windows 10 / Windows 11.
- Microsoft Edge WebView2 Runtime, usually preinstalled on modern Windows.

### Development

Install dependencies:

```bash
npm install
```

Run the Tauri dev app:

```bash
npm run tauri:dev
```

Build the frontend:

```bash
npm run build
```

Package the desktop app:

```bash
npm run tauri:build
```

For a full Windows UI rebuild and restart:

```bash
npm run tauri:build
src-tauri\target\release\pet_drawer.exe
```

Do not use a standalone `cargo build --release` as the full UI packaging flow; it may not include the latest frontend assets.

### Stack

- **Tauri v2** for desktop windows, tray, native capabilities, and Rust commands.
- **Vue 3** for the frontend windows and interactions.
- **TypeScript** for typed frontend logic.
- **Rust** for local data, window control, launching, music integrations, and AI request adapters.
- **SQLite + FTS5** for local memory and chat retrieval.
- **Three.js / WebGL** for immersive music stages and 3D lyrics.

### Documentation

- Completed features: [docs/completed-features.md](docs/completed-features.md)
- Unfinished features: [docs/unfinished-features.md](docs/unfinished-features.md)
- Development plan: [docs/next-development-plan.md](docs/next-development-plan.md)
- Music requirements: [docs/music-feature-requirements.md](docs/music-feature-requirements.md)
- AI pet asset plan: [docs/ai-pet-material-generation-requirements.md](docs/ai-pet-material-generation-requirements.md)
- Windows icon extraction: [docs/windows-executable-icon-extraction-method.md](docs/windows-executable-icon-extraction-method.md)
- Release notes: [docs/releases](docs/releases)

### Roadmap

Current focus:

- Local converter from `hatch-pet` / Codex pet atlas outputs to PetDrawer pet skins.
- Real-environment regression for Codex status aggregation.
- Music playback state to pet animation linkage.
- Splitting large music window and platform integration modules for maintainability.
